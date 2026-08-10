use crate::lrc_parser::fix_common_romaji_misreadings;
use reqwest::Client;
use serde_json::Value;

pub async fn translate_text(client: &Client, text: &str) -> Option<String> {
    let clean = text.trim();
    if clean.is_empty() || clean == "♪" || clean.is_ascii() {
        return None;
    }

    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=ja&dt=t&dt=rm&q={}",
        urlencoding::encode(clean)
    );

    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .ok()?;

    let val: Value = resp.json().await.ok()?;

    if let Some(arr) = val.get(0).and_then(|v| v.as_array()) {
        // 1. Try to extract Romanization (dt=rm) from item[3] / item[2] per segment
        let mut romaji_parts = Vec::new();
        for item in arr {
            if let Some(item_arr) = item.as_array() {
                let rom = item_arr
                    .get(3)
                    .or_else(|| item_arr.get(2))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty());
                if let Some(r) = rom {
                    romaji_parts.push(r.trim().to_string());
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

        // 2. Check full-sentence romanization in last element of val[0]
        if let Some(last_item) = arr.last().and_then(|v| v.as_array()) {
            for elem in last_item {
                if let Some(t) = elem.as_str() {
                    let cleaned = t.trim().to_string();
                    if !cleaned.is_empty()
                        && cleaned != clean
                        && !cleaned.chars().all(|c| c.is_numeric())
                        && cleaned.is_ascii()
                    {
                        let fixed = fix_common_romaji_misreadings(&cleaned, clean);
                        return Some(fixed);
                    }
                }
            }
        }

        // 3. Fall back to translated text
        let mut parts = Vec::new();
        for item in arr {
            if let Some(item_arr) = item.as_array() {
                if let Some(p) = item_arr
                    .first()
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    parts.push(p.trim().to_string());
                }
            }
        }
        if !parts.is_empty() {
            let combined = parts.join(" ");
            let cleaned = combined.split_whitespace().collect::<Vec<&str>>().join(" ");
            if !cleaned.is_empty() && cleaned != clean {
                let fixed = fix_common_romaji_misreadings(&cleaned, clean);
                return Some(fixed);
            }
        }
    }

    None
}
