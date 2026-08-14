#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod config;
mod d2d_engine;
mod gsmtc;
mod lrc_parser;
mod lyrics_api;
mod providers;
mod render;
mod settings_window;
mod tray;
mod utils;
mod window;

use crate::app_state::{AppState, APP_STATE};
use crate::config::load_or_create_config;
use crate::gsmtc::spawn_media_monitor;
use crate::lyrics_api::LyricsClient;
use crate::utils::trim_working_set;
use crate::window::{create_main_window, run_event_loop};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    let config = load_or_create_config();
    let media_handle = spawn_media_monitor();
    let lyrics_client = LyricsClient::new();

    let app_state = Arc::new(Mutex::new(AppState {
        media: Default::default(),
        lyrics_lines: Vec::new(),
        plain_lines: Vec::new(),
        current_index: 0,
        plain_lyrics: None,
        provider_name: None,
        is_loading: false,
        offset_ms: 0,
        is_locked: false,
        float_index: 0.0,
        config,
        last_pos_ms: 0,
        last_pos_update: Instant::now(),
        layout_cache_dirty: false,
    }));

    unsafe {
        APP_STATE = Some(app_state.clone());
    }

    let _hwnd = create_main_window();

    if app_state
        .lock()
        .map(|s| s.config.auto_trim_memory)
        .unwrap_or(true)
    {
        trim_working_set();
    }

    let state_clone = app_state.clone();
    let media_handle_clone = media_handle.clone();

    tokio::spawn(async move {
        let mut current_title = String::new();
        let mut current_artist = String::new();
        let mut current_album = String::new();
        let mut last_trim_time = Instant::now();
        let mut last_playing = false;
        let mut ticker = tokio::time::interval(Duration::from_millis(50));

        loop {
            ticker.tick().await;

            let media = if let Ok(m) = media_handle_clone.lock() {
                m.clone()
            } else {
                continue;
            };

            let (auto_trim, trim_interval) = if let Ok(s) = state_clone.lock() {
                (s.config.auto_trim_memory, s.config.trim_interval_secs)
            } else {
                (true, 60)
            };

            if trim_interval > 0 && last_trim_time.elapsed() >= Duration::from_secs(trim_interval) {
                if auto_trim {
                    trim_working_set();
                }
                last_trim_time = Instant::now();
            }

            if auto_trim && last_playing && !media.is_playing {
                trim_working_set();
            }
            last_playing = media.is_playing;

            let mut request_fetch = false;
            let mut fetch_title = String::new();
            let mut fetch_artist = String::new();
            let mut fetch_album = String::new();
            let mut fetch_dur = 0u64;

            if let Ok(mut s) = state_clone.lock() {
                if media.title != current_title
                    || media.artist != current_artist
                    || media.album != current_album
                {
                    current_title = media.title.clone();
                    current_artist = media.artist.clone();
                    current_album = media.album.clone();

                    s.layout_cache_dirty = true;

                    s.media = media.clone();
                    s.is_loading = !media.title.is_empty();
                    s.lyrics_lines.clear();
                    s.plain_lines.clear();
                    s.current_index = 0;
                    s.plain_lyrics = None;
                    s.provider_name = None;
                    s.last_pos_ms = media.position_ms;
                    s.last_pos_update = Instant::now();

                    request_fetch = true;
                    fetch_title = media.title.clone();
                    fetch_artist = media.artist.clone();
                    fetch_album = media.album.clone();
                    fetch_dur = media.duration_ms;
                } else {
                    let active_playing = media.is_playing || !media.title.is_empty();
                    if media.position_ms > 0 {
                        s.media.position_ms = media.position_ms;
                    }
                    s.media.is_playing = active_playing;

                    let real_pos = s.media.position_ms;
                    let adjusted_ms = (real_pos as i64 + s.offset_ms).max(0) as u64;

                    let new_index = if s.lyrics_lines.is_empty() {
                        0
                    } else {
                        let pos = Duration::from_millis(adjusted_ms);
                        lrc_parser::find_current_line(&s.lyrics_lines, pos).unwrap_or(0)
                    };

                    if s.current_index != new_index {
                        s.current_index = new_index;
                    }
                }
            }

            if request_fetch {
                let dur = (fetch_dur > 0).then_some(fetch_dur / 1000);
                let result = lyrics_client
                    .fetch_lyrics(&fetch_title, &fetch_artist, &fetch_album, dur)
                    .await;

                match result {
                    Ok((ttml_raw, plain_opt, provider)) => {
                        let karaoke_mode = if let Ok(s) = state_clone.lock() {
                            s.config.karaoke_mode.trim().to_lowercase()
                        } else {
                            "auto".to_string()
                        };

                        let mut parsed_lines = lrc_parser::parse_lrc(&ttml_raw);

                        // When karaoke_mode is "always", force word-by-word on
                        // all lines and synthesize per-syllable durations for
                        // lines that didn't come with real timing data.
                        if karaoke_mode == "always" {
                            for line in &mut parsed_lines {
                                if !line.is_karaoke {
                                    line.is_karaoke = true;

                                    let line_dur_ms = if let Some(end) = line.end_time {
                                        end.saturating_sub(line.time).as_millis() as u64
                                    } else {
                                        4000
                                    };
                                    let total_chars: usize =
                                        line.syllables.iter().map(|s| s.text.chars().count()).sum();
                                    if total_chars > 0 {
                                        let eff = line_dur_ms.clamp(500, 15000);
                                        for syl in line.syllables.iter_mut() {
                                            let cc = syl.text.chars().count() as u64;
                                            syl.duration = Duration::from_millis(
                                                ((eff * cc) / (total_chars as u64)).max(50),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Translate every line that needs it concurrently instead
                        // of awaiting each translation request one at a time —
                        // with N untranslated lines this turns an N * ~request_time
                        // serial wait into roughly one request's worth of wait.
                        let translation_futs =
                            parsed_lines.iter().enumerate().filter_map(|(idx, line)| {
                                let needs_translation = line.sub_text.is_none()
                                    || line.sub_text.as_ref().is_none_or(|st| st.trim().is_empty());
                                needs_translation.then(|| {
                                    let text = line.text.clone();
                                    let client = lyrics_client.clone();
                                    async move { (idx, client.translate_text(&text).await) }
                                })
                            });
                        let translations = futures::future::join_all(translation_futs).await;
                        for (idx, trans_opt) in translations {
                            if let Some(trans) = trans_opt {
                                parsed_lines[idx].sub_text = Some(trans);
                            }
                        }

                        if let Ok(mut s) = state_clone.lock() {
                            s.is_loading = false;
                            s.lyrics_lines = parsed_lines;
                            s.plain_lyrics = plain_opt;
                            s.provider_name = Some(provider);
                        }
                    }
                    Err(e) => {
                        if let Ok(mut s) = state_clone.lock() {
                            s.is_loading = false;
                            s.provider_name = None;
                        }
                        println!("[Main] Lyrics error: {}", e);
                    }
                }

                if auto_trim {
                    trim_working_set();
                }
            }
        }
    });

    run_event_loop();
}
