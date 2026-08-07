use crate::providers::http_debug::http_get_with_debug;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct TtmllibSearchResult {
    #[serde(default, rename = "lyricsTtml")]
    lyrics_ttml: Option<String>,
    #[serde(default, rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(default, rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(default)]
    ttml: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LyricsResult {
    pub synced: Option<String>,
    pub plain: Option<String>,
}

pub async fn fetch_ttmllib_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let dur_val = duration.unwrap_or(0);
    let album_val = if album.trim().is_empty() {
        "Unknown"
    } else {
        album
    };

    // 1. Try /api/get (exact match signature)
    let get_url = format!(
        "https://ttmllib.xyz/api/get?track_name={}&artist_name={}&album_name={}&duration={}",
        urlencoding::encode(clean_title),
        urlencoding::encode(artist),
        urlencoding::encode(album_val),
        dur_val
    );

    if let Ok(text) = http_get_with_debug(client, &get_url, "TTMLLIB /api/get").await {
        if text.trim().starts_with('{') {
            if let Ok(res) = serde_json::from_str::<TtmllibSearchResult>(&text) {
                let synced = res
                    .lyrics_ttml
                    .or(res.ttml)
                    .or(res.synced_lyrics)
                    .filter(|s| !s.trim().is_empty());
                let plain = res.plain_lyrics.filter(|s| !s.trim().is_empty());

                if synced.is_some() || plain.is_some() {
                    return Ok(LyricsResult { synced, plain });
                }
            }
        }
    }

    // 2. Fallback to /api/search (keyword search)
    let search_url = format!(
        "https://ttmllib.xyz/api/search?track_name={}&artist_name={}",
        urlencoding::encode(clean_title),
        urlencoding::encode(artist)
    );

    if let Ok(text) = http_get_with_debug(client, &search_url, "TTMLLIB /api/search").await {
        if text.trim().starts_with('[') {
            if let Ok(items) = serde_json::from_str::<Vec<TtmllibSearchResult>>(&text) {
                if let Some(res) = items.into_iter().next() {
                    let synced = res
                        .lyrics_ttml
                        .or(res.ttml)
                        .or(res.synced_lyrics)
                        .filter(|s| !s.trim().is_empty());
                    let plain = res.plain_lyrics.filter(|s| !s.trim().is_empty());

                    if synced.is_some() || plain.is_some() {
                        return Ok(LyricsResult { synced, plain });
                    }
                }
            }
        }
    }

    Err("No lyrics found in TTMLLIB response".into())
}
