use std::borrow::Cow;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Syllable {
    pub text: String,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct LrcLine {
    pub time: Duration,
    pub end_time: Option<Duration>,
    pub text: String,
    pub syllables: Vec<Syllable>,
    pub sub_text: Option<String>,
    #[allow(dead_code)]
    pub style_name: String,
    pub is_karaoke: bool,
    pub singer_index: u8,
}

pub fn unescape_xml_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp_idx) = rest.find('&') {
        out.push_str(&rest[..amp_idx]);
        rest = &rest[amp_idx..];
        if let Some(semi_idx) = rest.find(';') {
            let entity = &rest[1..semi_idx];
            let replacement: Option<Cow<'static, str>> = match entity {
                "apos" | "rsquo" | "lsquo" => Some("'".into()),
                "quot" | "rdquo" | "ldqu" | "ldquo" => Some("\"".into()),
                "amp" => Some("&".into()),
                "lt" => Some("<".into()),
                "gt" => Some(">".into()),
                "nbsp" => Some(" ".into()),
                "ndash" => Some("–".into()),
                "mdash" => Some("—".into()),
                "hellip" => Some("…".into()),
                "copy" => Some("©".into()),
                "reg" => Some("®".into()),
                _ => {
                    if let Some(hex) = entity
                        .strip_prefix("#x")
                        .or_else(|| entity.strip_prefix("#X"))
                    {
                        u32::from_str_radix(hex, 16)
                            .ok()
                            .and_then(char::from_u32)
                            .map(|c| c.to_string().into())
                    } else if let Some(dec) = entity.strip_prefix('#') {
                        dec.parse::<u32>()
                            .ok()
                            .and_then(char::from_u32)
                            .map(|c| c.to_string().into())
                    } else {
                        None
                    }
                }
            };
            if let Some(rep) = replacement {
                out.push_str(&rep);
                rest = &rest[semi_idx + 1..];
            } else {
                out.push('&');
                rest = &rest[1..];
            }
        } else {
            break;
        }
    }
    out.push_str(rest);
    out
}

fn detect_singer_index(p_open_tag: &str, div_open_tag: &str, text: &str) -> u8 {
    let combined = format!("{} {}", p_open_tag, div_open_tag).to_lowercase();
    if combined.contains("v2")
        || combined.contains("v3")
        || combined.contains("secondary")
        || combined.contains("duet")
        || combined.contains("agent=\"v2\"")
        || combined.contains("agent='v2'")
    {
        return 1;
    }
    let trimmed = text.trim();
    if (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        return 1;
    }
    0
}

fn parse_ttml_time_str(s: &str) -> Option<Duration> {
    let s = s.trim().trim_matches('"').trim_matches('\'');
    if s.is_empty() {
        return None;
    }

    if let Some(secs_str) = s.strip_suffix('s') {
        let secs_f: f64 = secs_str.parse().ok()?;
        return Some(Duration::from_secs_f64(secs_f));
    }

    if let Some(ms_str) = s.strip_suffix("ms") {
        let ms: u64 = ms_str.parse().ok()?;
        return Some(Duration::from_millis(ms));
    }

    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let hours: u64 = parts[0].parse().ok()?;
        let mins: u64 = parts[1].parse().ok()?;
        let secs_f: f64 = parts[2].parse().ok()?;
        let total_ms = (hours * 3600 + mins * 60) * 1000 + (secs_f * 1000.0) as u64;
        Some(Duration::from_millis(total_ms))
    } else if parts.len() == 2 {
        let mins: u64 = parts[0].parse().ok()?;
        let secs_f: f64 = parts[1].parse().ok()?;
        let total_ms = mins * 60000 + (secs_f * 1000.0) as u64;
        Some(Duration::from_millis(total_ms))
    } else {
        let secs_f: f64 = s.parse().ok()?;
        Some(Duration::from_secs_f64(secs_f))
    }
}

fn extract_xml_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(idx) = tag.find(&pattern) {
        let start = idx + pattern.len();
        if let Some(end) = tag[start..].find('"') {
            return Some(tag[start..start + end].to_string());
        }
    }
    let pattern_single = format!("{}='", attr);
    if let Some(idx) = tag.find(&pattern_single) {
        let start = idx + pattern_single.len();
        if let Some(end) = tag[start..].find('\'') {
            return Some(tag[start..start + end].to_string());
        }
    }
    None
}

fn strip_xml_tags(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(ch);
        }
    }
    out
}

fn clean_inter_xml_text(raw: &str) -> Option<String> {
    let clean = unescape_xml_entities(&strip_xml_tags(raw));
    if !clean.chars().any(|c| !c.is_whitespace()) {
        return None;
    }
    let mut out = String::new();
    let mut last_was_space = false;
    for ch in clean.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn parse_ttml(content: &str) -> Vec<LrcLine> {
    let mut lines = Vec::new();
    let mut pos = 0;

    while let Some(p_start) = content[pos..].find("<p") {
        let abs_p_start = pos + p_start;
        let p_end = match content[abs_p_start..].find("</p>") {
            Some(e) => abs_p_start + e + 4,
            None => break,
        };

        let p_block = &content[abs_p_start..p_end];
        pos = p_end;

        let p_tag_end = p_block.find('>').unwrap_or(p_block.len());
        let p_open_tag = &p_block[..p_tag_end];

        if p_open_tag.contains("role=\"translation\"")
            || p_open_tag.contains("role='translation'")
            || p_open_tag.contains("role=\"transliteration\"")
            || p_open_tag.contains("role='transliteration'")
        {
            continue;
        }

        let mut div_open_tag = "";
        if let Some(last_div_start) = content[..abs_p_start].rfind("<div") {
            let last_div_end = content[last_div_start..abs_p_start].rfind("</div>");
            if last_div_end.is_none() {
                let div_tag_end = content[last_div_start..].find('>').unwrap_or(0);
                div_open_tag = &content[last_div_start..last_div_start + div_tag_end];
                if div_open_tag.contains("role=\"translation\"")
                    || div_open_tag.contains("role='translation'")
                    || div_open_tag.contains("role=\"transliteration\"")
                    || div_open_tag.contains("role='transliteration'")
                {
                    continue;
                }
            }
        }

        let begin_val = extract_xml_attr(p_block, "begin");
        let end_val = extract_xml_attr(p_block, "end");

        let p_begin = match begin_val.as_deref().and_then(parse_ttml_time_str) {
            Some(t) => t,
            None => continue,
        };
        let p_end_time = end_val.as_deref().and_then(parse_ttml_time_str);

        let mut syllables = Vec::new();
        let mut full_text = String::new();
        let mut is_karaoke = false;

        let p_tag_open_end = p_block.find('>').map(|i| i + 1).unwrap_or(0);
        let mut prev_span_end = p_tag_open_end;

        let mut span_pos = 0;
        while let Some(s_start) = p_block[span_pos..].find("<span") {
            let abs_s_start = span_pos + s_start;
            let s_close_tag = match p_block[abs_s_start..].find('>') {
                Some(c) => abs_s_start + c + 1,
                None => break,
            };
            let s_end = match p_block[s_close_tag..].find("</span>") {
                Some(e) => s_close_tag + e,
                None => break,
            };

            let tag_attrs = &p_block[abs_s_start..s_close_tag];
            let span_text_raw = &p_block[s_close_tag..s_end];
            span_pos = s_end + 7;

            let role_val = extract_xml_attr(tag_attrs, "ttm:role")
                .or_else(|| extract_xml_attr(tag_attrs, "role"));

            let is_translation_span = role_val.as_deref().is_some_and(|role| {
                let r = role.to_lowercase();
                r.contains("translation") || r.contains("transliteration") || r.contains("roman")
            });

            // Capture untagged text appearing before this <span> tag (e.g. &apos;t or punctuation)
            if abs_s_start > prev_span_end {
                let inter_raw = &p_block[prev_span_end..abs_s_start];
                if let Some(inter_clean) = clean_inter_xml_text(inter_raw) {
                    let last_syl_ends_with_space = syllables
                        .last()
                        .is_some_and(|s: &Syllable| s.text.ends_with(' '));
                    let to_add = if last_syl_ends_with_space && inter_clean.starts_with(' ') {
                        inter_clean.trim_start()
                    } else {
                        &inter_clean
                    };
                    if !to_add.is_empty() {
                        if let Some(last_syl) = syllables.last_mut() {
                            last_syl.text.push_str(to_add);
                        }
                        full_text.push_str(to_add);
                    }
                }
            }

            prev_span_end = span_pos;

            if is_translation_span {
                continue;
            }

            let span_text = strip_xml_tags(span_text_raw);
            let span_text = unescape_xml_entities(&span_text);
            if span_text.is_empty() {
                continue;
            }

            let after_span = &p_block[s_end + 7..];
            let has_outside_space = after_span.starts_with(' ')
                && !after_span.starts_with('\r')
                && !after_span.starts_with('\n');

            let mut final_span_text = span_text;
            if has_outside_space && !final_span_text.ends_with(' ') {
                final_span_text.push(' ');
            }

            let s_begin = extract_xml_attr(tag_attrs, "begin")
                .as_deref()
                .and_then(parse_ttml_time_str);
            let s_end_t = extract_xml_attr(tag_attrs, "end")
                .as_deref()
                .and_then(parse_ttml_time_str);

            if s_begin.is_some() || s_end_t.is_some() {
                is_karaoke = true;
            }

            let dur = if let (Some(b), Some(e)) = (s_begin, s_end_t) {
                e.saturating_sub(b)
            } else {
                Duration::from_millis(300)
            };

            syllables.push(Syllable {
                text: final_span_text.clone(),
                duration: dur,
            });
            full_text.push_str(&final_span_text);
        }

        // Capture untagged text after the last <span> tag (before </p>)
        let p_content_end = p_block.rfind("</p>").unwrap_or(p_block.len());
        if p_content_end > prev_span_end {
            let trailing_raw = &p_block[prev_span_end..p_content_end];
            if let Some(trailing_clean) = clean_inter_xml_text(trailing_raw) {
                let last_syl_ends_with_space = syllables
                    .last()
                    .is_some_and(|s: &Syllable| s.text.ends_with(' '));
                let to_add = if last_syl_ends_with_space && trailing_clean.starts_with(' ') {
                    trailing_clean.trim_start()
                } else {
                    &trailing_clean
                };
                if !to_add.is_empty() {
                    if let Some(last_syl) = syllables.last_mut() {
                        last_syl.text.push_str(to_add);
                    }
                    full_text.push_str(to_add);
                }
            }
        }

        if syllables.is_empty() {
            let raw_inner = strip_xml_tags(p_block);
            let raw_inner = unescape_xml_entities(&raw_inner);
            if !raw_inner.trim().is_empty() {
                let (syls, plain, inner_k) = parse_api_karaoke(&raw_inner);
                syllables = syls;
                full_text = plain;
                is_karaoke = inner_k;
            }
        }

        let singer_index = detect_singer_index(p_open_tag, div_open_tag, &full_text);

        if !full_text.trim().is_empty() {
            lines.push(LrcLine {
                time: p_begin,
                end_time: p_end_time,
                text: full_text.trim().to_string(),
                syllables,
                sub_text: None,
                style_name: "Default".to_string(),
                is_karaoke,
                singer_index,
            });
        }
    }

    lines.sort_by_key(|l| l.time);

    lines
}

pub fn parse_lrc(content: &str) -> Vec<LrcLine> {
    let trimmed = content.trim();
    let mut raw_lines =
        if trimmed.contains("<tt") || trimmed.contains("<p") || trimmed.contains("xmlns=") {
            let ttml_lines = parse_ttml(content);
            if !ttml_lines.is_empty() {
                ttml_lines
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

    if raw_lines.is_empty() {
        for raw_line in content.lines() {
            let trimmed_line = raw_line.trim();
            if trimmed_line.is_empty()
                || trimmed_line.starts_with(';')
                || trimmed_line.starts_with('#')
            {
                continue;
            }

            if trimmed_line.starts_with('[') {
                if let Some(close_idx) = trimmed_line.find(']') {
                    let time_str = &trimmed_line[1..close_idx];
                    let mut raw_text = trimmed_line[close_idx + 1..].trim();

                    let mut sub_text = None;
                    if let Some(sub_idx) = raw_text.find("|sub:") {
                        let sub_part = &raw_text[sub_idx + 5..];
                        if !sub_part.trim().is_empty() {
                            sub_text = Some(sub_part.trim().to_string());
                        }
                        raw_text = raw_text[..sub_idx].trim();
                    }

                    if let Some(base_time) = parse_lrc_time_str(time_str) {
                        let (syllables, plain_text, is_karaoke) = parse_api_karaoke(raw_text);
                        let singer_index = detect_singer_index("", "", &plain_text);
                        if !plain_text.is_empty() {
                            raw_lines.push(LrcLine {
                                time: base_time,
                                end_time: None,
                                text: plain_text,
                                syllables,
                                sub_text,
                                style_name: "Default".to_string(),
                                is_karaoke,
                                singer_index,
                            });
                        }
                    }
                }
            }
        }
    }

    raw_lines.sort_by_key(|l| l.time);

    // Merge dual-line lyrics (main lyric + translation sharing same/near timestamp)
    let mut lines: Vec<LrcLine> = Vec::new();
    for line in raw_lines {
        if let Some(last) = lines.last_mut() {
            let diff = line.time.saturating_sub(last.time).as_millis();
            if diff <= 150 {
                if last.sub_text.is_none() {
                    last.sub_text = Some(line.text);
                }
                continue;
            }
        }
        lines.push(line);
    }

    for i in 0..lines.len() {
        if lines[i].end_time.is_none() && i + 1 < lines.len() {
            lines[i].end_time = Some(lines[i + 1].time);
        }

        if !lines[i].is_karaoke {
            continue;
        }

        let line_duration_ms = if let Some(end) = lines[i].end_time {
            end.saturating_sub(lines[i].time).as_millis() as u64
        } else {
            4000
        };

        let has_custom_duration = lines[i]
            .syllables
            .iter()
            .any(|s| s.duration.as_millis() != 300);

        if !has_custom_duration {
            let total_chars: usize = lines[i]
                .syllables
                .iter()
                .map(|s| s.text.chars().count())
                .sum();
            if total_chars > 0 {
                let effective_ms = line_duration_ms.clamp(500, 15000);

                for syl in lines[i].syllables.iter_mut() {
                    let char_count = syl.text.chars().count() as u64;
                    let calculated_ms = (effective_ms * char_count) / (total_chars as u64);
                    syl.duration = Duration::from_millis(calculated_ms.max(50));
                }
            }
        }
    }

    lines
}

fn parse_lrc_time_str(s: &str) -> Option<Duration> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let mins: u64 = parts[0].parse().ok()?;
        let sec_parts: Vec<&str> = parts[1].split('.').collect();
        let secs: u64 = sec_parts[0].parse().ok()?;
        let millis = if sec_parts.len() > 1 {
            let m_str = sec_parts[1];
            if m_str.len() == 2 {
                m_str.parse::<u64>().ok()? * 10
            } else if m_str.len() == 1 {
                m_str.parse::<u64>().ok()? * 100
            } else {
                m_str[..3].parse::<u64>().ok()?
            }
        } else {
            0
        };
        Some(Duration::from_millis(mins * 60000 + secs * 1000 + millis))
    } else {
        None
    }
}

fn is_cjk(c: char) -> bool {
    let u = c as u32;
    (0x4E00..=0x9FFF).contains(&u) || // CJK Unified Ideographs
    (0x3040..=0x309F).contains(&u) || // Hiragana
    (0x30A0..=0x30FF).contains(&u) || // Katakana
    (0xAC00..=0xD7AF).contains(&u) // Hangul
}

fn parse_api_karaoke(input: &str) -> (Vec<Syllable>, String, bool) {
    let mut syllables = Vec::new();
    let mut plain_text = String::new();
    let mut has_word_timestamps = false;

    if input.contains('<') && input.contains('>') {
        let mut current_text = String::new();
        let mut in_duration = false;
        let mut dur_str = String::new();

        for ch in input.chars() {
            if ch == '<' {
                in_duration = true;
            } else if ch == '>' {
                in_duration = false;
                let parsed_dur = dur_str.parse::<u64>().ok();
                let dur_ms = parsed_dur.unwrap_or(300);
                if parsed_dur.is_some() {
                    has_word_timestamps = true;
                }

                if !current_text.is_empty() {
                    syllables.push(Syllable {
                        text: current_text.clone(),
                        duration: Duration::from_millis(dur_ms),
                    });
                    plain_text.push_str(&current_text);
                }

                current_text.clear();
                dur_str.clear();
            } else if in_duration {
                dur_str.push(ch);
            } else {
                current_text.push(ch);
            }
        }
        if !current_text.trim().is_empty() {
            syllables.push(Syllable {
                text: current_text.clone(),
                duration: Duration::from_millis(300),
            });
            plain_text.push_str(&current_text);
        }
    } else {
        let mut current_word = String::new();
        for ch in input.chars() {
            if is_cjk(ch) {
                if !current_word.is_empty() {
                    syllables.push(Syllable {
                        text: current_word.clone(),
                        duration: Duration::from_millis(300),
                    });
                    plain_text.push_str(&current_word);
                    current_word.clear();
                }
                syllables.push(Syllable {
                    text: ch.to_string(),
                    duration: Duration::from_millis(300),
                });
                plain_text.push(ch);
            } else {
                current_word.push(ch);
                if ch.is_whitespace() {
                    syllables.push(Syllable {
                        text: current_word.clone(),
                        duration: Duration::from_millis(300),
                    });
                    plain_text.push_str(&current_word);
                    current_word.clear();
                }
            }
        }
        if !current_word.is_empty() {
            syllables.push(Syllable {
                text: current_word.clone(),
                duration: Duration::from_millis(300),
            });
            plain_text.push_str(&current_word);
        }
    }

    (
        syllables,
        plain_text.trim().to_string(),
        has_word_timestamps,
    )
}

pub fn find_current_line(lines: &[LrcLine], pos: Duration) -> Option<usize> {
    if lines.is_empty() {
        return None;
    }
    let mut best_idx = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.time <= pos {
            best_idx = i;
        } else {
            break;
        }
    }
    Some(best_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_lrc_not_karaoke() {
        let lrc = "[00:10.00] Hello world";
        let lines = parse_lrc(lrc);
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].is_karaoke);
    }

    #[test]
    fn test_word_by_word_lrc_is_karaoke() {
        let lrc = "[00:10.00] <200>Hello <300>world";
        let lines = parse_lrc(lrc);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_karaoke);
    }

    #[test]
    fn test_ttml_with_spans_is_karaoke() {
        let ttml = r#"<tt><body><div><p begin="00:00:10.00"><span begin="00:00:10.00" end="00:00:10.50">Hello </span><span begin="00:00:10.50" end="00:00:11.00">world</span></p></div></body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].is_karaoke);
    }

    #[test]
    fn test_ttml_without_spans_not_karaoke() {
        let ttml = r#"<tt><body><div><p begin="00:00:10.00">Hello world</p></div></body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].is_karaoke);
    }

    #[test]
    fn test_ttml_translation_and_transliteration_ignored() {
        let ttml = r#"<tt xmlns:ttm="http://www.w3.org/ns/ttml#metadata"><body>
            <div ttm:role="main"><p begin="00:00:10.00">Main lyric</p></div>
            <div ttm:role="translation" xml:lang="zh-CN"><p begin="00:00:10.00">Chinese translation</p></div>
            <div ttm:role="transliteration" xml:lang="zh-Latn"><p begin="00:00:10.00">Pinyin transliteration</p></div>
        </body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Main lyric");
        assert!(lines[0].sub_text.is_none());
    }

    #[test]
    fn test_ttml_inline_span_translation_separated() {
        let ttml = r#"<tt xmlns:ttm="http://www.w3.org/ns/ttml#metadata"><body><div>
            <p begin="00:00:10.00" end="00:00:15.00">
                <span begin="00:00:10.00" end="00:00:12.00">Hello </span>
                <span begin="00:00:12.00" end="00:00:15.00">world</span>
                <span ttm:role="x-translation" xml:lang="zh-CN">你好世界</span>
                <span ttm:role="x-roman" xml:lang="zh-Latn">nǐ hǎo shì jiè</span>
            </p>
        </div></body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Hello world");
        assert_eq!(lines[0].syllables.len(), 2);
        assert!(lines[0].sub_text.is_none());
    }

    #[test]
    fn test_ttml_split_word_syllables_no_extra_space() {
        let ttml = r#"<tt><body><div>
            <p begin="00:00:24.00" end="00:00:25.10">
                <span begin="00:00:24.00" end="00:00:24.30">me</span>
                <span begin="00:00:24.30" end="00:00:25.10">ans</span>
            </p>
        </div></body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "means");
    }

    #[test]
    fn test_ttml_ending_no_extra_space() {
        let ttml = r#"<tt><body><div>
            <p begin="00:00:20.00" end="00:00:25.00">
                <span begin="00:00:20.00" end="00:00:21.00">If </span>
                <span begin="00:00:21.00" end="00:00:22.00">the </span>
                <span begin="00:00:22.00" end="00:00:23.00">world </span>
                <span begin="00:00:23.00" end="00:00:23.50">was </span>
                <span begin="00:00:23.50" end="00:00:24.00">en</span>
                <span begin="00:00:24.00" end="00:00:25.00">ding</span>
            </p>
        </div></body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "If the world was ending");
    }

    #[test]
    fn test_unescape_xml_entities() {
        assert_eq!(
            unescape_xml_entities("here&apos;s my number"),
            "here's my number"
        );
        assert_eq!(
            unescape_xml_entities("It&apos;s &quot;crazy&quot; &amp; fun"),
            "It's \"crazy\" & fun"
        );
        assert_eq!(unescape_xml_entities("Fish &#38; Chips"), "Fish & Chips");
        assert_eq!(unescape_xml_entities("It&#39;s work"), "It's work");
        assert_eq!(unescape_xml_entities("It&#x27;s work"), "It's work");
    }

    #[test]
    fn test_ttml_with_xml_entities() {
        let ttml = r#"<tt><body><div>
            <p begin="00:00:29.170" end="00:00:30.670">
                <span begin="00:00:29.170" end="00:00:29.350">But </span>
                <span begin="00:00:29.350" end="00:00:29.646">here&apos;s </span>
                <span begin="00:00:29.646" end="00:00:29.900">my </span>
                <span begin="00:00:29.900" end="00:00:30.670">number</span>
            </p>
        </div></body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "But here's my number");
        assert_eq!(lines[0].syllables[1].text.trim(), "here's");
    }

    #[test]
    fn test_ttml_unspanned_word_suffixes() {
        let ttml = r#"<tt><body><div>
            <p begin="00:00:17.00" end="00:00:19.00">
                <span begin="00:00:17.00" end="00:00:17.20">I </span>
                <span begin="00:00:17.20" end="00:00:17.60">wasn</span>&apos;t 
                <span begin="00:00:17.60" end="00:00:18.00">looking </span>for this
            </p>
            <p begin="00:00:19.10" end="00:00:21.00">
                <span begin="00:00:19.10" end="00:00:19.30">But </span>
                <span begin="00:00:19.30" end="00:00:19.50">now </span>
                <span begin="00:00:19.50" end="00:00:19.80">you</span>&apos;re 
                <span begin="00:00:19.80" end="00:00:20.50">in my way</span>
            </p>
        </div></body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "I wasn't looking for this");
        assert_eq!(lines[0].syllables[1].text.trim(), "wasn't");
        assert_eq!(lines[1].text, "But now you're in my way");
        assert_eq!(lines[1].syllables[2].text.trim(), "you're");
    }
}
