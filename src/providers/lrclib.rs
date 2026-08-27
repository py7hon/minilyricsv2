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
    let artist_vars = crate::utils::get_artist_variations(artist);

    let search_q = format!("{} {}", title, artist);
    let search_url = format!(
        "https://lrclib.net/api/search?q={}",
        urlencoding::encode(&search_q)
    );

    let mut set = tokio::task::JoinSet::new();

    for art in &artist_vars {
        let mut url = format!(
            "https://lrclib.net/api/get?track_name={}&artist_name={}",
            urlencoding::encode(title),
            urlencoding::encode(art)
        );
        if !album.is_empty() {
            url.push_str(&format!("&album_name={}", urlencoding::encode(album)));
        }
        if let Some(dur) = duration {
            url.push_str(&format!("&duration={}", dur));
        }
        let client_c = client.clone();
        set.spawn(async move { http_get_with_debug(&client_c, &url, "LRCLIB").await.ok() });
    }

    let client_c = client.clone();
    let search_fut = async move {
        http_get_with_debug(&client_c, &search_url, "LRCLIB Search")
            .await
            .ok()
    };

    // Check /api/get hits first
    while let Some(joined) = set.join_next().await {
        if let Ok(Some(text)) = joined {
            if let Ok(data) = serde_json::from_str::<LrclibResponse>(&text) {
                let synced = data.synced_lyrics.filter(|s| !s.trim().is_empty());
                let plain = data.plain_lyrics.filter(|s| !s.trim().is_empty());
                if synced.is_some() || plain.is_some() {
                    set.abort_all();
                    return Ok(LyricsResult { synced, plain });
                }
            }
        }
    }

    if let Some(search_body) = search_fut.await {
        if let Ok(results) = serde_json::from_str::<Vec<LrclibSearchResult>>(&search_body) {
            let title_lower = title.to_lowercase();
            let artist_lowers: Vec<String> = artist_vars.iter().map(|a| a.to_lowercase()).collect();

            for item in results {
                let t_match = item
                    .track_name
                    .as_ref()
                    .is_some_and(|t| t.to_lowercase().contains(&title_lower));
                let a_match = item.artist_name.as_ref().is_some_and(|a| {
                    let al = a.to_lowercase();
                    artist_lowers
                        .iter()
                        .any(|art_l| al.contains(art_l) || art_l.contains(&al))
                });

                if t_match || a_match {
                    let synced = item.synced_lyrics.filter(|s| !s.trim().is_empty());
                    let plain = item.plain_lyrics.filter(|s| !s.trim().is_empty());
                    if synced.is_some() || plain.is_some() {
                        return Ok(LyricsResult { synced, plain });
                    }
                }
            }
        }
    }

    Err("No matching lyrics found on LRCLIB".into())
}
