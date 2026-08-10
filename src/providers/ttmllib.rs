use crate::providers::http_debug::http_get_with_debug;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

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

/// Shared KPOE-style lyrics-array -> LRC text converter. Several providers
/// (LyricsPlus, LRCMux's /compat/kpoe endpoint, ...) return lyrics as a
/// JSON array of `{ text|words|line, time|startTime|start|t, ... }` objects
/// instead of a plain LRC/TTML string. Kept here (rather than duplicated
/// per-provider) so every provider that hits this shape parses it the
/// same way instead of silently dropping valid lyrics because a provider's
pub fn parse_time_val(v: &Value) -> Option<u64> {
    if let Some(f) = v.as_f64() {
        if f < 500.0 {
            Some((f * 1000.0) as u64)
        } else {
            Some(f as u64)
        }
    } else if let Some(u) = v.as_u64() {
        if u < 500 {
            Some(u * 1000)
        } else {
            Some(u)
        }
    } else if let Some(s) = v.as_str() {
        if let Ok(f) = s.parse::<f64>() {
            if f < 500.0 {
                Some((f * 1000.0) as u64)
            } else {
                Some(f as u64)
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Shared KPOE-style lyrics-array -> LRC text converter.
pub fn convert_kpoe_array_to_lrc(lines_arr: &[Value]) -> Option<String> {
    let mut lrc_lines = Vec::new();

    for item in lines_arr {
        let text = item
            .get("text")
            .or_else(|| item.get("words"))
            .or_else(|| item.get("line"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        let time_ms = item
            .get("time")
            .or_else(|| item.get("startTime"))
            .or_else(|| item.get("start"))
            .or_else(|| item.get("t"))
            .and_then(parse_time_val);

        if let Some(ms_val) = time_ms {
            let total_secs = ms_val / 1000;
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            let centis = (ms_val % 1000) / 10;
            let time_tag = format!("[{:02}:{:02}.{:02}]", mins, secs, centis);

            let mut line_str = String::new();
            if let Some(syllabus_arr) = item.get("syllabus").and_then(|v| v.as_array()) {
                if !syllabus_arr.is_empty() {
                    line_str.push_str(&time_tag);
                    for syl in syllabus_arr {
                        let syl_text = syl.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        let syl_time = syl
                            .get("time")
                            .or_else(|| syl.get("startTime"))
                            .or_else(|| syl.get("start"))
                            .and_then(parse_time_val)
                            .unwrap_or(ms_val);
                        let s_secs = syl_time / 1000;
                        let s_mins = s_secs / 60;
                        let s_sec_rem = s_secs % 60;
                        let s_centis = (syl_time % 1000) / 10;
                        line_str.push_str(&format!(
                            " <{:02}:{:02}.{:02}>{}",
                            s_mins, s_sec_rem, s_centis, syl_text
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

            if let Some(trans_obj) = item.get("transliteration") {
                let trans_text = trans_obj
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                if !trans_text.is_empty() {
                    lrc_lines.push(format!("{} {}", time_tag, trans_text));
                }
            }
        }
    }

    if lrc_lines.is_empty() {
        None
    } else {
        Some(lrc_lines.join("\n"))
    }
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

    // Try /api/get and /api/search concurrently instead of sequentially —
    // /api/get is preferred, /api/search is the fallback, but there's no
    // reason to wait for /api/get to finish before starting /api/search.
    let search_url = format!(
        "https://ttmllib.xyz/api/search?track_name={}&artist_name={}",
        urlencoding::encode(clean_title),
        urlencoding::encode(artist)
    );
    let get_fut = http_get_with_debug(client, &get_url, "TTMLLIB /api/get");
    let search_fut = http_get_with_debug(client, &search_url, "TTMLLIB /api/search");
    let (get_resp, search_resp) = tokio::join!(get_fut, search_fut);

    if let Ok(text) = get_resp {
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

    if let Ok(text) = search_resp {
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
