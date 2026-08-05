#![windows_subsystem = "windows"]

mod app_state;
mod config;
mod gsmtc;
mod lrc_parser;
mod lyrics_api;
mod providers;
mod render;
mod tray;
mod window;

use app_state::{AppState, APP_STATE};
use config::load_or_create_config;
use gsmtc::spawn_media_monitor;
use lyrics_api::LyricsClient;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use window::{create_main_window, run_event_loop};

#[tokio::main]
async fn main() {
    let config = load_or_create_config();
    let media_handle = spawn_media_monitor();
    let lyrics_client = LyricsClient::new();

    let initial_offset = config.offset_ms;

    let app_state = Arc::new(Mutex::new(AppState {
        media: gsmtc::MediaInfo::default(),
        lyrics_lines: Vec::new(),
        plain_lines: Vec::new(),
        current_index: 0,
        plain_lyrics: None,
        is_loading: false,
        offset_ms: initial_offset,
        is_locked: false,
        float_index: 0.0,
        config,
        last_pos_ms: 0,
        last_pos_update: Instant::now(),
    }));

    unsafe {
        APP_STATE = Some(app_state.clone());
    }

    let _hwnd = create_main_window();

    let state_clone = app_state.clone();
    let media_handle_clone = media_handle.clone();

    tokio::spawn(async move {
        let mut current_title = String::new();
        let mut current_artist = String::new();
        let mut current_album = String::new();
        let mut ticker = tokio::time::interval(Duration::from_millis(50));

        loop {
            ticker.tick().await;

            let media = if let Ok(m) = media_handle_clone.lock() {
                m.clone()
            } else {
                continue;
            };

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

                    s.media = media.clone();
                    s.is_loading = !media.title.is_empty();
                    s.lyrics_lines.clear();
                    s.plain_lines.clear();
                    s.current_index = 0;
                    s.plain_lyrics = None;
                    s.last_pos_ms = if media.position_ms > 0 {
                        media.position_ms
                    } else {
                        0
                    };
                    s.last_pos_update = Instant::now();

                    request_fetch = true;
                    fetch_title = media.title.clone();
                    fetch_artist = media.artist.clone();
                    fetch_album = media.album.clone();
                    fetch_dur = media.duration_ms;
                } else {
                    let active_playing = media.is_playing || !media.title.is_empty();

                    if media.position_ms > 0 && media.position_ms != s.media.position_ms {
                        s.media.position_ms = media.position_ms;
                        s.last_pos_ms = media.position_ms;
                        s.last_pos_update = Instant::now();
                    }
                    s.media.is_playing = active_playing;

                    let real_pos = if s.media.is_playing {
                        s.last_pos_ms + s.last_pos_update.elapsed().as_millis() as u64
                    } else {
                        s.last_pos_ms
                    };
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
                    Ok((ttml_raw, plain_opt)) => {
                        let mut parsed_lines = lrc_parser::parse_lrc(&ttml_raw);

                        for line in &mut parsed_lines {
                            if line.sub_text.is_none()
                                || line.sub_text.as_ref().is_none_or(|st| st.trim().is_empty())
                            {
                                if let Some(trans) = lyrics_client.translate_text(&line.text).await
                                {
                                    line.sub_text = Some(trans);
                                }
                            }
                        }

                        if let Ok(mut s) = state_clone.lock() {
                            s.is_loading = false;
                            s.lyrics_lines = parsed_lines;
                            s.plain_lyrics = plain_opt;
                        }
                    }
                    Err(e) => {
                        if let Ok(mut s) = state_clone.lock() {
                            s.is_loading = false;
                        }
                        println!("[Main] Lyrics error: {}", e);
                    }
                }
            }
        }
    });

    run_event_loop();
}
