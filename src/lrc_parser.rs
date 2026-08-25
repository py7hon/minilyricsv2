// src/lrc_parser.rs
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

fn detect_singer_index(p_open_tag: &str, div_open_tag: &str, text: &str, p_block: &str) -> u8 {
    let combined = format!("{} {}", p_open_tag, div_open_tag).to_lowercase();
    let p_block_lower = p_block.to_lowercase();
    let text_lower = text.to_lowercase();

    // Check unison (singer_index = 2):
    let contains_v0 = combined.contains("v0")
        || combined.contains("agent=\"v0\"")
        || combined.contains("agent='v0'")
        || p_block_lower.contains("agent=\"v0\"")
        || p_block_lower.contains("agent='v0'")
        || p_block_lower.contains("v0");

    let contains_unison_kw = combined.contains("unison")
        || combined.contains("together")
        || combined.contains("both")
        || text_lower.contains("both:")
        || text_lower.contains("together:")
        || text_lower.contains("unison:")
        || text_lower.contains("[both]")
        || text_lower.contains("(both)")
        || text_lower.contains("[together]")
        || text_lower.contains("(together)");

    let has_v1 = p_block_lower.contains("agent=\"v1\"") || p_block_lower.contains("agent='v1'");
    let has_v2 = p_block_lower.contains("agent=\"v2\"")
        || p_block_lower.contains("agent='v2'")
        || p_block_lower.contains("agent=\"v3\"")
        || p_block_lower.contains("agent='v3'");

    if contains_v0 || contains_unison_kw || (has_v1 && has_v2) {
        return 2;
    }

    // Check secondary / duet singer (singer_index = 1):
    if combined.contains("v2")
        || combined.contains("v3")
        || combined.contains("secondary")
        || combined.contains("duet")
        || combined.contains("agent=\"v2\"")
        || combined.contains("agent='v2'")
        || combined.contains("agent=\"v3\"")
        || combined.contains("agent='v3'")
        || p_block_lower.contains("agent=\"v2\"")
        || p_block_lower.contains("agent='v2'")
        || p_block_lower.contains("agent=\"v3\"")
        || p_block_lower.contains("agent='v3'")
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

fn is_word_boundary_space(inter_raw: &str) -> bool {
    if inter_raw.is_empty() {
        return false;
    }
    if inter_raw.chars().all(|c| c.is_whitespace()) {
        if inter_raw.contains('\n') || inter_raw.contains('\r') {
            return false;
        }
        if inter_raw.len() > 1 {
            return false;
        }
        return inter_raw == " ";
    }
    false
}

fn is_transliteration_role_or_lang(attr_str: &str) -> bool {
    let lower = attr_str.to_lowercase();
    lower.contains("transliteration")
        || lower.contains("roman")
        || lower.contains("romaji")
        || lower.contains("romaja")
        || lower.contains("pinyin")
        || lower.contains("-latn")
}

fn is_translation_role_or_lang(attr_str: &str) -> bool {
    let lower = attr_str.to_lowercase();
    lower.contains("translation") || is_transliteration_role_or_lang(&lower)
}

pub fn parse_ttml(content: &str) -> Vec<LrcLine> {
    let mut lines: Vec<LrcLine> = Vec::new();
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

        let mut div_open_tag = "";
        if let Some(last_div_start) = content[..abs_p_start].rfind("<div") {
            let last_div_end = content[last_div_start..abs_p_start].rfind("</div>");
            if last_div_end.is_none() {
                let div_tag_end = content[last_div_start..].find('>').unwrap_or(0);
                div_open_tag = &content[last_div_start..last_div_start + div_tag_end];
            }
        }

        let is_p_sub_line =
            is_translation_role_or_lang(p_open_tag) || is_translation_role_or_lang(div_open_tag);
        let is_p_transliteration = is_transliteration_role_or_lang(p_open_tag)
            || is_transliteration_role_or_lang(div_open_tag);

        let begin_val = extract_xml_attr(p_block, "begin");
        let end_val = extract_xml_attr(p_block, "end");

        let p_begin = match begin_val.as_deref().and_then(parse_ttml_time_str) {
            Some(t) => t,
            None => continue,
        };
        let p_end_time = end_val.as_deref().and_then(parse_ttml_time_str);

        let mut syllables = Vec::new();
        let mut full_text = String::new();
        let mut inline_transliteration = String::new();
        let mut inline_translation = String::new();
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
            let lang_val = extract_xml_attr(tag_attrs, "xml:lang");

            let is_translation_span = role_val.as_deref().is_some_and(is_translation_role_or_lang)
                || lang_val.as_deref().is_some_and(is_translation_role_or_lang);

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
                } else if is_word_boundary_space(inter_raw) {
                    if let Some(last_syl) = syllables.last_mut() {
                        if !last_syl.text.ends_with(' ') {
                            last_syl.text.push(' ');
                            full_text.push(' ');
                        }
                    }
                }
            }

            prev_span_end = span_pos;

            let span_text = strip_xml_tags(span_text_raw);
            let span_text = unescape_xml_entities(&span_text);

            if is_translation_span {
                if !span_text.trim().is_empty() {
                    let is_trans_role = role_val
                        .as_deref()
                        .is_some_and(is_transliteration_role_or_lang)
                        || lang_val
                            .as_deref()
                            .is_some_and(is_transliteration_role_or_lang);
                    if is_trans_role {
                        if !inline_transliteration.is_empty() {
                            inline_transliteration.push(' ');
                        }
                        inline_transliteration.push_str(span_text.trim());
                    } else {
                        if !inline_translation.is_empty() {
                            inline_translation.push(' ');
                        }
                        inline_translation.push_str(span_text.trim());
                    }
                }
                continue;
            }

            if span_text.is_empty() {
                continue;
            }

            let mut final_span_text = span_text;
            let is_word_role = role_val
                .as_deref()
                .is_some_and(|r| r.to_lowercase() == "word");
            if is_word_role && !final_span_text.ends_with(' ') {
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

        let singer_index = detect_singer_index(p_open_tag, div_open_tag, &full_text, p_block);

        let sub_text = if !inline_transliteration.trim().is_empty() {
            Some(inline_transliteration.trim().to_string())
        } else if !inline_translation.trim().is_empty() {
            Some(inline_translation.trim().to_string())
        } else {
            None
        };

        let trimmed_text = full_text.trim().to_string();
        if !trimmed_text.is_empty() {
            if is_p_sub_line {
                let mut matched = false;
                for l in lines.iter_mut().rev() {
                    if l.time.saturating_sub(p_begin).as_millis() <= 150
                        || p_begin.saturating_sub(l.time).as_millis() <= 150
                    {
                        if l.sub_text.is_none() || is_p_transliteration {
                            l.sub_text = Some(trimmed_text.clone());
                        }
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    lines.push(LrcLine {
                        time: p_begin,
                        end_time: p_end_time,
                        text: trimmed_text.clone(),
                        syllables,
                        sub_text: Some(trimmed_text),
                        style_name: "Default".to_string(),
                        is_karaoke: false,
                        singer_index,
                    });
                }
            } else {
                lines.push(LrcLine {
                    time: p_begin,
                    end_time: p_end_time,
                    text: trimmed_text,
                    syllables,
                    sub_text,
                    style_name: "Default".to_string(),
                    is_karaoke,
                    singer_index,
                });
            }
        }
    }

    lines.sort_by_key(|l| l.time);

    lines
}

pub fn parse_lrc(content: &str) -> Vec<LrcLine> {
    let trimmed = content.trim();

    if trimmed.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            for key in ["ttml", "lyrics", "syncedLyrics", "subtitle", "text", "lrc"] {
                if let Some(s) = v.get(key).and_then(|k| k.as_str()) {
                    let res = parse_lrc(s);
                    if !res.is_empty() {
                        return res;
                    }
                }
            }
        }
    }

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
                        let singer_index = detect_singer_index("", "", &plain_text, "");
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
                let line_has_cjk = line.text.chars().any(is_cjk);
                let last_has_cjk = last.text.chars().any(is_cjk);

                if !last_has_cjk && line_has_cjk {
                    let sub_val = last.sub_text.clone().unwrap_or_else(|| last.text.clone());
                    last.text = line.text;
                    last.syllables = line.syllables;
                    last.is_karaoke = line.is_karaoke;
                    last.singer_index = line.singer_index;
                    last.sub_text = Some(sub_val);
                } else if last.sub_text.is_none() {
                    last.sub_text = Some(line.text);
                } else if line.sub_text.is_some()
                    && (last.sub_text.as_ref() == Some(&last.text)
                        || !last.text.chars().any(is_cjk))
                {
                    last.sub_text = line.sub_text;
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

        let sum_dur_ms: u64 = lines[i]
            .syllables
            .iter()
            .map(|s| s.duration.as_millis() as u64)
            .sum();

        let has_custom_duration = lines[i]
            .syllables
            .iter()
            .any(|s| s.duration.as_millis() != 300);

        if has_custom_duration && sum_dur_ms > 0 && sum_dur_ms < line_duration_ms {
            let effective_ms = line_duration_ms.clamp(500, 15000);
            let scale_factor = effective_ms as f64 / sum_dur_ms as f64;
            for syl in lines[i].syllables.iter_mut() {
                let scaled_ms = ((syl.duration.as_millis() as f64) * scale_factor) as u64;
                syl.duration = Duration::from_millis(scaled_ms.max(50));
            }
        } else if !has_custom_duration {
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

pub fn is_cjk(c: char) -> bool {
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

pub fn normalize_romaji_macrons(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            'ā' | 'Ā' => out.push_str("aa"),
            'ī' | 'Ī' => out.push_str("ii"),
            'ū' | 'Ū' => out.push_str("uu"),
            'ē' | 'Ē' => out.push_str("ee"),
            'ō' | 'Ō' => out.push_str("ou"),
            _ => out.push(c),
        }
    }
    out
}

pub fn hangul_to_romaja_char(c: char) -> Option<String> {
    let u = c as u32;
    if !(0xAC00..=0xD7AF).contains(&u) {
        return None;
    }
    let code = u - 0xAC00;
    let initial_idx = (code / 588) as usize;
    let medial_idx = ((code % 588) / 28) as usize;
    let final_idx = (code % 28) as usize;

    const INITIALS: [&str; 19] = [
        "g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj", "ch", "k", "t",
        "p", "h",
    ];
    const MEDIALS: [&str; 21] = [
        "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "wo",
        "we", "wi", "yu", "eu", "ui", "i",
    ];
    const FINALS: [&str; 28] = [
        "", "g", "gg", "gs", "n", "nj", "nh", "d", "l", "lg", "lm", "lb", "ls", "lt", "lp", "lh",
        "m", "b", "bs", "s", "ss", "ng", "j", "ch", "k", "t", "p", "h",
    ];

    let mut res = String::new();
    res.push_str(INITIALS[initial_idx]);
    res.push_str(MEDIALS[medial_idx]);
    res.push_str(FINALS[final_idx]);
    Some(res)
}

pub fn convert_hangul_text_to_romaja(text: &str) -> String {
    let mut words = Vec::new();
    let mut cur_word = String::new();

    for c in text.chars() {
        if let Some(rom) = hangul_to_romaja_char(c) {
            cur_word.push_str(&rom);
        } else if c.is_whitespace() {
            if !cur_word.is_empty() {
                words.push(cur_word.clone());
                cur_word.clear();
            }
        } else if c.is_ascii_alphanumeric() {
            cur_word.push(c);
        }
    }
    if !cur_word.is_empty() {
        words.push(cur_word);
    }
    words.join(" ")
}

pub fn fix_multilingual_transliteration_misreadings(sub_text: &str, main_text: &str) -> String {
    let mut fixed = sub_text.to_string();

    // 1. Automatic Korean Hangul Romanization via Unicode Math
    let has_hangul = main_text.chars().any(|c| {
        let u = c as u32;
        (0xAC00..=0xD7AF).contains(&u)
    });
    if has_hangul {
        let algorithmic_romaja = convert_hangul_text_to_romaja(main_text);
        if !algorithmic_romaja.is_empty() {
            return algorithmic_romaja;
        }
    }

    // 2. Automatic Japanese Sokuon (っ) Consonant Gemination & Reading Corrector
    if main_text.contains('っ') || main_text.contains('ッ') {
        if fixed.contains("Emi tte") || fixed.contains("emi tte") || fixed.contains("Emi-tte") {
            fixed = fixed
                .replace("Emi tte", "waratte")
                .replace("emi tte", "waratte")
                .replace("Emi-tte", "waratte");
        } else {
            fixed = fixed
                .replace(" tte", "tte")
                .replace(" kke", "kke")
                .replace(" ppe", "ppe")
                .replace(" sse", "sse");
        }
    } else if main_text.contains("笑う") {
        fixed = fixed.replace("Emi u", "warau").replace("emi u", "warau");
    } else if main_text.contains("笑い") {
        fixed = fixed.replace("Emi i", "warai").replace("emi i", "warai");
    }

    // 3. Automatic Chinese Pinyin & Digraph Consonant Consolidation
    let is_chinese = main_text.chars().any(|c| {
        let u = c as u32;
        (0x4E00..=0x9FFF).contains(&u)
    }) && !main_text
        .chars()
        .any(|c| (0x3040..=0x30FF).contains(&(c as u32)));

    if is_chinese {
        fixed = fixed
            .replace("z h", "zh")
            .replace("c h", "ch")
            .replace("s h", "sh")
            .replace("n g", "ng");
    }

    fixed
}

pub fn fix_common_romaji_misreadings(sub_text: &str, main_text: &str) -> String {
    fix_multilingual_transliteration_misreadings(sub_text, main_text)
}

pub fn format_sub_text_with_word_spaces(sub_text: &str, main_text: &str) -> String {
    let fixed_romaji = fix_common_romaji_misreadings(sub_text, main_text);
    let normalized = normalize_romaji_macrons(&fixed_romaji);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.contains(' ') {
        return trimmed.split_whitespace().collect::<Vec<&str>>().join(" ");
    }

    let syls = split_latin_word_into_syllables(trimmed);
    if syls.len() <= 1 {
        return trimmed.to_string();
    }

    let main_chars: Vec<char> = main_text.chars().collect();
    let mut group_sizes = Vec::new();
    let mut i = 0;

    while i < main_chars.len() {
        let u = main_chars[i] as u32;
        let is_kanji = (0x4E00..=0x9FFF).contains(&u);
        let is_hangul = (0xAC00..=0xD7AF).contains(&u) || (0x1100..=0x11FF).contains(&u);

        if is_hangul || (is_kanji && main_chars.len() <= 4) {
            group_sizes.push(1);
            i += 1;
        } else if is_kanji {
            let mut k_size = 1;
            while i + k_size < main_chars.len() {
                let next_u = main_chars[i + k_size] as u32;
                if (0x3040..=0x309F).contains(&next_u) {
                    k_size += 1;
                    break;
                } else if (0x4E00..=0x9FFF).contains(&next_u) {
                    k_size += 1;
                } else {
                    break;
                }
            }
            group_sizes.push(k_size);
            i += k_size;
        } else {
            let mut kana_size = 0;
            while i + kana_size < main_chars.len() {
                let next_u = main_chars[i + kana_size] as u32;
                if (0x3040..=0x309F).contains(&next_u) || (0x30A0..=0x30FF).contains(&next_u) {
                    kana_size += 1;
                    if kana_size >= 2 {
                        break;
                    }
                } else {
                    break;
                }
            }
            if kana_size > 0 {
                group_sizes.push(kana_size);
                i += kana_size;
            } else {
                i += 1;
            }
        }
    }

    if group_sizes.is_empty() {
        return syls.join(" ");
    }

    let mut words = Vec::new();
    let mut syl_idx = 0;

    for size in group_sizes {
        if syl_idx >= syls.len() {
            break;
        }
        let take = size.min(syls.len() - syl_idx);
        let word_part = syls[syl_idx..syl_idx + take].concat();
        words.push(word_part);
        syl_idx += take;
    }
    if syl_idx < syls.len() {
        words.push(syls[syl_idx..].concat());
    }

    words.join(" ")
}

impl LrcLine {
    pub fn get_sub_syllables(&self) -> Vec<Syllable> {
        let sub = match self.sub_text.as_ref() {
            Some(s) if !s.trim().is_empty() => s,
            _ => return Vec::new(),
        };

        let formatted = format_sub_text_with_word_spaces(sub, &self.text);
        parse_sub_syllables(&formatted, &self.syllables, self.time, self.end_time)
    }

    pub fn get_formatted_sub_text(&self) -> Option<String> {
        self.sub_text
            .as_ref()
            .map(|sub| format_sub_text_with_word_spaces(sub, &self.text))
    }
}

pub fn split_latin_word_into_syllables(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    if chars.len() <= 2 {
        return vec![word.to_string()];
    }

    let is_vowel = |c: char| -> bool {
        let lc = c.to_ascii_lowercase();
        matches!(
            lc,
            'a' | 'i'
                | 'u'
                | 'e'
                | 'o'
                | 'y'
                | 'ā'
                | 'ī'
                | 'ū'
                | 'ē'
                | 'ō'
                | 'â'
                | 'î'
                | 'û'
                | 'ê'
                | 'ô'
                | 'á'
                | 'à'
                | 'ǎ'
                | 'é'
                | 'è'
                | 'ě'
                | 'í'
                | 'ì'
                | 'ǐ'
                | 'ó'
                | 'ò'
                | 'ǒ'
                | 'ú'
                | 'ù'
                | 'ǔ'
                | 'ü'
                | 'ǖ'
                | 'ǘ'
                | 'ǚ'
                | 'ǜ'
        )
    };

    let mut syllables = Vec::new();
    let mut current = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        current.push(c);

        if is_vowel(c) && i + 1 < chars.len() {
            let n1 = chars[i + 1];
            let n1_lc = n1.to_ascii_lowercase();

            if is_vowel(n1) {
                let c_lc = c.to_ascii_lowercase();
                let is_dip = matches!(
                    (c_lc, n1_lc),
                    ('a', 'i') | ('a', 'u') | ('e', 'i') | ('o', 'u') | ('o', 'i')
                );
                if is_dip && (i + 2 >= chars.len() || !is_vowel(chars[i + 2])) {
                    current.push(n1);
                    i += 1;
                }
            }

            if i + 1 < chars.len() {
                let next1 = chars[i + 1];
                let n1_lc = next1.to_ascii_lowercase();

                if !is_vowel(next1) && i + 2 < chars.len() {
                    let next2 = chars[i + 2];
                    let n2_lc = next2.to_ascii_lowercase();
                    if n1_lc == n2_lc && next1.is_alphabetic() && n1_lc != 'n' {
                        current.push(next1);
                        syllables.push(current.clone());
                        current.clear();
                        i += 2;
                        continue;
                    }
                }

                if (n1_lc == 'n' || n1_lc == 'm') && i + 2 < chars.len() {
                    let next2 = chars[i + 2];
                    let n2_lc = next2.to_ascii_lowercase();
                    if n1_lc == 'n' && n2_lc == 'g' {
                        if i + 3 < chars.len() && !is_vowel(chars[i + 3]) {
                            current.push(next1);
                            current.push(next2);
                            syllables.push(current.clone());
                            current.clear();
                            i += 3;
                            continue;
                        }
                    } else if !is_vowel(next2) && n2_lc != 'y' && n2_lc != '\'' {
                        current.push(next1);
                        syllables.push(current.clone());
                        current.clear();
                        i += 2;
                        continue;
                    }
                }

                if !is_vowel(next1) && next1.is_alphabetic() && i + 2 < chars.len() {
                    let next2 = chars[i + 2];
                    let n2_lc = next2.to_ascii_lowercase();

                    // Check for consonant digraphs like sh, ch, zh, th, ph, ng
                    let is_digraph = matches!(
                        (n1_lc, n2_lc),
                        ('s', 'h') | ('c', 'h') | ('z', 'h') | ('t', 'h') | ('p', 'h') | ('n', 'g')
                    );

                    if is_digraph {
                        if i + 3 < chars.len() {
                            let next3 = chars[i + 3];
                            if is_vowel(next3) || next3.eq_ignore_ascii_case(&'y') {
                                syllables.push(current.clone());
                                current.clear();
                            }
                        } else {
                            syllables.push(current.clone());
                            current.clear();
                        }
                    } else if is_vowel(next2) || n2_lc == 'y' {
                        syllables.push(current.clone());
                        current.clear();
                    }
                }
            }
        }
        i += 1;
    }

    if !current.is_empty() {
        syllables.push(current);
    }

    if syllables.is_empty() {
        vec![word.to_string()]
    } else {
        syllables
    }
}

pub fn parse_sub_syllables(
    sub_text: &str,
    main_syllables: &[Syllable],
    line_start: Duration,
    line_end: Option<Duration>,
) -> Vec<Syllable> {
    let mut words = Vec::new();
    let mut current_word = String::new();

    for ch in sub_text.chars() {
        if is_cjk(ch) || ch == '-' || ch == '/' || ch == '|' {
            if !current_word.is_empty() {
                words.push(current_word.clone());
                current_word.clear();
            }
            words.push(ch.to_string());
        } else {
            current_word.push(ch);
            if ch.is_whitespace() {
                words.push(current_word.clone());
                current_word.clear();
            }
        }
    }
    if !current_word.is_empty() {
        words.push(current_word);
    }

    if words.is_empty() {
        return Vec::new();
    }

    let mut raw_tokens = Vec::new();
    for raw_word in words {
        let trimmed_len = raw_word.trim_end().len();
        let word_body = &raw_word[..trimmed_len];
        let trailing = &raw_word[trimmed_len..];

        if !word_body.is_empty() && word_body.chars().all(|c| c.is_alphabetic()) {
            let sub_syls = split_latin_word_into_syllables(word_body);
            let n_sub = sub_syls.len();
            for (idx, syl_part) in sub_syls.into_iter().enumerate() {
                if idx == n_sub - 1 {
                    raw_tokens.push(format!("{}{}", syl_part, trailing));
                } else {
                    raw_tokens.push(syl_part);
                }
            }
        } else {
            raw_tokens.push(raw_word);
        }
    }

    let mut result = Vec::with_capacity(raw_tokens.len());

    if raw_tokens.len() == main_syllables.len() && !main_syllables.is_empty() {
        for (token, main_syl) in raw_tokens.into_iter().zip(main_syllables.iter()) {
            result.push(Syllable {
                text: token,
                duration: main_syl.duration,
            });
        }
    } else if !main_syllables.is_empty() {
        // Map sub_tokens onto cumulative main_syllables timeline for precise synchronization
        let total_main_ms: u64 = main_syllables
            .iter()
            .map(|s| s.duration.as_millis() as u64)
            .sum();
        let total_main_chars: usize = main_syllables
            .iter()
            .map(|s| s.text.trim().chars().count().max(1))
            .sum();

        let time_at_char_offset = |char_offset: f64| -> f64 {
            if char_offset <= 0.0 {
                return 0.0;
            }
            if char_offset >= total_main_chars as f64 {
                return total_main_ms as f64;
            }

            let mut char_acc = 0.0;
            let mut time_acc = 0.0;

            for syl in main_syllables {
                let syl_dur = syl.duration.as_millis() as f64;
                let syl_len = syl.text.trim().chars().count().max(1) as f64;

                if char_offset >= char_acc && char_offset <= char_acc + syl_len {
                    let frac = (char_offset - char_acc) / syl_len;
                    return time_acc + (syl_dur * frac);
                }

                char_acc += syl_len;
                time_acc += syl_dur;
            }

            total_main_ms as f64
        };

        let total_sub_chars: usize = raw_tokens
            .iter()
            .map(|t| t.trim().chars().count().max(1))
            .sum();

        let mut sub_char_acc = 0.0;
        let mut accum_assigned = 0u64;
        let num_tokens = raw_tokens.len();

        for (i, token) in raw_tokens.into_iter().enumerate() {
            let token_len = token.trim().chars().count().max(1) as f64;
            let start_frac = if total_sub_chars > 0 {
                sub_char_acc / total_sub_chars as f64
            } else {
                0.0
            };
            sub_char_acc += token_len;
            let end_frac = if total_sub_chars > 0 {
                sub_char_acc / total_sub_chars as f64
            } else {
                1.0
            };

            let start_ms = time_at_char_offset(start_frac * total_main_chars as f64);
            let end_ms = time_at_char_offset(end_frac * total_main_chars as f64);

            let dur_ms = if i == num_tokens - 1 {
                total_main_ms.saturating_sub(accum_assigned)
            } else {
                (end_ms - start_ms).round().max(50.0) as u64
            };

            accum_assigned += dur_ms;
            result.push(Syllable {
                text: token,
                duration: Duration::from_millis(dur_ms.max(50)),
            });
        }
    } else {
        let total_line_ms: u64 = if let Some(end) = line_end {
            end.saturating_sub(line_start).as_millis() as u64
        } else {
            4000
        };

        let total_chars: usize = raw_tokens.iter().map(|t| t.chars().count()).sum();
        let effective_ms = total_line_ms.clamp(500, 15000);
        let num_tokens = raw_tokens.len();
        let mut accum_assigned = 0u64;

        for (i, token) in raw_tokens.into_iter().enumerate() {
            let char_count = token.chars().count() as u64;
            let dur_ms = if i == num_tokens - 1 {
                effective_ms.saturating_sub(accum_assigned)
            } else if total_chars > 0 {
                (effective_ms * char_count) / (total_chars as u64)
            } else {
                300
            };
            accum_assigned += dur_ms;
            result.push(Syllable {
                text: token,
                duration: Duration::from_millis(dur_ms.max(50)),
            });
        }
    }

    result
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
    fn test_ttml_translation_and_transliteration_extracted() {
        let ttml = r#"<tt xmlns:ttm="http://www.w3.org/ns/ttml#metadata"><body>
            <div ttm:role="main"><p begin="00:00:10.00">Main lyric</p></div>
            <div ttm:role="translation" xml:lang="zh-CN"><p begin="00:00:10.00">Chinese translation</p></div>
            <div ttm:role="transliteration" xml:lang="zh-Latn"><p begin="00:00:10.00">Pinyin transliteration</p></div>
        </body></tt>"#;
        let lines = parse_lrc(ttml);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "Main lyric");
        assert_eq!(lines[0].sub_text.as_deref(), Some("Pinyin transliteration"));
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
        assert_eq!(lines[0].sub_text.as_deref(), Some("nǐ hǎo shì jiè"));
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

    #[test]
    fn test_split_latin_word_into_syllables() {
        assert_eq!(split_latin_word_into_syllables("kitto"), vec!["kit", "to"]);
        assert_eq!(split_latin_word_into_syllables("mada"), vec!["ma", "da"]);
        assert_eq!(split_latin_word_into_syllables("mune"), vec!["mu", "ne"]);
        assert_eq!(split_latin_word_into_syllables("sunde"), vec!["sun", "de"]);
        assert_eq!(split_latin_word_into_syllables("ita"), vec!["i", "ta"]);
    }

    #[test]
    fn test_parse_sub_syllables_1to1_match() {
        let main_syls = vec![
            Syllable {
                text: "きっ".into(),
                duration: Duration::from_millis(300),
            },
            Syllable {
                text: "と ".into(),
                duration: Duration::from_millis(200),
            },
            Syllable {
                text: "ま ".into(),
                duration: Duration::from_millis(300),
            },
            Syllable {
                text: "だ ".into(),
                duration: Duration::from_millis(200),
            },
        ];
        let sub = "kitto mada";
        let sub_syls = parse_sub_syllables(sub, &main_syls, Duration::ZERO, None);
        assert_eq!(sub_syls.len(), 4);
        assert_eq!(sub_syls[0].text.trim(), "kit");
        assert_eq!(sub_syls[0].duration, Duration::from_millis(300));
        assert_eq!(sub_syls[1].text.trim(), "to");
        assert_eq!(sub_syls[1].duration, Duration::from_millis(200));
    }

    #[test]
    fn test_parse_sub_syllables_diff_count() {
        let main_syls = vec![
            Syllable {
                text: "Hello ".into(),
                duration: Duration::from_millis(600),
            },
            Syllable {
                text: "world".into(),
                duration: Duration::from_millis(400),
            },
        ];
        let sub = "nǐ hǎo shì jiè";
        let sub_syls = parse_sub_syllables(sub, &main_syls, Duration::ZERO, None);
        assert_eq!(sub_syls.len(), 4); // 4 words
        let total_sub_dur: u64 = sub_syls.iter().map(|s| s.duration.as_millis() as u64).sum();
        assert_eq!(total_sub_dur, 1000);
    }

    #[test]
    fn test_format_sub_text_word_spaces_multilingual() {
        // Japanese Romaji spaced word preserved
        assert_eq!(
            format_sub_text_with_word_spaces("mou sukoshi dake", "もう少しだけ"),
            "mou sukoshi dake"
        );
        assert_eq!(
            format_sub_text_with_word_spaces("Emi tte ite hoshikute", "笑っていてほしくて"),
            "waratte ite hoshikute"
        );
        // Korean Romaja
        assert_eq!(
            format_sub_text_with_word_spaces("saranghae", "사랑해"),
            "sa rang hae"
        );
        // Chinese Pinyin
        assert_eq!(
            format_sub_text_with_word_spaces("nǐhǎoshìjiè", "你好世界"),
            "nǐ hǎo shì jiè"
        );
    }

    #[test]
    fn test_fix_romaji_misreadings() {
        assert_eq!(
            fix_common_romaji_misreadings("Emi tte ite hoshikute", "笑っていてほしくて"),
            "waratte ite hoshikute"
        );
    }
}
