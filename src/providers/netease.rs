use crate::providers::http_debug::{http_get_with_debug, http_post_form_with_debug};
use crate::providers::ttmllib::LyricsResult;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct NeteaseSearchResult {
    result: Option<NeteaseResultData>,
}

#[derive(Deserialize)]
struct NeteaseResultData {
    songs: Option<Vec<NeteaseSong>>,
}

#[derive(Deserialize)]
struct NeteaseSong {
    id: u64,
}

#[derive(Deserialize)]
struct NeteaseLyricResult {
    lrc: Option<NeteaseLrcDetail>,
}

#[derive(Deserialize)]
struct NeteaseLrcDetail {
    lyric: Option<String>,
}

pub async fn fetch_netease_lyrics(
    client: &Client,
    title: &str,
    artist: &str,
) -> Result<LyricsResult, Box<dyn std::error::Error + Send + Sync>> {
    let search_q = format!("{} {}", title, artist);
    let search_url = "https://music.163.com/api/search/get/web";

    let params = [
        ("s", search_q.as_str()),
        ("type", "1"),
        ("offset", "0"),
        ("total", "true"),
        ("limit", "1"),
    ];

    let search_body = http_post_form_with_debug(client, search_url, &params, "NetEase").await?;
    let search_res: NeteaseSearchResult = serde_json::from_str(&search_body)?;

    let songs = search_res
        .result
        .and_then(|r| r.songs)
        .ok_or("No songs found on NetEase")?;

    if songs.is_empty() {
        return Err("NetEase search returned 0 items".into());
    }

    let song_id = songs[0].id;
    let lyric_url = format!(
        "https://music.163.com/api/song/lyric?id={}&lv=1&kv=1&tv=-1",
        song_id
    );

    let lyric_body = http_get_with_debug(client, &lyric_url, "NetEase").await?;
    let lyric_res: NeteaseLyricResult = serde_json::from_str(&lyric_body)?;

    let synced = lyric_res
        .lrc
        .and_then(|l| l.lyric)
        .filter(|s| !s.trim().is_empty());

    if synced.is_none() {
        return Err("No synced lyrics in NetEase detail response".into());
    }

    Ok(LyricsResult {
        synced,
        plain: None,
    })
}
