use crate::providers::http_debug::http_get_with_debug;
use crate::providers::ttmllib::{convert_kpoe_array_to_lrc, LyricsResult};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct LrcMuxResponse {
    #[serde(default, rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(default, rename = "lyricsTtml")]
    lyrics_ttml: Option<String>,
    #[serde(default)]
    ttml: Option<String>,
    #[serde(default)]
    lyrics: Option<String>,
    #[serde(default, rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

use crate::providers::ttmllib::parse_time_val;

fn convert_lrcmux_lines_to_lrc(lines: &[Value]) -> Option<String> {
    let mut lrc_lines = Vec::new();

    for item in lines {
        let text = item
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        let start_ms = item
            .get("start")
            .or_else(|| item.get("time"))
            .and_then(parse_time_val);

        if let Some(ms_val) = start_ms {
            let total_secs = ms_val / 1000;
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            let centis = (ms_val % 1000) / 10;
            let time_tag = format!("[{:02}:{:02}.{:02}]", mins, secs, centis);

            let mut line_str = String::new();
            if let Some(words_arr) = item.get("words").and_then(|v| v.as_array()) {
                if !words_arr.is_empty() {
                    line_str.push_str(&time_tag);
                    for w in words_arr {
                        let w_text = w.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let w_start = w
                            .get("start")
                            .or_else(|| w.get("time"))
                            .and_then(parse_time_val)
                            .unwrap_or(ms_val);
                        let w_secs = w_start / 1000;
                        let w_mins = w_secs / 60;
                        let w_sec_rem = w_secs % 60;
                        let w_centis = (w_start % 1000) / 10;
                        line_str.push_str(&format!(
                            " <{:02}:{:02}.{:02}>{}",
                            w_mins, w_sec_rem, w_centis, w_text
                        ));
                    }
                }
            }

            if line_str.is_empty() {
                if !text.is_empty() {
                    line_str = format!("{} {}", time_tag, text);
                } else {
                    line_str = time_tag.clone();
                }
            }

            lrc_lines.push(line_str);
        }
    }

    if lrc_lines.is_empty() {
        None
    } else {
        Some(lrc_lines.join("\n"))
    }
}

fn parse_lrcmux_response(text: &str) -> Option<LyricsResult> {
    if !text.trim().starts_with('{') {
        return None;
    }

    let v: Value = serde_json::from_str(text).ok()?;

    // 1. Handle native LRCMux response with "lines" array (`{ "lines": [ { "start", "words", ... } ] }`)
    if let Some(lines_arr) = v.get("lines").and_then(|l| l.as_array()) {
        if let Some(synced) = convert_lrcmux_lines_to_lrc(lines_arr) {
            return Some(LyricsResult {
                synced: Some(synced),
                plain: None,
            });
        }
    }

    // 2. Handle flat string shape (syncedLyrics, lyricsTtml, ttml, lyrics)
    if let Ok(res) = serde_json::from_str::<LrcMuxResponse>(text) {
        let synced = res
            .lyrics_ttml
            .or(res.ttml)
            .or(res.synced_lyrics)
            .or(res.lyrics)
            .filter(|s| !s.trim().is_empty());
        let plain = res.plain_lyrics.filter(|s| !s.trim().is_empty());

        if synced.is_some() || plain.is_some() {
            return Some(LyricsResult { synced, plain });
        }
    }

    // 3. Handle KPOE-compat array shape in "lyrics"
    if let Some(arr) = v.get("lyrics").and_then(|l| l.as_array()) {
        if let Some(synced) = convert_kpoe_array_to_lrc(arr) {
            return Some(LyricsResult {
                synced: Some(synced),
                plain: None,
            });
        }
    }

    None
}

pub async fn fetch_lrcmux_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let enc_title = urlencoding::encode(clean_title);
    let enc_artist = urlencoding::encode(artist);
    let enc_album = urlencoding::encode(album);
    let dur_val = duration.unwrap_or(0);

    let urls = [
        format!("https://api.lrcmux.dev/get?title={}&artist={}&album={}&duration={}", enc_title, enc_artist, enc_album, dur_val),
        format!("https://api.lrcmux.dev/compat/kpoe/v2/lyrics/get?title={}&artist={}", enc_title, enc_artist),
        format!("https://api.lrcmux.dev/compat/lrclib/api/get?track_name={}&artist_name={}&album_name={}&duration={}", enc_title, enc_artist, enc_album, dur_val),
        format!("https://api.lrcmux.dev/get?title={}&artist={}", enc_title, enc_artist),
    ];

    // Race all fallback URLs concurrently: whichever request returns usable
    // lyrics first wins immediately, and the rest are cancelled instead of
    // being awaited to completion (or to their 4s timeout) for nothing.
    let mut set = tokio::task::JoinSet::new();
    for url in urls {
        let client = client.clone();
        set.spawn(async move { http_get_with_debug(&client, &url, "LRCMux").await.ok() });
    }

    while let Some(joined) = set.join_next().await {
        if let Ok(Some(text)) = joined {
            if let Some(result) = parse_lrcmux_response(&text) {
                return Ok(result);
            }
        }
    }

    Err("LRCMux returned non-ok status or no lyrics".into())
}
