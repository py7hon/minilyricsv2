use crate::providers::ttmllib::LyricsResult;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct LrclibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

#[derive(Deserialize)]
struct LrclibSearchResult {
    #[serde(rename = "trackName")]
    track_name: Option<String>,
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

pub async fn fetch_lrclib_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let mut url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        urlencoding::encode(title),
        urlencoding::encode(artist)
    );
    if !album.is_empty() {
        url.push_str(&format!("&album_name={}", urlencoding::encode(album)));
    }
    if let Some(dur) = duration {
        url.push_str(&format!("&duration={}", dur));
    }

    let res = client
        .get(&url)
        .header("User-Agent", "MiniLyric/2.0")
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await;

    if let Ok(resp) = res {
        if resp.status().is_success() {
            if let Ok(data) = resp.json::<LrclibResponse>().await {
                let synced = data.synced_lyrics.filter(|s| !s.trim().is_empty());
                let plain = data.plain_lyrics.filter(|s| !s.trim().is_empty());
                if synced.is_some() || plain.is_some() {
                    return Ok(LyricsResult { synced, plain });
                }
            }
        }
    }

    let search_q = format!("{} {}", title, artist);
    let search_url = format!(
        "https://lrclib.net/api/search?q={}",
        urlencoding::encode(&search_q)
    );

    let search_resp = client
        .get(&search_url)
        .header("User-Agent", "MiniLyric/2.0")
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await?;

    if !search_resp.status().is_success() {
        return Err("LRCLIB search non-ok status".into());
    }

    let results: Vec<LrclibSearchResult> = search_resp.json().await?;
    let title_lower = title.to_lowercase();
    let artist_lower = artist.to_lowercase();

    for item in results {
        let t_match = item
            .track_name
            .as_ref()
            .map_or(false, |t| t.to_lowercase().contains(&title_lower));
        let a_match = item
            .artist_name
            .as_ref()
            .map_or(false, |a| a.to_lowercase().contains(&artist_lower));

        if t_match || a_match {
            let synced = item.synced_lyrics.filter(|s| !s.trim().is_empty());
            let plain = item.plain_lyrics.filter(|s| !s.trim().is_empty());
            if synced.is_some() || plain.is_some() {
                return Ok(LyricsResult { synced, plain });
            }
        }
    }

    Err("No matching lyrics found on LRCLIB".into())
}
