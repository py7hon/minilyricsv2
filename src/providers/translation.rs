use reqwest::Client;
use serde_json::Value;

pub async fn translate_text(client: &Client, text: &str) -> Option<String> {
    let clean = text.trim();
    if clean.is_empty() || clean == "♪" || clean.chars().all(|c| c.is_ascii()) {
        return None;
    }

    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=en&dt=t&dt=rm&q={}",
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
        if let Some(last_item) = arr.last().and_then(|v| v.as_array()) {
            for idx in [2, 3, 0, 1] {
                if let Some(t) = last_item.get(idx).and_then(|v| v.as_str()) {
                    let cleaned = t.trim().to_string();
                    if !cleaned.is_empty()
                        && cleaned != clean
                        && !cleaned.chars().all(|c| c.is_numeric())
                    {
                        return Some(cleaned);
                    }
                }
            }
        }

        for item in arr {
            if let Some(item_arr) = item.as_array() {
                for idx in [2, 3] {
                    if let Some(t) = item_arr.get(idx).and_then(|v| v.as_str()) {
                        let cleaned = t.trim().to_string();
                        if !cleaned.is_empty()
                            && cleaned != clean
                            && !cleaned.chars().all(|c| c.is_numeric())
                        {
                            return Some(cleaned);
                        }
                    }
                }
            }
        }

        let mut parts = Vec::new();
        for item in arr {
            if let Some(item_arr) = item.as_array() {
                if let Some(p) = item_arr
                    .get(0)
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
                return Some(cleaned);
            }
        }
    }

    None
}
