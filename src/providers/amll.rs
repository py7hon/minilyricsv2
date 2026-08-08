use crate::providers::http_debug::http_get_with_debug;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct AmllSearchResponse {
    #[serde(default)]
    data: Option<serde_json::Value>,
}

#[derive(Deserialize, Clone)]
struct AmllItem {
    #[serde(default)]
    id: Option<serde_json::Value>,
}

impl AmllItem {
    fn get_id_str(&self) -> Option<String> {
        let v = self.id.as_ref()?;
        if let Some(s) = v.as_str() {
            Some(s.to_string())
        } else if let Some(n) = v.as_u64() {
            Some(n.to_string())
        } else {
            v.as_i64().map(|n| n.to_string())
        }
    }
}

#[derive(Deserialize)]
struct AmllGetResponse {
    #[serde(default)]
    data: Option<AmllGetDetail>,
    #[serde(default)]
    lyrics: Option<String>,
    #[serde(default, rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
}

#[derive(Deserialize)]
struct AmllGetDetail {
    #[serde(default)]
    lyrics: Option<String>,
    #[serde(default, rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
}

fn parse_amll_items(v: &serde_json::Value) -> Option<Vec<AmllItem>> {
    if v.is_array() {
        serde_json::from_value(v.clone()).ok()
    } else if v.is_object() {
        if let Some(items_val) = v.get("items") {
            if items_val.is_array() {
                serde_json::from_value(items_val.clone()).ok()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn fetch_amll_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let enc_title = urlencoding::encode(clean_title);
    let enc_artist = urlencoding::encode(artist);
    let q_str = format!("{} {}", clean_title, artist);
    let enc_q = urlencoding::encode(&q_str);

    // 1. Try AMLL official search endpoints
    let search_urls = [
        format!(
            "https://api.amll.dev/v1/lyrics/search?musicName={}&artistName={}",
            enc_title, enc_artist
        ),
        format!("https://api.amll.dev/v1/lyrics/search?q={}", enc_q),
    ];

    // Try both AMLL search endpoints concurrently instead of sequentially.
    let (search_r0, search_r1) = tokio::join!(
        http_get_with_debug(client, &search_urls[0], "AMLL"),
        http_get_with_debug(client, &search_urls[1], "AMLL"),
    );

    let mut found_id = None;

    for body in [search_r0, search_r1].into_iter().flatten() {
        if body.trim().starts_with('{') {
            if let Ok(search_res) = serde_json::from_str::<AmllSearchResponse>(&body) {
                if let Some(data_val) = search_res.data {
                    if let Some(items) = parse_amll_items(&data_val) {
                        if !items.is_empty() {
                            if let Some(id_str) = items[0].get_id_str() {
                                found_id = Some(id_str);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Fetch full TTML by ID
    if let Some(first_id) = found_id {
        let get_url = format!("https://api.amll.dev/v1/lyrics/get?id={}", first_id);
        if let Ok(get_body) = http_get_with_debug(client, &get_url, "AMLL").await {
            if let Ok(get_res) = serde_json::from_str::<AmllGetResponse>(&get_body) {
                let ttml = get_res
                    .data
                    .as_ref()
                    .and_then(|d| d.lyrics.clone().or(d.synced_lyrics.clone()))
                    .or(get_res.lyrics)
                    .or(get_res.synced_lyrics);

                if let Some(ttml_str) = ttml {
                    if !ttml_str.trim().is_empty() {
                        return Ok(ttml_str);
                    }
                }
            }
        }
    }

    // 3. Fallback to /v1/lrclib/get
    let lrclib_url = format!(
        "https://api.amll.dev/v1/lrclib/get?track_name={}&artist_name={}",
        enc_title, enc_artist
    );
    if let Ok(lrc_body) = http_get_with_debug(client, &lrclib_url, "AMLL LRCLIB").await {
        if let Ok(get_res) = serde_json::from_str::<AmllGetResponse>(&lrc_body) {
            let ttml = get_res
                .data
                .as_ref()
                .and_then(|d| d.lyrics.clone().or(d.synced_lyrics.clone()))
                .or(get_res.lyrics)
                .or(get_res.synced_lyrics);

            if let Some(ttml_str) = ttml {
                if !ttml_str.trim().is_empty() {
                    return Ok(ttml_str);
                }
            }
        }
    }

    Err("AMLL returned no lyrics".into())
}
