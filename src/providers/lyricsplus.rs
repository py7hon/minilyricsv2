use crate::providers::http_debug::http_get_with_debug;
use crate::providers::ttmllib::LyricsResult;
use reqwest::Client;
use serde_json::Value;

fn convert_kpoe_array_to_lrc(lines_arr: &[Value]) -> Option<String> {
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
            .and_then(|v| {
                if let Some(f) = v.as_f64() {
                    if f < 10000.0 {
                        Some((f * 1000.0) as u64)
                    } else {
                        Some(f as u64)
                    }
                } else if let Some(u) = v.as_u64() {
                    if u < 10000 {
                        Some(u * 1000)
                    } else {
                        Some(u)
                    }
                } else if let Some(s) = v.as_str() {
                    if let Ok(f) = s.parse::<f64>() {
                        if f < 10000.0 {
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
            });

        if let Some(ms_val) = time_ms {
            let total_secs = ms_val / 1000;
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            let centis = (ms_val % 1000) / 10;
            let time_tag = format!("[{:02}:{:02}.{:02}]", mins, secs, centis);

            if !text.is_empty() {
                lrc_lines.push(format!("{} {}", time_tag, text));
            } else {
                lrc_lines.push(time_tag);
            }
        }
    }

    if lrc_lines.is_empty() {
        None
    } else {
        Some(lrc_lines.join("\n"))
    }
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

    for url in urls {
        if let Ok(text) = http_get_with_debug(client, &url, "LyricsPlus").await {
            if text.trim().starts_with('{') {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    // 1. Check for raw TTML string
                    if let Some(ttml) = v
                        .get("ttml")
                        .and_then(|s| s.as_str())
                        .filter(|s| !s.trim().is_empty())
                    {
                        return Ok(LyricsResult {
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
                                        return Ok(LyricsResult {
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

                        return Ok(LyricsResult {
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
                                            return Ok(LyricsResult {
                                                synced: Some(lrc_content),
                                                plain: None,
                                            });
                                        }
                                    }
                                }
                            }
                            if !trimmed_s.is_empty() {
                                return Ok(LyricsResult {
                                    synced: Some(trimmed_s.to_string()),
                                    plain: None,
                                });
                            }
                        } else if let Some(arr) = arr_val.as_array() {
                            if let Some(lrc_content) = convert_kpoe_array_to_lrc(arr) {
                                return Ok(LyricsResult {
                                    synced: Some(lrc_content),
                                    plain: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Err("LyricsPlus returned non-ok status or no lyrics".into())
}
