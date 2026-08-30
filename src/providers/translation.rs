use crate::lrc_parser::{convert_hangul_text_to_romaja, fix_common_romaji_misreadings, is_cjk};
use pinyin::ToPinyin;
use reqwest::Client;
use serde_json::Value;

/// Offline local transliteration: Romaji (kakasi), Pinyin (pinyin crate).
/// Romaja stays algorithmic via convert_hangul_text_to_romaja.
pub fn transliterate_local(text: &str) -> Option<String> {
    let clean = text.trim();
    if clean.is_empty() {
        return None;
    }

    let has_jp = kakasi::is_japanese(clean) != kakasi::IsJapanese::False;
    if has_jp {
        let romaji = kakasi::convert(clean).romaji.trim().to_string();
        if !romaji.is_empty() && !romaji.chars().any(is_cjk) {
            return Some(romaji);
        }
    }

    let has_hanzi = clean
        .chars()
        .any(|c| (0x4E00..=0x9FFF).contains(&(c as u32)));
    let has_kana = clean
        .chars()
        .any(|c| (0x3040..=0x30FF).contains(&(c as u32)));
    if has_hanzi && !has_kana {
        let mut out = String::new();
        for ch in clean.chars() {
            if let Some(p) = ch.to_pinyin() {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(p.with_tone());
            } else {
                out.push(ch);
            }
        }
        let trimmed = out.trim().to_string();
        if !trimmed.is_empty() && trimmed != clean {
            return Some(trimmed);
        }
    }

    let has_hangul = clean
        .chars()
        .any(|c| (0xAC00..=0xD7AF).contains(&(c as u32)));
    if has_hangul {
        let romaja = convert_hangul_text_to_romaja(clean);
        if !romaja.trim().is_empty() {
            return Some(romaja.trim().to_string());
        }
    }

    None
}

pub async fn translate_text(client: &Client, text: &str) -> Option<String> {
    let clean = text.trim();
    if clean.is_empty() || clean == "♪" || clean.is_ascii() {
        return None;
    }

    // Local crates first: no network, no Google rate limits.
    if let Some(local) = transliterate_local(clean) {
        return Some(local);
    }

    let encoded = urlencoding::encode(clean);
    let urls = [
        format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=ja&dt=t&dt=rm&q={}",
            encoded
        ),
        format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=en&dt=t&dt=rm&q={}",
            encoded
        ),
    ];

    for url in &urls {
        if let Ok(resp) = client
            .get(url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
            )
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
        {
            if let Ok(val) = resp.json::<Value>().await {
                if let Some(parsed) = parse_google_translate_response(&val, clean) {
                    return Some(parsed);
                }
            }
        }
    }

    // Fallback: Algorithmic Korean Hangul Romanization if Google Translate fails or is blocked
    let has_hangul = clean.chars().any(|c| {
        let u = c as u32;
        (0xAC00..=0xD7AF).contains(&u)
    });
    if has_hangul {
        let romaja = convert_hangul_text_to_romaja(clean);
        if !romaja.trim().is_empty() {
            return Some(romaja.trim().to_string());
        }
    }

    None
}

pub fn parse_google_translate_response(val: &Value, clean: &str) -> Option<String> {
    if let Some(arr) = val.get(0).and_then(|v| v.as_array()) {
        // 1. Try to extract Romanization (dt=rm) from index 3 / 2 per segment
        let mut romaji_parts = Vec::new();
        for item in arr {
            if let Some(item_arr) = item.as_array() {
                let rom = item_arr
                    .get(3)
                    .or_else(|| item_arr.get(2))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty() && *s != clean && !s.chars().any(is_cjk));
                if let Some(r) = rom {
                    romaji_parts.push(r.to_string());
                }
            }
        }

        if !romaji_parts.is_empty() {
            let combined = romaji_parts.join(" ");
            let fixed = fix_common_romaji_misreadings(&combined, clean);
            if !fixed.trim().is_empty() && fixed.trim() != clean {
                return Some(fixed.trim().to_string());
            }
        }

        // 2. Check full-sentence romanization in last element of val[0] at index 2/3
        if let Some(last_item) = arr.last().and_then(|v| v.as_array()) {
            for elem in last_item.iter().skip(2) {
                if let Some(t) = elem.as_str() {
                    let cleaned = t.trim().to_string();
                    if !cleaned.is_empty()
                        && cleaned != clean
                        && !cleaned.chars().all(|c| c.is_numeric())
                        && !cleaned.chars().any(is_cjk)
                    {
                        let fixed = fix_common_romaji_misreadings(&cleaned, clean);
                        if !fixed.trim().is_empty() && fixed.trim() != clean {
                            return Some(fixed);
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_google_translate_romaji() {
        let payload = json!([[[
            "I want you to smile",
            "笑っていてほしくて",
            null,
            "Waratte ite hoshikute"
        ]]]);
        let res = parse_google_translate_response(&payload, "笑っていてほしくて").unwrap();
        assert_eq!(res, "Waratte ite hoshikute");
    }

    #[test]
    fn test_parse_google_translate_romaja() {
        let payload = json!([[["love it", "사랑해", null, "salanghae"]]]);
        let res = parse_google_translate_response(&payload, "사랑해").unwrap();
        assert_eq!(res, "saranghae");
    }

    #[test]
    fn test_parse_google_translate_pinyin() {
        let payload = json!([[["hello world", "你好世界", null, "Nǐ hǎo shì jiè"]]]);
        let res = parse_google_translate_response(&payload, "你好世界").unwrap();
        assert_eq!(res, "Nǐ hǎo shì jiè");
    }
}
