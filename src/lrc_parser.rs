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
    pub style_name: String,
}

pub fn parse_lrc(content: &str) -> Vec<LrcLine> {
    let mut lines = Vec::new();

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') {
            if let Some(close_idx) = trimmed.find(']') {
                let time_str = &trimmed[1..close_idx];
                let mut raw_text = trimmed[close_idx + 1..].trim();
                
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
                        lines.push(LrcLine {
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

    lines.sort_by_key(|l| l.time);

    for i in 0..lines.len() {
        if lines[i].end_time.is_none() && i + 1 < lines.len() {
            lines[i].end_time = Some(lines[i + 1].time);
        }

        let line_duration_ms = if let Some(end) = lines[i].end_time {
            end.saturating_sub(lines[i].time).as_millis() as u64
        } else {
            4000 
        };

        let has_custom_duration = lines[i].syllables.iter().any(|s| s.duration.as_millis() != 300);

        if !has_custom_duration {
            let total_chars: usize = lines[i].syllables.iter().map(|s| s.text.chars().count()).sum();
            if total_chars > 0 {
                let max_realistic_ms = (total_chars as u64 * 150).max(1500).min(5000);
                let effective_ms = line_duration_ms.min(max_realistic_ms);

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
            if m_str.len() == 2 { m_str.parse::<u64>().ok()? * 10 }
            else if m_str.len() == 1 { m_str.parse::<u64>().ok()? * 100 }
            else { m_str[..3].parse::<u64>().ok()? }
        } else { 0 };
        Some(Duration::from_millis(mins * 60000 + secs * 1000 + millis))
    } else {
        None
    }
}

// Fungsi helper untuk mendeteksi karakter CJK (China, Jepang, Korea)
fn is_cjk(c: char) -> bool {
    let u = c as u32;
    (u >= 0x4E00 && u <= 0x9FFF) || // CJK Unified Ideographs (Hanzi / Kanji)
    (u >= 0x3040 && u <= 0x309F) || // Hiragana
    (u >= 0x30A0 && u <= 0x30FF) || // Katakana
    (u >= 0xAC00 && u <= 0xD7AF)    // Hangul (Korea)
}

fn parse_api_karaoke(input: &str) -> (Vec<Syllable>, String) {
    let mut syllables = Vec::new();
    let mut plain_text = String::new();

    if input.contains('<') && input.contains('>') {
        // Mode Lirik Sinkronisasi Tingkat Lanjut (TTML / Musixmatch Custom)
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
        // Mode LRC Polos Biasa (Memerlukan pemisahan manual)
        let mut current_word = String::new();
        for ch in input.chars() {
            if is_cjk(ch) {
                // Jika karakter saat ini adalah CJK, simpan huruf sebelumnya (jika ada huruf latin)
                if !current_word.is_empty() {
                    syllables.push(Syllable {
                        text: current_word.clone(),
                        duration: Duration::from_millis(300),
                    });
                    plain_text.push_str(&current_word);
                    current_word.clear();
                }
                // Paksa pemisahan karakter: 1 Huruf CJK = 1 Suku Kata Animasi
                syllables.push(Syllable {
                    text: ch.to_string(),
                    duration: Duration::from_millis(300),
                });
                plain_text.push(ch);
            } else {
                // Untuk karakter latin, kumpulkan huruf hingga bertemu spasi
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
    if lines.is_empty() { return None; }
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