use crate::providers::http_debug::http_get_with_debug;
use crate::providers::ttmllib::convert_kpoe_array_to_lrc;
use reqwest::Client;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct LyricsResult {
    pub synced: Option<String>,
    pub plain: Option<String>,
}

fn extract_lyrics_str(val: &Value) -> Option<String> {
    if let Some(s) = val.as_str() {
        if !s.trim().is_empty() {
            return Some(s.to_string());
        }
    } else if let Some(arr) = val.as_array() {
        return convert_kpoe_array_to_lrc(arr);
    } else if let Some(obj) = val.as_object() {
        for key in ["ttml", "lyrics", "syncedLyrics", "subtitle", "text", "lrc"] {
            if let Some(v) = obj.get(key) {
                if let Some(s) = v.as_str() {
                    if !s.trim().is_empty() {
                        return Some(s.to_string());
                    }
                } else if let Some(arr) = v.as_array() {
                    if let Some(conv) = convert_kpoe_array_to_lrc(arr) {
                        return Some(conv);
                    }
                }
            }
        }
    }
    None
}

pub async fn fetch_binimum_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let dur_val = duration.unwrap_or(0);

    let url = format!(
        "https://lyrics-api.binimum.org/?track={}&artist={}&album={}&duration={}",
        urlencoding::encode(clean_title),
        urlencoding::encode(artist),
        urlencoding::encode(album),
        dur_val
    );

    let body = http_get_with_debug(client, &url, "Binimum").await?;

    if !body.trim().is_empty() {
        if body.contains("<tt") || body.contains("[0") {
            return Ok(LyricsResult {
                synced: Some(body),
                plain: None,
            });
        }
        if let Ok(val) = serde_json::from_str::<Value>(&body) {
            if let Some(extracted) = extract_lyrics_str(&val) {
                return Ok(LyricsResult {
                    synced: Some(extracted),
                    plain: None,
                });
            }
        }
    }

    Err("Binimum returned no lyrics".into())
}
