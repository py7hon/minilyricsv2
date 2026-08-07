use crate::providers::http_debug::http_get_with_debug;
use crate::providers::ttmllib::LyricsResult;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct UnisonResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct UnisonItem {
    #[serde(default)]
    lyrics: Option<String>,
    #[serde(default, rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

fn parse_unison_data(v: &serde_json::Value) -> Option<LyricsResult> {
    let target = if v.is_array() {
        v.as_array()?.first()?
    } else if v.is_object() {
        v
    } else {
        return None;
    };

    let item: UnisonItem = serde_json::from_value(target.clone()).ok()?;
    let synced = item.lyrics.filter(|s| !s.trim().is_empty());
    let plain = item.plain_lyrics.filter(|s| !s.trim().is_empty());

    if synced.is_some() || plain.is_some() {
        Some(LyricsResult { synced, plain })
    } else {
        None
    }
}

pub async fn fetch_unison_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let enc_song = urlencoding::encode(clean_title);
    let enc_artist = urlencoding::encode(artist);

    let mut base_url = format!(
        "https://unison.boidu.dev/lyrics?song={}&artist={}",
        enc_song, enc_artist
    );
    if !album.trim().is_empty() {
        base_url.push_str(&format!("&album={}", urlencoding::encode(album)));
    }
    if let Some(dur) = duration {
        base_url.push_str(&format!("&duration={}", dur));
    }

    let search_url = format!(
        "https://unison.boidu.dev/lyrics/search?song={}&artist={}",
        enc_song, enc_artist
    );

    let urls = [base_url, search_url];

    for url in urls {
        if let Ok(text) = http_get_with_debug(client, &url, "Unison").await {
            if text.trim().starts_with('{') {
                if let Ok(resp) = serde_json::from_str::<UnisonResponse>(&text) {
                    if resp.success {
                        if let Some(data_val) = resp.data {
                            if let Some(res) = parse_unison_data(&data_val) {
                                return Ok(res);
                            }
                        }
                    }
                }
            }
        }
    }

    Err("Unison returned no lyrics".into())
}
