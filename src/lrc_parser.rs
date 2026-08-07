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

        let begin_val = extract_xml_attr(p_block, "begin");
        let end_val = extract_xml_attr(p_block, "end");

        let p_begin = match begin_val.as_deref().and_then(parse_ttml_time_str) {
            Some(t) => t,
            None => continue,
        };
        let p_end_time = end_val.as_deref().and_then(parse_ttml_time_str);

        let mut syllables = Vec::new();
        let mut full_text = String::new();

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

            let mut span_text = strip_xml_tags(span_text_raw);
            if span_text.is_empty() {
                continue;
            }

            let after_span = &p_block[s_end + 7..];
            let has_outside_space = after_span.starts_with(' ')
                || after_span.starts_with('\n')
                || after_span.starts_with('\r')
                || after_span.starts_with('\t');

            if has_outside_space && !span_text.ends_with(' ') {
                span_text.push(' ');
            }

            let s_begin = extract_xml_attr(tag_attrs, "begin")
                .as_deref()
                .and_then(parse_ttml_time_str);
            let s_end_t = extract_xml_attr(tag_attrs, "end")
                .as_deref()
                .and_then(parse_ttml_time_str);

            let dur = if let (Some(b), Some(e)) = (s_begin, s_end_t) {
                e.saturating_sub(b)
            } else {
                Duration::from_millis(300)
            };

            syllables.push(Syllable {
                text: span_text.clone(),
                duration: dur,
            });
            full_text.push_str(&span_text);
        }

        if syllables.is_empty() {
            let raw_inner = strip_xml_tags(p_block);
            if !raw_inner.trim().is_empty() {
                let (syls, plain) = parse_api_karaoke(&raw_inner);
                syllables = syls;
                full_text = plain;
            }
        }

        if !full_text.trim().is_empty() {
            lines.push(LrcLine {
                time: p_begin,
                end_time: p_end_time,
                text: full_text.trim().to_string(),
                syllables,
                sub_text: None,
                style_name: "Default".to_string(),
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
                        let (syllables, plain_text) = parse_api_karaoke(raw_text);
                        if !plain_text.is_empty() {
                            raw_lines.push(LrcLine {
                                time: base_time,
                                end_time: None,
                                text: plain_text,
                                syllables,
                                sub_text,
                                style_name: "Default".to_string(),
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

fn parse_api_karaoke(input: &str) -> (Vec<Syllable>, String) {
    let mut syllables = Vec::new();
    let mut plain_text = String::new();

    if input.contains('<') && input.contains('>') {
        let mut current_text = String::new();
        let mut in_duration = false;
        let mut dur_str = String::new();

        for ch in input.chars() {
            if ch == '<' {
                in_duration = true;
            } else if ch == '>' {
                in_duration = false;
                let dur_ms = dur_str.parse::<u64>().unwrap_or(300);

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

    (syllables, plain_text.trim().to_string())
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
