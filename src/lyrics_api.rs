use crate::providers::amll::fetch_amll_lyrics;
use crate::providers::lrclib::fetch_lrclib_lyrics;
use crate::providers::netease::fetch_netease_lyrics;
use crate::providers::translation::translate_text;
use crate::providers::ttmllib::{fetch_ttmllib_lyrics, LyricsResult};
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct LyricsClient {
    pub reqwest_client: Client,
}

impl LyricsClient {
    pub fn new() -> Self {
        let reqwest_client = Client::builder()
            .timeout(Duration::from_secs(4))
            .build()
            .unwrap_or_default();
        Self { reqwest_client }
    }

    pub async fn fetch_lyrics(
        &self,
        title: &str,
        artist: &str,
        album: &str,
        duration: Option<u64>,
    ) -> Result<(String, Option<String>), Box<dyn std::error::Error + Send + Sync>> {
        if let Ok(amll_ttml) = fetch_amll_lyrics(&self.reqwest_client, title, artist).await {
            println!("[AMLL] Successfully fetched primary TTML lyrics!");
            return Ok((amll_ttml, None));
        }

        if let Ok(res) = fetch_ttmllib_lyrics(&self.reqwest_client, title, artist, duration).await {
            if let Some(synced) = res.synced {
                return Ok((synced, res.plain));
            }
        }

        if let Ok(res) = fetch_lrclib_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            if let Some(synced) = res.synced {
                return Ok((synced, res.plain));
            }
        }

        if let Ok(res) = fetch_netease_lyrics(&self.reqwest_client, title, artist).await {
            if let Some(synced) = res.synced {
                return Ok((synced, res.plain));
            }
        }

        Err("All lyric providers failed to return synced lyrics".into())
    }

    pub async fn translate_text(&self, text: &str) -> Option<String> {
        translate_text(&self.reqwest_client, text).await
    }
}