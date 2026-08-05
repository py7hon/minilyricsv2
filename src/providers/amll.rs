use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct AmllSearchResponse {
    #[serde(default)]
    data: Option<AmllSearchData>,
}

#[derive(Deserialize)]
struct AmllSearchData {
    #[serde(default)]
    items: Option<Vec<AmllItem>>,
}

#[derive(Deserialize, Clone)]
struct AmllItem {
    id: u64,
}

#[derive(Deserialize)]
struct AmllGetResponse {
    #[serde(default)]
    data: Option<AmllGetDetail>,
}

#[derive(Deserialize)]
struct AmllGetDetail {
    lyrics: Option<String>,
}

pub async fn fetch_amll_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let search_q = format!("{} {}", title, artist);
    let search_url = format!(
        "https://api.amll.dev/search?keyword={}&type=song",
        urlencoding::encode(&search_q)
    );

    let resp = client
        .get(&search_url)
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err("AMLL search status non-ok".into());
    }

    let search_res: AmllSearchResponse = resp.json().await?;
    let items = search_res
        .data
        .and_then(|d| d.items)
        .ok_or("No AMLL items found")?;

    if items.is_empty() {
        return Err("AMLL search returned empty array".into());
    }

    let first_id = items[0].id;
    let get_url = format!("https://api.amll.dev/get?id={}", first_id);

    let get_resp = client
        .get(&get_url)
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await?;

    if !get_resp.status().is_success() {
        return Err("AMLL get status non-ok".into());
    }

    let get_res: AmllGetResponse = get_resp.json().await?;
    let ttml = get_res
        .data
        .and_then(|d| d.lyrics)
        .ok_or("No TTML lyrics in AMLL response")?;

    if ttml.trim().is_empty() {
        return Err("AMLL returned empty TTML string".into());
    }

    Ok(ttml)
}
