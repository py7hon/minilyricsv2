// src/providers/ttmllib.rs
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

pub fn ms_to_ttml_time(ms: u64) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let millis = ms % 1000;
    format!("{:02}:{:02}.{:03}", mins, secs, millis)
}

/// Shared KPOE-style lyrics-array -> TTML XML converter.
pub fn convert_kpoe_array_to_ttml(lines_arr: &[Value]) -> Option<String> {
    let mut p_blocks = Vec::new();

    for item in lines_arr {
        let line_is_bg = item
            .get("isBackground")
            .or_else(|| item.get("is_background"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let text = item
            .get("text")
            .or_else(|| item.get("words"))
            .or_else(|| item.get("line"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        let p_start_ms = item
            .get("time")
            .or_else(|| item.get("startTime"))
            .or_else(|| item.get("start"))
            .or_else(|| item.get("t"))
            .and_then(parse_time_val);

        let p_end_ms = item
            .get("endTime")
            .or_else(|| item.get("end"))
            .and_then(parse_time_val);

        let p_start_str = p_start_ms.map(ms_to_ttml_time);
        let p_end_str = p_end_ms.map(ms_to_ttml_time);

        let mut spans = Vec::new();
        let mut has_bg_syl = false;
        let is_words_array = item.get("words").is_some();
        let syllabus_arr = item
            .get("syllabus")
            .or_else(|| item.get("words"))
            .and_then(|v| v.as_array());

        if let Some(syllabus_arr) = syllabus_arr {
            let total_syls = syllabus_arr.len();
            if !syllabus_arr.is_empty() {
                let mut char_cursor = 0usize;
                let line_chars: Vec<char> = text.chars().collect();

                for (idx, syl) in syllabus_arr.iter().enumerate() {
                    let syl_text = syl.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    if syl_text.is_empty() {
                        continue;
                    }
                    let syl_start_ms = syl
                        .get("time")
                        .or_else(|| syl.get("startTime"))
                        .or_else(|| syl.get("start"))
                        .and_then(parse_time_val)
                        .or(p_start_ms);
                    let syl_end_ms = syl
                        .get("endTime")
                        .or_else(|| syl.get("end"))
                        .and_then(parse_time_val);

                    let syl_is_bg = syl
                        .get("isBackground")
                        .or_else(|| syl.get("is_background"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if syl_is_bg {
                        has_bg_syl = true;
                    }

                    let bg_attr = if syl_is_bg || line_is_bg {
                        " isBackground=\"true\""
                    } else {
                        ""
                    };

                    let mut final_text = syl_text.to_string();
                    let is_cjk_syl = final_text.chars().any(crate::lrc_parser::is_cjk);

                    if is_words_array {
                        if idx + 1 < total_syls
                            && !is_cjk_syl
                            && !final_text.ends_with(' ')
                            && !final_text.ends_with('-')
                            && !final_text.ends_with('\t')
                        {
                            final_text.push(' ');
                        }
                    } else if !line_chars.is_empty() {
                        let syl_clean = syl_text.trim();
                        let mut match_len = 0;
                        for c in syl_clean.chars() {
                            if char_cursor + match_len < line_chars.len()
                                && line_chars[char_cursor + match_len] == c
                            {
                                match_len += 1;
                            }
                        }
                        if match_len > 0 {
                            char_cursor += match_len;
                            if char_cursor < line_chars.len() && line_chars[char_cursor] == ' ' {
                                if !final_text.ends_with(' ') {
                                    final_text.push(' ');
                                }
                                while char_cursor < line_chars.len()
                                    && line_chars[char_cursor] == ' '
                                {
                                    char_cursor += 1;
                                }
                            }
                        } else if idx + 1 < total_syls
                            && !is_cjk_syl
                            && !final_text.ends_with(' ')
                            && !final_text.ends_with('-')
                        {
                            final_text.push(' ');
                        }
                    } else if idx + 1 < total_syls
                        && !is_cjk_syl
                        && !final_text.ends_with(' ')
                        && !final_text.ends_with('-')
                    {
                        final_text.push(' ');
                    }

                    let escaped = final_text
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");

                    match (syl_start_ms, syl_end_ms) {
                        (Some(b), Some(e)) => spans.push(format!(
                            "        <span begin=\"{}\" end=\"{}\"{}>{}</span>",
                            ms_to_ttml_time(b),
                            ms_to_ttml_time(e),
                            bg_attr,
                            escaped
                        )),
                        (Some(b), None) => spans.push(format!(
                            "        <span begin=\"{}\"{}>{}</span>",
                            ms_to_ttml_time(b),
                            bg_attr,
                            escaped
                        )),
                        _ => spans.push(format!("        <span{}>{}</span>", bg_attr, escaped)),
                    }
                }
            }
        }

        let p_bg_attr = if line_is_bg || has_bg_syl {
            " isBackground=\"true\""
        } else {
            ""
        };

        let singer_val = item
            .get("agent")
            .or_else(|| item.get("ttm:agent"))
            .or_else(|| item.get("singer"))
            .or_else(|| item.get("singerIndex"))
            .or_else(|| item.get("singer_index"))
            .or_else(|| item.get("vocalist"))
            .or_else(|| item.get("type"))
            .or_else(|| item.get("element").and_then(|e| e.get("singer")));

        let agent_attr = if let Some(v) = singer_val {
            if let Some(s) = v.as_str() {
                let s_lower = s.to_lowercase();
                if s_lower == "v2"
                    || s_lower == "2"
                    || s_lower == "secondary"
                    || s_lower == "duet"
                    || s_lower == "singer2"
                {
                    " ttm:agent=\"v2\"".to_string()
                } else if s_lower == "v1"
                    || s_lower == "1"
                    || s_lower == "primary"
                    || s_lower == "singer1"
                {
                    " ttm:agent=\"v1\"".to_string()
                } else if s_lower == "v0"
                    || s_lower == "0"
                    || s_lower == "both"
                    || s_lower == "together"
                    || s_lower == "unison"
                {
                    " ttm:agent=\"v0\"".to_string()
                } else {
                    format!(" ttm:agent=\"{}\"", s)
                }
            } else if let Some(i) = v.as_u64() {
                if i == 2 {
                    " ttm:agent=\"v2\"".to_string()
                } else if i == 0 {
                    " ttm:agent=\"v0\"".to_string()
                } else {
                    " ttm:agent=\"v1\"".to_string()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if spans.is_empty() && !text.is_empty() {
            let escaped = text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            spans.push(format!("        <span{}>{}</span>", p_bg_attr, escaped));
        }

        if !spans.is_empty() {
            let p_b = p_start_str.as_deref().unwrap_or("00:00.000");
            let mut p_html = if let Some(ref p_e) = p_end_str {
                format!(
                    "      <p begin=\"{}\" end=\"{}\"{}{}>\n{}\n      </p>",
                    p_b,
                    p_e,
                    p_bg_attr,
                    agent_attr,
                    spans.join("\n")
                )
            } else {
                format!(
                    "      <p begin=\"{}\"{}{}>\n{}\n      </p>",
                    p_b,
                    p_bg_attr,
                    agent_attr,
                    spans.join("\n")
                )
            };

            let trans_val = item
                .get("transliteration")
                .or_else(|| item.get("romanization"))
                .or_else(|| item.get("romaji"))
                .or_else(|| item.get("romaja"))
                .or_else(|| item.get("pinyin"))
                .or_else(|| item.get("translation"));

            let trans_text = trans_val
                .and_then(|v| {
                    v.as_str().map(|s| s.to_string()).or_else(|| {
                        v.get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                v.get("roman")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string())
                            })
                    })
                })
                .unwrap_or_default();
            let trans_clean = trans_text.trim();

            if !trans_clean.is_empty() {
                let escaped_trans = trans_clean
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                let sub_p = if let Some(ref p_e) = p_end_str {
                    format!(
                        "      <p begin=\"{}\" end=\"{}\" ttm:role=\"transliteration\">{}</p>",
                        p_b, p_e, escaped_trans
                    )
                } else {
                    format!(
                        "      <p begin=\"{}\" ttm:role=\"transliteration\">{}</p>",
                        p_b, escaped_trans
                    )
                };
                p_html.push('\n');
                p_html.push_str(&sub_p);
            }

            p_blocks.push(p_html);
        }
    }

    if p_blocks.is_empty() {
        None
    } else {
        Some(format!(
            "<tt xmlns=\"http://www.w3.org/ns/ttml\">\n  <body>\n    <div>\n{}\n    </div>\n  </body>\n</tt>",
            p_blocks.join("\n")
        ))
    }
}

pub fn convert_kpoe_array_to_lrc(lines_arr: &[Value]) -> Option<String> {
    convert_kpoe_array_to_ttml(lines_arr)
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
