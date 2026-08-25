use crate::providers::http_debug::http_get_with_debug;
use crate::providers::ttmllib::{convert_kpoe_array_to_lrc, LyricsResult};
use reqwest::Client;
use serde_json::Value;

fn parse_lyricsplus_response(text: &str) -> Option<LyricsResult> {
    if !text.trim().starts_with('{') {
        return None;
    }
    let v: Value = serde_json::from_str(text).ok()?;

    let mut result = if let Some(ttml) = v
        .get("ttml")
        .and_then(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
    {
        Some(LyricsResult {
            synced: Some(ttml.to_string()),
            plain: None,
        })
    } else if let Some(arr_val) = v
        .get("lyrics")
        .or_else(|| v.get("element"))
        .or_else(|| v.get("lines"))
    {
        if let Some(s) = arr_val.as_str() {
            let trimmed_s = s.trim();
            if trimmed_s.starts_with('[')
                && (trimmed_s.contains("{\"") || trimmed_s.contains("{ \""))
            {
                if let Ok(parsed_arr) = serde_json::from_str::<Value>(trimmed_s) {
                    if let Some(arr) = parsed_arr.as_array() {
                        convert_kpoe_array_to_lrc(arr).map(|lrc_content| LyricsResult {
                            synced: Some(lrc_content),
                            plain: v
                                .get("plainLyrics")
                                .or_else(|| v.get("plain"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Some(arr) = arr_val.as_array() {
            convert_kpoe_array_to_lrc(arr).map(|lrc_content| LyricsResult {
                synced: Some(lrc_content),
                plain: v
                    .get("plainLyrics")
                    .or_else(|| v.get("plain"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
            })
        } else {
            None
        }
    } else if let Some(synced) = v
        .get("syncedLyrics")
        .or_else(|| v.get("synced"))
        .and_then(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
    {
        let trimmed_synced = synced.trim();
        if trimmed_synced.starts_with('[')
            && (trimmed_synced.contains("{\"") || trimmed_synced.contains("{ \""))
        {
            if let Ok(arr_val) = serde_json::from_str::<Value>(trimmed_synced) {
                if let Some(arr) = arr_val.as_array() {
                    convert_kpoe_array_to_lrc(arr).map(|lrc_content| LyricsResult {
                        synced: Some(lrc_content),
                        plain: v
                            .get("plainLyrics")
                            .or_else(|| v.get("plain"))
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string()),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            let final_synced = if trimmed_synced.contains('<')
                && trimmed_synced.contains('>')
                && trimmed_synced.contains('[')
            {
                crate::providers::betterlyrics::convert_musixmatch_word_by_word_to_ttml(
                    trimmed_synced,
                )
                .unwrap_or_else(|| trimmed_synced.to_string())
            } else {
                trimmed_synced.to_string()
            };

            Some(LyricsResult {
                synced: Some(final_synced),
                plain: v
                    .get("plainLyrics")
                    .or_else(|| v.get("plain"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
            })
        }
    } else {
        None
    };

    let top_sub = v
        .get("transliteration")
        .or_else(|| v.get("romanization"))
        .or_else(|| v.get("romaji"))
        .or_else(|| v.get("romaja"))
        .or_else(|| v.get("pinyin"))
        .or_else(|| v.get("translation"))
        .and_then(|sub_v| {
            sub_v.as_str().map(|s| s.to_string()).or_else(|| {
                sub_v
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
        })
        .filter(|s| !s.trim().is_empty());

    if let (Some(ref mut res), Some(sub)) = (&mut result, top_sub) {
        if let Some(ref mut synced) = res.synced {
            attach_subtext_to_synced(synced, &sub);
        }
    }

    result
}

fn attach_subtext_to_synced(synced: &mut String, sub: &str) {
    let sub = sub.trim();
    if sub.is_empty() {
        return;
    }
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
                            sub_p_tags.push(format!(
                                "      <p begin=\"{}\" ttm:role=\"transliteration\">{}</p>",
                                ts_str,
                                txt.replace('&', "&amp;")
                                    .replace('<', "&lt;")
                                    .replace('>', "&gt;")
                            ));
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
        synced.push_str(sub);
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
            "https://lyrics-api.boidu.dev/getLyrics?s={}&a={}&al={}&d={}",
            enc_title, enc_artist, enc_album, dur_val
        ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_lyricsplus_top_level_transliteration() {
        let raw = json!({
            "syncedLyrics": "[00:10.02] (Aqours! WAVE!)",
            "transliteration": "[00:10.02] (Aqours! WAVE!)"
        })
        .to_string();

        let res = parse_lyricsplus_response(&raw).unwrap();
        let synced = res.synced.unwrap();
        let lines = crate::lrc_parser::parse_lrc(&synced);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "(Aqours! WAVE!)");
        assert_eq!(lines[0].sub_text.as_deref(), Some("(Aqours! WAVE!)"));
    }

    #[test]
    fn test_parse_lyricsplus_kpoe_item_transliteration() {
        let raw = json!({
            "lyrics": [
                {
                    "time": 10000,
                    "text": "笑っていてほしくて",
                    "transliteration": {
                        "lang": "ja-Latn",
                        "text": "waratte ite hoshikute"
                    }
                }
            ]
        })
        .to_string();

        let res = parse_lyricsplus_response(&raw).unwrap();
        let synced = res.synced.unwrap();
        let lines = crate::lrc_parser::parse_lrc(&synced);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "笑っていてほしくて");
        assert_eq!(lines[0].sub_text.as_deref(), Some("waratte ite hoshikute"));
    }
}
