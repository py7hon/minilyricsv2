use crate::providers::ttmllib::LyricsResult;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct NeteaseSearchResponse {
    result: Option<NeteaseSearchResult>,
}

#[derive(Deserialize)]
struct NeteaseSearchResult {
    songs: Option<Vec<NeteaseSong>>,
}

#[derive(Deserialize)]
struct NeteaseSong {
    id: u64,
}

#[derive(Deserialize)]
struct NeteaseLyricResponse {
    lrc: Option<NeteaseLrcDetail>,
}

#[derive(Deserialize)]
struct NeteaseLrcDetail {
    lyric: Option<String>,
}

#[allow(dead_code)]
pub fn parse_ttml_time_ms(s: &str) -> Option<u64> {
    let s = s.trim().trim_end_matches('s');
    let parts: Vec<&str> = s.split(':').collect();
    let total_secs: f64 = match parts.len() {
        1 => parts[0].parse().ok()?,
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let sec: f64 = parts[1].parse().ok()?;
            m * 60.0 + sec
        }
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let sec: f64 = parts[2].parse().ok()?;
            h * 3600.0 + m * 60.0 + sec
        }
        _ => return None,
    };
    Some((total_secs * 1000.0).round() as u64)
}

#[allow(dead_code)]
pub fn clean_xml_text(raw: &str) -> String {
    let mut clean_text = String::new();
    let mut in_tag = false;
    for ch in raw.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            clean_text.push(ch);
        }
    }
    clean_text
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

#[allow(dead_code)]
pub fn ttml_to_lrc(ttml_content: &str) -> Option<String> {
    let mut lrc_lines = Vec::new();
    let mut search_idx = 0;

    while let Some(p_start) = ttml_content[search_idx..].find("<p ") {
        let actual_p_start = search_idx + p_start;
        let p_close = match ttml_content[actual_p_start..].find('>') {
            Some(c) => actual_p_start + c,
            None => break,
        };

        let tag_content = &ttml_content[actual_p_start..p_close];

        let begin_ms = if let Some(b_idx) = tag_content.find("begin=\"") {
            let val_start = b_idx + "begin=\"".len();
            if let Some(b_end) = tag_content[val_start..].find('"') {
                parse_ttml_time_ms(&tag_content[val_start..val_start + b_end])
            } else {
                None
            }
        } else {
            None
        };

        let content_start = p_close + 1;
        let end_p = match ttml_content[content_start..].find("</p>") {
            Some(e) => content_start + e,
            None => {
                search_idx = content_start;
                continue;
            }
        };

        let inner_html = &ttml_content[content_start..end_p];
        search_idx = end_p + 4;

        let Some(ts) = begin_ms else { continue };

        let mins = ts / 60000;
        let secs = (ts % 60000) / 1000;
        let hundredths = (ts % 1000) / 10;
        let ts_str = format!("[{:02}:{:02}.{:02}]", mins, secs, hundredths);

        if inner_html.contains("<span") {
            let mut line_str = String::new();
            let mut translation_str = String::new();
            let mut span_search_idx = 0;

            while let Some(s_start) = inner_html[span_search_idx..].find("<span") {
                let actual_s_start = span_search_idx + s_start;

                let s_close = match inner_html[actual_s_start..].find('>') {
                    Some(c) => actual_s_start + c,
                    None => break,
                };

                let s_tag = &inner_html[actual_s_start..s_close];
                let span_content_start = s_close + 1;
                let end_span = match inner_html[span_content_start..].find("</span>") {
                    Some(e) => span_content_start + e,
                    None => break,
                };

                let raw_text = &inner_html[span_content_start..end_span];
                let clean_text = clean_xml_text(raw_text);

                span_search_idx = end_span + 7;

                let is_translation = s_tag.contains("role=\"x-translation\"")
                    || s_tag.contains("role=\"translation\"")
                    || s_tag.contains("role=\"x-roman\"");

                if is_translation {
                    if !clean_text.trim().is_empty() {
                        translation_str = clean_text.trim().to_string();
                    }
                    continue;
                }

                let s_begin = if let Some(b_idx) = s_tag.find("begin=\"") {
                    let val_start = b_idx + "begin=\"".len();
                    if let Some(b_end) = s_tag[val_start..].find('"') {
                        parse_ttml_time_ms(&s_tag[val_start..val_start + b_end])
                    } else {
                        None
                    }
                } else {
                    None
                };

                let s_end = if let Some(e_idx) = s_tag.find("end=\"") {
                    let val_start = e_idx + "end=\"".len();
                    if let Some(e_end) = s_tag[val_start..].find('"') {
                        parse_ttml_time_ms(&s_tag[val_start..val_start + e_end])
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let (Some(sb), Some(se)) = (s_begin, s_end) {
                    let duration = se.saturating_sub(sb);

                    let has_trailing_space = inner_html[end_span + 7..].starts_with(' ')
                        || inner_html[end_span + 7..].starts_with('\n');

                    let mut formatted_word = clean_text.clone();
                    if has_trailing_space && !formatted_word.ends_with(' ') {
                        formatted_word.push(' ');
                    }

                    line_str.push_str(&format!("{}<{}>", formatted_word, duration));
                } else if !clean_text.is_empty() {
                    line_str.push_str(&clean_text);
                }
            }

            let mut final_line = format!("{}{}", ts_str, line_str.trim());
            if !translation_str.is_empty() {
                final_line.push_str(&format!("|sub:{}", translation_str));
            }

            if !line_str.trim().is_empty() {
                lrc_lines.push(final_line);
            }
        } else {
            let clean_text = clean_xml_text(inner_html);
            if !clean_text.trim().is_empty() {
                lrc_lines.push(format!("{}{}", ts_str, clean_text.trim()));
            }
        }
    }

    if !lrc_lines.is_empty() {
        Some(lrc_lines.join("\n"))
    } else {
        None
    }
}

pub async fn fetch_netease_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let search_q = format!("{} {}", title, artist);
    let search_url = format!(
        "https://music.163.com/api/search/get/web?s={}&type=1&limit=5",
        urlencoding::encode(&search_q)
    );

    let search_resp = client
        .get(&search_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await?;

    if !search_resp.status().is_success() {
        return Err("NetEase search status non-ok".into());
    }

    let search_res: NeteaseSearchResponse = search_resp.json().await?;
    let songs = search_res
        .result
        .and_then(|r| r.songs)
        .ok_or("No NetEase songs found")?;

    if songs.is_empty() {
        return Err("NetEase empty song list".into());
    }

    let song_id = songs[0].id;
    let lyric_url = format!(
        "https://music.163.com/api/song/lyric?id={}&lv=1&kv=1&tv=-1",
        song_id
    );

    let lyric_resp = client
        .get(&lyric_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await?;

    if !lyric_resp.status().is_success() {
        return Err("NetEase lyric status non-ok".into());
    }

    let lyric_res: NeteaseLyricResponse = lyric_resp.json().await?;
    let lrc = lyric_res
        .lrc
        .and_then(|l| l.lyric)
        .ok_or("No lyric field in NetEase response")?;

    if lrc.trim().is_empty() {
        return Err("NetEase returned empty LRC string".into());
    }

    Ok(LyricsResult {
        synced: Some(lrc),
        plain: None,
    })
}
