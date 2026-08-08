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

/// Parses a single LRCMux response body. Handles both the "flat" shape
/// (synced/plain lyrics as plain strings — `LrcMuxResponse`) used by the
/// default endpoint, and the KPOE-compat endpoint's shape, where `lyrics`
/// is a JSON array of per-word/per-line `{ time, duration, text }` objects
/// instead of a string. The old code only tried the flat struct, so a
/// perfectly good KPOE-compat response would fail to deserialize (type
/// mismatch on `lyrics`) and get silently discarded.
fn parse_lrcmux_response(text: &str) -> Option<LyricsResult> {
    if !text.trim().starts_with('{') {
        return None;
    }

    // 1. Try the flat-string shape first.
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

    // 2. Fall back to the KPOE-compat array shape: `{ "lyrics": [ {time,
    //    duration, text}, ... ] }` (this is what /compat/kpoe/v2/... and
    //    similar endpoints actually return).
    let v: Value = serde_json::from_str(text).ok()?;
    let arr = v.get("lyrics").and_then(|l| l.as_array())?;
    let synced = convert_kpoe_array_to_lrc(arr)?;
    Some(LyricsResult {
        synced: Some(synced),
        plain: None,
    })
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
