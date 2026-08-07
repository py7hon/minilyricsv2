use crate::providers::http_debug::http_get_with_debug;
use crate::providers::ttmllib::LyricsResult;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct LrcMuxResponse {
    #[serde(default, rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(default, rename = "lyricsTtml")]
    lyrics_ttml: Option<String>,
    #[serde(default)]
    ttml: Option<String>,
    #[serde(default)]
    lyrics: Option<String>,
    #[serde(default, rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

pub async fn fetch_lrcmux_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: Option<u64>,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let clean_title = title.split('(').next().unwrap_or(title).trim();
    let enc_title = urlencoding::encode(clean_title);
    let enc_artist = urlencoding::encode(artist);
    let enc_album = urlencoding::encode(album);
    let dur_val = duration.unwrap_or(0);

    let urls = [
        format!("https://api.lrcmux.dev/get?title={}&artist={}&album={}&duration={}", enc_title, enc_artist, enc_album, dur_val),
        format!("https://api.lrcmux.dev/compat/kpoe/v2/lyrics/get?title={}&artist={}", enc_title, enc_artist),
        format!("https://api.lrcmux.dev/compat/lrclib/api/get?track_name={}&artist_name={}&album_name={}&duration={}", enc_title, enc_artist, enc_album, dur_val),
        format!("https://api.lrcmux.dev/get?title={}&artist={}", enc_title, enc_artist),
    ];

    for url in urls {
        if let Ok(text) = http_get_with_debug(client, &url, "LRCMux").await {
            if text.trim().starts_with('{') {
                if let Ok(res) = serde_json::from_str::<LrcMuxResponse>(&text) {
                    let synced = res
                        .lyrics_ttml
                        .or(res.ttml)
                        .or(res.synced_lyrics)
                        .or(res.lyrics)
                        .filter(|s| !s.trim().is_empty());
                    let plain = res.plain_lyrics.filter(|s| !s.trim().is_empty());

                    if synced.is_some() || plain.is_some() {
                        return Ok(LyricsResult { synced, plain });
                    }
                }
            }
        }
    }

    Err("LRCMux returned non-ok status or no lyrics".into())
}
