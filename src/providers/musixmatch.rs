use crate::providers::http_debug::http_get_with_debug;
use reqwest::Client;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct LyricsResult {
    pub synced: Option<String>,
    pub plain: Option<String>,
}

pub async fn fetch_musixmatch_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let dur_val = duration.unwrap_or(0);
    let artist_vars = crate::utils::get_artist_variations(artist);

    let mut set = tokio::task::JoinSet::new();

    for art in &artist_vars {
        let url = format!(
            "https://apic-desktop.musixmatch.com/ws/1.1/macro.subtitles.get?format=json&q_track={}&q_artist={}&q_album={}&user_language=en&f_subtitle_length={}&app_id=web-desktop-app-v1.0",
            urlencoding::encode(clean_title),
            urlencoding::encode(art),
            urlencoding::encode(album),
            dur_val
        );
        let client_c = client.clone();
        set.spawn(async move {
            http_get_with_debug(&client_c, &url, "Musixmatch")
                .await
                .ok()
        });
    }

    while let Some(joined) = set.join_next().await {
        if let Ok(Some(body)) = joined {
            if let Ok(val) = serde_json::from_str::<Value>(&body) {
                if let Some(message) = val.get("message").and_then(|m| m.get("body")) {
                    if let Some(macro_calls) = message.get("macro_calls") {
                        if let Some(sub_get) = macro_calls.get("track.subtitles.get") {
                            if let Some(sub_body) =
                                sub_get.get("message").and_then(|m| m.get("body"))
                            {
                                if let Some(subtitle_list) =
                                    sub_body.get("subtitle_list").and_then(|s| s.as_array())
                                {
                                    if let Some(first_sub) = subtitle_list.first() {
                                        if let Some(subtitle) = first_sub.get("subtitle") {
                                            if let Some(lrc) = subtitle
                                                .get("subtitle_body")
                                                .and_then(|v| v.as_str())
                                            {
                                                if !lrc.trim().is_empty() {
                                                    set.abort_all();
                                                    return Ok(LyricsResult {
                                                        synced: Some(lrc.to_string()),
                                                        plain: None,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err("Musixmatch returned no lyrics".into())
}
