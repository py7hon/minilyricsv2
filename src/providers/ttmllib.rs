use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct TtmllibSearchResult {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(rename = "lyricsTtml")]
    lyrics_ttml: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LyricsResult {
    pub synced: Option<String>,
    pub plain: Option<String>,
}

pub async fn fetch_ttmllib_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let mut url = format!(
        "https://ttmllib.xyz/api/get?track_name={}&artist_name={}",
        urlencoding::encode(title),
        urlencoding::encode(artist)
    );
    if let Some(dur) = duration {
        url.push_str(&format!("&duration={}", dur));
    }

    let resp = client
        .get(&url)
        .header("User-Agent", "MiniLyric/2.0")
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err("TTMLLIB returned non-ok status".into());
    }

    let res: TtmllibSearchResult = resp.json().await?;

    let synced = res
        .lyrics_ttml
        .or(res.synced_lyrics)
        .filter(|s| !s.trim().is_empty());
    let plain = res.plain_lyrics.filter(|s| !s.trim().is_empty());

    if synced.is_none() && plain.is_none() {
        return Err("No lyrics found in TTMLLIB response".into());
    }

    Ok(LyricsResult { synced, plain })
}
