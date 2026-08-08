use crate::providers::http_debug::http_get_with_debug;
use crate::providers::ttmllib::{convert_kpoe_array_to_lrc, LyricsResult};
use reqwest::Client;
use serde_json::Value;

fn parse_lyricsplus_response(text: &str) -> Option<LyricsResult> {
    if !text.trim().starts_with('{') {
        return None;
    }
    let v: Value = serde_json::from_str(text).ok()?;

    // 1. Check for raw TTML string
    if let Some(ttml) = v
        .get("ttml")
        .and_then(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
    {
        return Some(LyricsResult {
            synced: Some(ttml.to_string()),
            plain: None,
        });
    }

    // 2. Check for syncedLyrics / synced string
    let synced_str = v
        .get("syncedLyrics")
        .or_else(|| v.get("synced"))
        .and_then(|s| s.as_str())
        .filter(|s| !s.trim().is_empty());

    if let Some(synced) = synced_str {
        let trimmed_synced = synced.trim();
        if trimmed_synced.starts_with('[')
            && (trimmed_synced.contains("{\"") || trimmed_synced.contains("{ \""))
        {
            if let Ok(arr_val) = serde_json::from_str::<Value>(trimmed_synced) {
                if let Some(arr) = arr_val.as_array() {
                    if let Some(lrc_content) = convert_kpoe_array_to_lrc(arr) {
                        return Some(LyricsResult {
                            synced: Some(lrc_content),
                            plain: v
                                .get("plainLyrics")
                                .or_else(|| v.get("plain"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                        });
                    }
                }
            }
        }

        return Some(LyricsResult {
            synced: Some(synced.to_string()),
            plain: v
                .get("plainLyrics")
                .or_else(|| v.get("plain"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string()),
        });
    }

    // 3. Check if "lyrics" or "element" is a JSON array of KPOE lines
    let lyrics_val = v.get("lyrics").or_else(|| v.get("element"));
    if let Some(arr_val) = lyrics_val {
        if let Some(s) = arr_val.as_str() {
            let trimmed_s = s.trim();
            if trimmed_s.starts_with('[')
                && (trimmed_s.contains("{\"") || trimmed_s.contains("{ \""))
            {
                if let Ok(parsed_arr) = serde_json::from_str::<Value>(trimmed_s) {
                    if let Some(arr) = parsed_arr.as_array() {
                        if let Some(lrc_content) = convert_kpoe_array_to_lrc(arr) {
                            return Some(LyricsResult {
                                synced: Some(lrc_content),
                                plain: None,
                            });
                        }
                    }
                }
            }
            if !trimmed_s.is_empty() {
                return Some(LyricsResult {
                    synced: Some(trimmed_s.to_string()),
                    plain: None,
                });
            }
        } else if let Some(arr) = arr_val.as_array() {
            if let Some(lrc_content) = convert_kpoe_array_to_lrc(arr) {
                return Some(LyricsResult {
                    synced: Some(lrc_content),
                    plain: None,
                });
            }
        }
    }

    None
}

pub async fn fetch_lyricsplus_lyrics(
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
        format!(
            "https://lyricsplus.prjktla.my.id/v2/lyrics/get?title={}&artist={}&album={}&duration={}",
            enc_title, enc_artist, enc_album, dur_val
        ),
        format!(
            "https://lyricsplus.prjktla.my.id/v1/lyrics?title={}&artist={}&album={}&duration={}",
            enc_title, enc_artist, enc_album, dur_val
        ),
        format!(
            "https://lyricsplus-seven.vercel.app/v2/lyrics/get?title={}&artist={}&album={}&duration={}",
            enc_title, enc_artist, enc_album, dur_val
        ),
        format!("https://lyricsplus.prjktla.my.id/v2/lyrics/get?title={}&artist={}", enc_title, enc_artist),
    ];

    // Race all fallback URLs concurrently: whichever request returns usable
    // lyrics first wins immediately, and the rest are cancelled instead of
    // being awaited to completion (or to their 4s timeout) for nothing —
    // this is what was making a request wait the full 4s even after a
    // sibling URL had already returned good data seconds earlier.
    let mut set = tokio::task::JoinSet::new();
    for url in urls {
        let client = client.clone();
        set.spawn(async move { http_get_with_debug(&client, &url, "LyricsPlus").await.ok() });
    }

    while let Some(joined) = set.join_next().await {
        if let Ok(Some(text)) = joined {
            if let Some(result) = parse_lyricsplus_response(&text) {
                return Ok(result);
            }
        }
    }

    Err("LyricsPlus returned non-ok status or no lyrics".into())
}
