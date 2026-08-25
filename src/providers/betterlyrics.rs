// src/providers/betterlyrics.rs
use crate::providers::http_debug::http_get_with_debug;
use crate::providers::ttmllib::LyricsResult;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BetterLyricsResponse {
    #[serde(default)]
    musixmatch_word_by_word_lyrics: Option<Value>,
    #[serde(default)]
    musixmatch_synced_lyrics: Option<Value>,
    #[serde(default)]
    lrclib_synced_lyrics: Option<Value>,
    #[serde(default)]
    lrclib_plain_lyrics: Option<Value>,
    #[serde(default)]
    go_lyrics_api_ttml: Option<Value>,
    #[serde(default)]
    qq_lyrics_api_lyrics: Option<Value>,
    #[serde(default)]
    kugou_lyrics_api_lyrics: Option<Value>,
    #[serde(default)]
    binimum_ttml: Option<Value>,
    #[serde(default)]
    apple_music_ttml: Option<Value>,
    #[serde(default)]
    kugou_direct_lyrics: Option<Value>,
    #[serde(default)]
    better_lyrics_ttml: Option<Value>,
    #[serde(default)]
    unison_ttml: Option<Value>,
    #[serde(default)]
    amll_ttml: Option<Value>,
    #[serde(default)]
    ttmllib_ttml: Option<Value>,
    #[serde(default)]
    romaji_lyrics: Option<Value>,
    #[serde(default)]
    romaja_lyrics: Option<Value>,
    #[serde(default)]
    pinyin_lyrics: Option<Value>,
}

fn extract_str(v: Option<&Value>) -> Option<String> {
    let val = v?;

    if let Some(s) = val.as_str() {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.starts_with('{') {
            if let Ok(inner_val) = serde_json::from_str::<Value>(trimmed) {
                if let Some(extracted) = extract_str(Some(&inner_val)) {
                    return Some(extracted);
                }
            }
        }

        return Some(trimmed.to_string());
    }

    if let Some(obj) = val.as_object() {
        for key in [
            "ttml",
            "lyrics",
            "syncedLyrics",
            "subtitle",
            "text",
            "lrc",
            "pinyin",
            "romaji",
            "romaja",
        ] {
            if let Some(inner_v) = obj.get(key) {
                if let Some(extracted) = extract_str(Some(inner_v)) {
                    return Some(extracted);
                }
            }
        }
    }

    if let Some(arr) = val.as_array() {
        if let Some(conv) = crate::providers::ttmllib::convert_kpoe_array_to_lrc(arr) {
            return Some(conv);
        }
    }

    None
}

fn parse_time_str_ms(ts: &str) -> Option<u64> {
    let ts = ts.trim();
    if ts.is_empty() {
        return None;
    }
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() == 2 {
        let mins: u64 = parts[0].parse().ok()?;
        let secs: f64 = parts[1].parse().ok()?;
        Some(mins * 60_000 + (secs * 1000.0) as u64)
    } else if parts.len() == 3 {
        let hours: u64 = parts[0].parse().ok()?;
        let mins: u64 = parts[1].parse().ok()?;
        let secs: f64 = parts[2].parse().ok()?;
        Some((hours * 3600 + mins * 60) * 1000 + (secs * 1000.0) as u64)
    } else {
        let secs: f64 = ts.parse().ok()?;
        Some((secs * 1000.0) as u64)
    }
}

pub fn convert_musixmatch_word_by_word_to_ttml(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut p_blocks = Vec::new();

    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let line_start_ms = if line.starts_with('[') {
            if let Some(r_idx) = line.find(']') {
                parse_time_str_ms(&line[1..r_idx])
            } else {
                None
            }
        } else {
            None
        };

        let rest = if line.starts_with('[') {
            if let Some(r_idx) = line.find(']') {
                line[r_idx + 1..].trim()
            } else {
                line
            }
        } else {
            line
        };

        let mut search_pos = 0;
        let mut raw_pairs = Vec::new();

        while let Some(start_open) = rest[search_pos..].find('<') {
            let actual_start = search_pos + start_open;
            if let Some(start_close) = rest[actual_start..].find('>') {
                let actual_start_close = actual_start + start_close;
                let ts_start = &rest[actual_start + 1..actual_start_close];

                let after_tag = actual_start_close + 1;
                let end_open = rest[after_tag..].find('<');
                let word_text = if let Some(eo) = end_open {
                    &rest[after_tag..after_tag + eo]
                } else {
                    &rest[after_tag..]
                };

                if let Some(end_open_rel) = end_open {
                    let actual_end_open = after_tag + end_open_rel;
                    if let Some(end_close_rel) = rest[actual_end_open..].find('>') {
                        let actual_end_close = actual_end_open + end_close_rel;
                        let ts_end = &rest[actual_end_open + 1..actual_end_close];

                        raw_pairs.push((ts_start, ts_end, word_text));
                        search_pos = actual_end_close + 1;
                        continue;
                    }
                }

                raw_pairs.push((ts_start, "", word_text));
                search_pos = after_tag + word_text.len();
            } else {
                break;
            }
        }

        let mut valid_words: Vec<(Option<u64>, Option<u64>, String)> = Vec::new();
        for (ts_start, ts_end, word_text) in raw_pairs {
            let clean_w = word_text.trim();
            if clean_w.is_empty() {
                if let Some(last) = valid_words.last_mut() {
                    if !last.2.ends_with(' ') {
                        last.2.push(' ');
                    }
                }
                continue;
            }

            let start_ms = parse_time_str_ms(ts_start).map(|t| {
                if let Some(l_ms) = line_start_ms {
                    if t < l_ms {
                        l_ms + t
                    } else {
                        t
                    }
                } else {
                    t
                }
            });

            let end_ms = parse_time_str_ms(ts_end).map(|t| {
                if let Some(l_ms) = line_start_ms {
                    if t < l_ms {
                        l_ms + t
                    } else {
                        t
                    }
                } else {
                    t
                }
            });

            valid_words.push((start_ms, end_ms, clean_w.to_string()));
        }

        if valid_words.is_empty() {
            continue;
        }

        let p_begin_ms = valid_words[0].0.or(line_start_ms).unwrap_or(0);
        let p_end_ms = valid_words
            .last()
            .and_then(|w| w.1.or(w.0))
            .unwrap_or(p_begin_ms + 3000);

        let mut spans = Vec::new();
        let total_count = valid_words.len();
        for (idx, (start_ms, end_ms, word_text)) in valid_words.iter().enumerate() {
            let mut final_text = word_text.clone();
            if idx + 1 < total_count && !final_text.ends_with(' ') {
                final_text.push(' ');
            }
            let escaped_text = final_text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");

            let w_b = start_ms.unwrap_or(p_begin_ms);
            let w_e = end_ms.or_else(|| {
                if idx + 1 < total_count {
                    valid_words[idx + 1].0
                } else {
                    Some(p_end_ms)
                }
            });

            if let Some(e) = w_e {
                spans.push(format!(
                    "        <span begin=\"{}\" end=\"{}\">{}</span>",
                    crate::providers::ttmllib::ms_to_ttml_time(w_b),
                    crate::providers::ttmllib::ms_to_ttml_time(e),
                    escaped_text
                ));
            } else {
                spans.push(format!(
                    "        <span begin=\"{}\">{}</span>",
                    crate::providers::ttmllib::ms_to_ttml_time(w_b),
                    escaped_text
                ));
            }
        }

        let p_html = format!(
            "      <p begin=\"{}\" end=\"{}\">\n{}\n      </p>",
            crate::providers::ttmllib::ms_to_ttml_time(p_begin_ms),
            crate::providers::ttmllib::ms_to_ttml_time(p_end_ms),
            spans.join("\n")
        );
        p_blocks.push(p_html);
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

fn parse_betterlyrics_response(text: &str) -> Option<LyricsResult> {
    if !text.trim().starts_with('{') {
        return None;
    }

    let resp: BetterLyricsResponse = serde_json::from_str(text).ok()?;

    // 1. Try TTML sources (word-by-word karaoke)
    let ttml_candidate = extract_str(resp.better_lyrics_ttml.as_ref())
        .or_else(|| extract_str(resp.apple_music_ttml.as_ref()))
        .or_else(|| extract_str(resp.amll_ttml.as_ref()))
        .or_else(|| extract_str(resp.unison_ttml.as_ref()))
        .or_else(|| extract_str(resp.ttmllib_ttml.as_ref()))
        .or_else(|| extract_str(resp.binimum_ttml.as_ref()))
        .or_else(|| extract_str(resp.go_lyrics_api_ttml.as_ref()));

    // 2. Try Musixmatch word-by-word LRC converted to TTML XML
    let mx_word = extract_str(resp.musixmatch_word_by_word_lyrics.as_ref())
        .and_then(|raw| convert_musixmatch_word_by_word_to_ttml(&raw));

    // 3. Try synced LRC sources
    let synced_lrc = extract_str(resp.musixmatch_synced_lyrics.as_ref())
        .or_else(|| extract_str(resp.lrclib_synced_lyrics.as_ref()))
        .or_else(|| extract_str(resp.qq_lyrics_api_lyrics.as_ref()))
        .or_else(|| extract_str(resp.kugou_lyrics_api_lyrics.as_ref()))
        .or_else(|| extract_str(resp.kugou_direct_lyrics.as_ref()));

    let mut main_synced = ttml_candidate.or(mx_word).or(synced_lrc);

    // 4. Attach pre-provided Romaji / Romaja / Pinyin sub-text if present
    let subtext_candidate = extract_str(resp.romaji_lyrics.as_ref())
        .or_else(|| extract_str(resp.romaja_lyrics.as_ref()))
        .or_else(|| extract_str(resp.pinyin_lyrics.as_ref()))
        .filter(|s| !s.trim().is_empty());

    if let (Some(ref mut synced), Some(sub)) = (&mut main_synced, subtext_candidate) {
        let is_ttml = synced.contains("<tt") || synced.contains("<p");
        if is_ttml {
            let mut sub_p_tags = Vec::new();
            if sub.contains('[') && sub.contains(']') {
                for line in sub.lines() {
                    let line = line.trim();
                    if line.starts_with('[') {
                        if let Some(r_idx) = line.find(']') {
                            let ts_str = &line[1..r_idx];
                            let txt = line[r_idx + 1..].trim();
                            if !txt.is_empty() {
                                if let Some(ms) = parse_time_str_ms(ts_str) {
                                    sub_p_tags.push(format!(
                                        "      <p begin=\"{}\" ttm:role=\"transliteration\">{}</p>",
                                        crate::providers::ttmllib::ms_to_ttml_time(ms),
                                        txt.replace('&', "&amp;")
                                            .replace('<', "&lt;")
                                            .replace('>', "&gt;")
                                    ));
                                }
                            }
                        }
                    }
                }
            } else {
                let sub_lines: Vec<&str> = sub
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .collect();
                let mut begin_times = Vec::new();
                let mut search_pos = 0;
                while let Some(p_idx) = synced[search_pos..].find("<p") {
                    let abs_p = search_pos + p_idx;
                    if let Some(p_end) = synced[abs_p..].find('>') {
                        let p_tag = &synced[abs_p..abs_p + p_end];
                        let pattern = "begin=\"";
                        if let Some(idx) = p_tag.find(pattern) {
                            let start = idx + pattern.len();
                            if let Some(end) = p_tag[start..].find('"') {
                                begin_times.push(p_tag[start..start + end].to_string());
                            }
                        }
                        search_pos = abs_p + p_end + 1;
                    } else {
                        break;
                    }
                }
                for (i, sub_line) in sub_lines.iter().enumerate() {
                    if i < begin_times.len() {
                        sub_p_tags.push(format!(
                            "      <p begin=\"{}\" ttm:role=\"transliteration\">{}</p>",
                            begin_times[i],
                            sub_line
                                .replace('&', "&amp;")
                                .replace('<', "&lt;")
                                .replace('>', "&gt;")
                        ));
                    }
                }
            }

            if !sub_p_tags.is_empty() {
                if let Some(div_end) = synced.rfind("</div>") {
                    synced.insert_str(div_end, &format!("\n{}\n", sub_p_tags.join("\n")));
                } else if let Some(body_end) = synced.rfind("</body>") {
                    synced.insert_str(body_end, &format!("\n{}\n", sub_p_tags.join("\n")));
                }
            }
        } else if sub.contains('[') && sub.contains(']') {
            synced.push('\n');
            synced.push_str(&sub);
        } else {
            let main_lines: Vec<&str> = synced.lines().collect();
            let sub_lines: Vec<&str> = sub.lines().collect();
            let mut merged = Vec::new();
            for (i, m_line) in main_lines.iter().enumerate() {
                if i < sub_lines.len() && !sub_lines[i].trim().is_empty() {
                    merged.push(format!("{} |sub: {}", m_line, sub_lines[i].trim()));
                } else {
                    merged.push((*m_line).to_string());
                }
            }
            *synced = merged.join("\n");
        }
    }

    let plain = extract_str(resp.lrclib_plain_lyrics.as_ref());

    if main_synced.is_some() || plain.is_some() {
        Some(LyricsResult {
            synced: main_synced,
            plain,
        })
    } else {
        None
    }
}

pub async fn fetch_betterlyrics_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let enc_song = urlencoding::encode(clean_title);
    let enc_artist = urlencoding::encode(artist);

    // Fast query (song + artist) responds in ~300ms on lyrics.pyoi.eu.org
    let simple_url = format!(
        "https://lyrics.pyoi.eu.org/lyrics?song={}&artist={}",
        enc_song, enc_artist
    );

    let has_detailed = !album.trim().is_empty() || duration.is_some();

    if !has_detailed {
        if let Ok(text) = http_get_with_debug(client, &simple_url, "BetterLyrics").await {
            if let Some(res) = parse_betterlyrics_response(&text) {
                return Ok(res);
            }
        }
        return Err("BetterLyrics API returned no lyrics".into());
    }

    let mut detailed_url = format!(
        "https://lyrics.pyoi.eu.org/lyrics?song={}&artist={}",
        enc_song, enc_artist
    );
    if !album.trim().is_empty() {
        detailed_url.push_str(&format!("&album={}", urlencoding::encode(album)));
    }
    if let Some(dur) = duration {
        detailed_url.push_str(&format!("&duration={}", dur));
    }

    // Execute simple_url and detailed_url in parallel using tokio::join!
    let simple_fut = async {
        if let Ok(text) = http_get_with_debug(client, &simple_url, "BetterLyrics").await {
            parse_betterlyrics_response(&text)
        } else {
            None
        }
    };

    let detailed_fut = async {
        if let Ok(text) = http_get_with_debug(client, &detailed_url, "BetterLyrics-Detailed").await
        {
            parse_betterlyrics_response(&text)
        } else {
            None
        }
    };

    let (simple_res, detailed_res) = tokio::join!(simple_fut, detailed_fut);

    if let Some(res) = simple_res.or(detailed_res) {
        Ok(res)
    } else {
        Err("BetterLyrics API returned no lyrics".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_musixmatch_word_by_word() {
        let input = "[00:13.50] <00:13.50> Tahun <00:13.91>   <00:13.99> lalu, <00:14.67>   <00:14.77> berjuta <00:15.54>   <00:15.61> alasanku <00:16.03>";
        let res = convert_musixmatch_word_by_word_to_ttml(input).unwrap();
        assert!(res.starts_with("<tt xmlns="));
        assert!(res.contains("<span begin=\"00:13.500\" end=\"00:13.910\">Tahun </span>"));
        assert!(res.contains("<span begin=\"00:13.990\" end=\"00:14.670\">lalu, </span>"));
        assert!(res.contains("<span begin=\"00:14.770\" end=\"00:15.540\">berjuta </span>"));
        assert!(res.contains("<span begin=\"00:15.610\" end=\"00:16.030\">alasanku</span>"));
    }

    #[test]
    fn test_extract_str_nested_json_string() {
        let raw_json = serde_json::json!({
            "betterLyricsTtml": "{\"ttml\":\"<tt xmlns=\\\"http://www.w3.org/ns/ttml\\\"><p begin=\\\"00:01.00\\\">Hello</p></tt>\"}"
        });
        let extracted = extract_str(raw_json.get("betterLyricsTtml")).unwrap();
        assert_eq!(
            extracted,
            "<tt xmlns=\"http://www.w3.org/ns/ttml\"><p begin=\"00:01.00\">Hello</p></tt>"
        );
    }
}
