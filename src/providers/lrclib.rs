use crate::providers::http_debug::http_get_with_debug;
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

    let search_q = format!("{} {}", title, artist);
    let search_url = format!(
        "https://lrclib.net/api/search?q={}",
        urlencoding::encode(&search_q)
    );

    // Fire /api/get (exact match) and /api/search (fuzzy fallback)
    // concurrently instead of waiting for /api/get to finish (and often
    // time out) before even starting /api/search.
    let get_fut = http_get_with_debug(client, &url, "LRCLIB");
    let search_fut = http_get_with_debug(client, &search_url, "LRCLIB Search");
    let (get_resp, search_resp) = tokio::join!(get_fut, search_fut);

    if let Ok(text) = get_resp {
        if let Ok(data) = serde_json::from_str::<LrclibResponse>(&text) {
            let synced = data.synced_lyrics.filter(|s| !s.trim().is_empty());
            let plain = data.plain_lyrics.filter(|s| !s.trim().is_empty());
            if synced.is_some() || plain.is_some() {
                return Ok(LyricsResult { synced, plain });
            }
        }
    }

    let search_body = search_resp?;
    let results: Vec<LrclibSearchResult> = serde_json::from_str(&search_body)?;

    let title_lower = title.to_lowercase();
    let artist_lower = artist.to_lowercase();

    for item in results {
        let t_match = item
            .track_name
            .as_ref()
            .is_some_and(|t| t.to_lowercase().contains(&title_lower));
        let a_match = item
            .artist_name
            .as_ref()
            .is_some_and(|a| a.to_lowercase().contains(&artist_lower));

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
