// src/lyrics_api.rs
use crate::dprintln;
use crate::providers::amll::fetch_amll_lyrics;
use crate::providers::betterlyrics::fetch_betterlyrics_lyrics;
use crate::providers::binimum::fetch_binimum_lyrics;
use crate::providers::boidu::fetch_boidu_lyrics;
use crate::providers::lrclib::fetch_lrclib_lyrics;
use crate::providers::lrcmux::fetch_lrcmux_lyrics;
use crate::providers::lyricsplus::fetch_lyricsplus_lyrics;
use crate::providers::musixmatch::fetch_musixmatch_lyrics;
use crate::providers::netease::fetch_netease_lyrics;
use crate::providers::translation::translate_text;
use crate::providers::ttmllib::fetch_ttmllib_lyrics;
#[allow(unused_imports)]
use crate::providers::unison::fetch_unison_lyrics;
use reqwest::Client;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn is_karaoke_or_ttml_format(text: &str) -> bool {
    text.contains("<tt") || text.contains("xml") || (text.contains('<') && text.contains('>'))
}

fn get_response_preview(content: &str) -> String {
    let clean_lines: Vec<&str> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("<?xml") && !l.starts_with("<tt"))
        .collect();

    if clean_lines.is_empty() {
        return content
            .chars()
            .take(80)
            .collect::<String>()
            .replace('\n', " ");
    }

    let sample: Vec<&str> = clean_lines.into_iter().take(2).collect();
    let joined = sample.join(" ║ ");
    let char_count = joined.chars().count();
    if char_count > 100 {
        let truncated: String = joined.chars().take(100).collect();
        format!("{}...", truncated)
    } else {
        joined
    }
}

/// A successful TTML result from any provider (provider name + raw content + optional plain).
struct TtmlHit {
    content: String,
    plain: Option<String>,
    provider: String,
}

/// A successful synced LRC result from any provider.
struct LrcHit {
    content: String,
    plain: Option<String>,
    provider: String,
}

#[derive(Clone)]
pub struct LyricsClient {
    pub reqwest_client: Client,
}

impl LyricsClient {
    pub fn new() -> Self {
        let reqwest_client = Client::builder()
            .timeout(Duration::from_secs(10))
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
    ) -> Result<(String, Option<String>, String), Box<dyn std::error::Error + Send + Sync>> {
        if title.trim().is_empty() {
            return Err("Title is empty".into());
        }

        let overall_start = Instant::now();

        dprintln!(
            "\n┌───────────────────────────────────────────────────────────────────────────────┐"
        );
        dprintln!("│ 🔍 [LYRICS FETCH TASK STARTED]");
        dprintln!("│    Title    : \"{}\"", title);
        dprintln!("│    Artist   : \"{}\"", artist);
        dprintln!(
            "│    Album    : \"{}\"",
            if album.is_empty() { "-" } else { album }
        );
        dprintln!(
            "│    Duration : {:?}",
            duration
                .map(|d| format!("{}s", d))
                .unwrap_or_else(|| "Unknown".into())
        );
        dprintln!(
            "└───────────────────────────────────────────────────────────────────────────────┘"
        );

        dprintln!(
            "\n🚀 [FETCH] Querying all lyrics providers concurrently (early-exit race mode):"
        );

        let client = self.reqwest_client.clone();
        let (tx, mut rx) = mpsc::channel::<(usize, Option<TtmlHit>, Option<LrcHit>)>(11);

        // 0. BetterLyrics
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let album_c = album.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_betterlyrics_lyrics(&client_c, &title_c, &artist_c, &album_c, duration),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) => {
                        let is_ttml = res
                            .synced
                            .as_ref()
                            .is_some_and(|s| is_karaoke_or_ttml_format(s));
                        dprintln!(
                            "  ├─ BetterLyrics {} ✅ ({}ms) | {} bytes",
                            if is_ttml { "TTML" } else { "LRC" },
                            t.elapsed().as_millis(),
                            res.synced.as_ref().map(|s| s.len()).unwrap_or(0)
                        );
                        (
                            is_ttml.then(|| TtmlHit {
                                content: res.synced.clone().unwrap_or_default(),
                                plain: res.plain.clone(),
                                provider: "BetterLyrics (TTML)".into(),
                            }),
                            res.synced.map(|content| LrcHit {
                                content,
                                plain: res.plain,
                                provider: "BetterLyrics".into(),
                            }),
                        )
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ BetterLyrics ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!(
                            "  ├─ BetterLyrics ❌ ({}ms) Timeout",
                            t.elapsed().as_millis()
                        );
                        (None, None)
                    }
                };
                let _ = tx_c.send((0, hit.0, hit.1)).await;
            });
        }

        // 1. Boidu
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let album_c = album.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_boidu_lyrics(&client_c, &title_c, &artist_c, &album_c, duration),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) => {
                        let is_ttml = res
                            .synced
                            .as_ref()
                            .is_some_and(|s| is_karaoke_or_ttml_format(s));
                        dprintln!(
                            "  ├─ Boidu {} ✅ ({}ms) | {} bytes",
                            if is_ttml { "TTML" } else { "LRC" },
                            t.elapsed().as_millis(),
                            res.synced.as_ref().map(|s| s.len()).unwrap_or(0)
                        );
                        (
                            is_ttml.then(|| TtmlHit {
                                content: res.synced.clone().unwrap_or_default(),
                                plain: res.plain.clone(),
                                provider: "Boidu (TTML)".into(),
                            }),
                            res.synced.map(|content| LrcHit {
                                content,
                                plain: res.plain,
                                provider: "Boidu".into(),
                            }),
                        )
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ Boidu ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  ├─ Boidu ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((1, hit.0, hit.1)).await;
            });
        }

        // 2. LyricsPlus
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let album_c = album.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_lyricsplus_lyrics(&client_c, &title_c, &artist_c, &album_c, duration),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) => {
                        let is_ttml = res
                            .synced
                            .as_ref()
                            .is_some_and(|s| is_karaoke_or_ttml_format(s));
                        dprintln!(
                            "  ├─ LyricsPlus {} ✅ ({}ms) | {} bytes",
                            if is_ttml { "TTML" } else { "LRC" },
                            t.elapsed().as_millis(),
                            res.synced.as_ref().map(|s| s.len()).unwrap_or(0)
                        );
                        (
                            is_ttml.then(|| TtmlHit {
                                content: res.synced.clone().unwrap_or_default(),
                                plain: res.plain.clone(),
                                provider: "LyricsPlus (TTML)".into(),
                            }),
                            res.synced.map(|content| LrcHit {
                                content,
                                plain: res.plain,
                                provider: "LyricsPlus".into(),
                            }),
                        )
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ LyricsPlus ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  ├─ LyricsPlus ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((2, hit.0, hit.1)).await;
            });
        }

        // 3. AMLL
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_amll_lyrics(&client_c, &title_c, &artist_c),
                )
                .await;
                let hit = match res {
                    Ok(Ok(amll_ttml)) if is_karaoke_or_ttml_format(&amll_ttml) => {
                        dprintln!(
                            "  ├─ AMLL TTML ✅ ({}ms) | {} bytes",
                            t.elapsed().as_millis(),
                            amll_ttml.len()
                        );
                        (
                            Some(TtmlHit {
                                content: amll_ttml,
                                plain: None,
                                provider: "AMLL".into(),
                            }),
                            None,
                        )
                    }
                    Ok(Ok(_)) => {
                        dprintln!("  ├─ AMLL ❌ ({}ms) non-TTML", t.elapsed().as_millis());
                        (None, None)
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ AMLL ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  ├─ AMLL ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((3, hit.0, hit.1)).await;
            });
        }

        // 4. Musixmatch
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let album_c = album.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_musixmatch_lyrics(&client_c, &title_c, &artist_c, &album_c, duration),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) => {
                        let is_ttml = res
                            .synced
                            .as_ref()
                            .is_some_and(|s| is_karaoke_or_ttml_format(s));
                        dprintln!(
                            "  ├─ Musixmatch {} ✅ ({}ms) | {} bytes",
                            if is_ttml { "TTML" } else { "LRC" },
                            t.elapsed().as_millis(),
                            res.synced.as_ref().map(|s| s.len()).unwrap_or(0)
                        );
                        (
                            is_ttml.then(|| TtmlHit {
                                content: res.synced.clone().unwrap_or_default(),
                                plain: res.plain.clone(),
                                provider: "Musixmatch (TTML)".into(),
                            }),
                            res.synced.map(|content| LrcHit {
                                content,
                                plain: res.plain,
                                provider: "Musixmatch".into(),
                            }),
                        )
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ Musixmatch ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  ├─ Musixmatch ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((4, hit.0, hit.1)).await;
            });
        }

        // 5. Binimum
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let album_c = album.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_binimum_lyrics(&client_c, &title_c, &artist_c, &album_c, duration),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) => {
                        let is_ttml = res
                            .synced
                            .as_ref()
                            .is_some_and(|s| is_karaoke_or_ttml_format(s));
                        dprintln!(
                            "  ├─ Binimum {} ✅ ({}ms) | {} bytes",
                            if is_ttml { "TTML" } else { "LRC" },
                            t.elapsed().as_millis(),
                            res.synced.as_ref().map(|s| s.len()).unwrap_or(0)
                        );
                        (
                            is_ttml.then(|| TtmlHit {
                                content: res.synced.clone().unwrap_or_default(),
                                plain: res.plain.clone(),
                                provider: "Binimum (TTML)".into(),
                            }),
                            res.synced.map(|content| LrcHit {
                                content,
                                plain: res.plain,
                                provider: "Binimum".into(),
                            }),
                        )
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ Binimum ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  ├─ Binimum ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((5, hit.0, hit.1)).await;
            });
        }

        // 6. LRCMux
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let album_c = album.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_lrcmux_lyrics(&client_c, &title_c, &artist_c, &album_c, duration),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) => {
                        let is_ttml = res
                            .synced
                            .as_ref()
                            .is_some_and(|s| is_karaoke_or_ttml_format(s));
                        dprintln!(
                            "  ├─ LRCMux {} ✅ ({}ms) | {} bytes",
                            if is_ttml { "TTML" } else { "LRC" },
                            t.elapsed().as_millis(),
                            res.synced.as_ref().map(|s| s.len()).unwrap_or(0)
                        );
                        (
                            is_ttml.then(|| TtmlHit {
                                content: res.synced.clone().unwrap_or_default(),
                                plain: res.plain.clone(),
                                provider: "LRCMux (TTML)".into(),
                            }),
                            res.synced.map(|content| LrcHit {
                                content,
                                plain: res.plain,
                                provider: "LRCMux".into(),
                            }),
                        )
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ LRCMux ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  ├─ LRCMux ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((6, hit.0, hit.1)).await;
            });
        }

        // 7. Unison
        {
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let _ = tx_c.send((7, None, None)).await;
            });
        }

        // 8. TTMLLIB
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let album_c = album.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_ttmllib_lyrics(&client_c, &title_c, &artist_c, &album_c, duration),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) => {
                        let is_ttml = res
                            .synced
                            .as_ref()
                            .is_some_and(|s| is_karaoke_or_ttml_format(s));
                        dprintln!(
                            "  ├─ TTMLLIB {} ✅ ({}ms) | {} bytes",
                            if is_ttml { "TTML" } else { "LRC" },
                            t.elapsed().as_millis(),
                            res.synced.as_ref().map(|s| s.len()).unwrap_or(0)
                        );
                        (
                            is_ttml.then(|| TtmlHit {
                                content: res.synced.clone().unwrap_or_default(),
                                plain: res.plain.clone(),
                                provider: "TTMLLIB (TTML)".into(),
                            }),
                            res.synced.map(|content| LrcHit {
                                content,
                                plain: res.plain,
                                provider: "TTMLLIB".into(),
                            }),
                        )
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ TTMLLIB ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  ├─ TTMLLIB ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((8, hit.0, hit.1)).await;
            });
        }

        // 9. LRCLIB
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let album_c = album.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_lrclib_lyrics(&client_c, &title_c, &artist_c, &album_c, duration),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) if res.synced.is_some() => {
                        let synced = res.synced.unwrap();
                        dprintln!(
                            "  ├─ LRCLIB LRC ✅ ({}ms) | {} bytes",
                            t.elapsed().as_millis(),
                            synced.len()
                        );
                        (
                            None,
                            Some(LrcHit {
                                content: synced,
                                plain: res.plain,
                                provider: "LRCLIB".into(),
                            }),
                        )
                    }
                    Ok(Ok(_)) => {
                        dprintln!("  ├─ LRCLIB ❌ ({}ms) no synced", t.elapsed().as_millis());
                        (None, None)
                    }
                    Ok(Err(e)) => {
                        dprintln!("  ├─ LRCLIB ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  ├─ LRCLIB ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((9, hit.0, hit.1)).await;
            });
        }

        // 10. NetEase
        {
            let client_c = client.clone();
            let title_c = title.to_string();
            let artist_c = artist.to_string();
            let tx_c = tx.clone();
            tokio::spawn(async move {
                let t = Instant::now();
                let res = tokio::time::timeout(
                    Duration::from_secs(6),
                    fetch_netease_lyrics(&client_c, &title_c, &artist_c),
                )
                .await;
                let hit = match res {
                    Ok(Ok(res)) if res.synced.is_some() => {
                        let synced = res.synced.unwrap();
                        dprintln!(
                            "  └─ NetEase LRC ✅ ({}ms) | {} bytes",
                            t.elapsed().as_millis(),
                            synced.len()
                        );
                        (
                            None,
                            Some(LrcHit {
                                content: synced,
                                plain: res.plain,
                                provider: "NetEase".into(),
                            }),
                        )
                    }
                    Ok(Ok(_)) => {
                        dprintln!("  └─ NetEase ❌ ({}ms) no synced", t.elapsed().as_millis());
                        (None, None)
                    }
                    Ok(Err(e)) => {
                        dprintln!("  └─ NetEase ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                        (None, None)
                    }
                    Err(_) => {
                        dprintln!("  └─ NetEase ❌ ({}ms) Timeout", t.elapsed().as_millis());
                        (None, None)
                    }
                };
                let _ = tx_c.send((10, hit.0, hit.1)).await;
            });
        }

        // Drop local sender handle so receiver closes when all 11 background tasks complete
        drop(tx);

        let mut lrc_hits: HashMap<usize, LrcHit> = HashMap::new();

        while let Some((idx, ttml_opt, lrc_opt)) = rx.recv().await {
            // Eager early-exit: Return immediately on the first valid TTML hit
            if let Some(hit) = ttml_opt {
                dprintln!(
                    "┌───────────────────────────────────────────────────────────────────────────────┐"
                );
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: {} | Total Time: {}ms",
                    hit.provider,
                    overall_start.elapsed().as_millis()
                );
                dprintln!("│    Preview: \"{}\"", get_response_preview(&hit.content));
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((hit.content, hit.plain, hit.provider));
            }

            if let Some(hit) = lrc_opt {
                lrc_hits.entry(idx).or_insert(hit);
            }
        }

        // Fallback: If no TTML hit arrived, return the highest-priority LRC hit
        for idx in 0..11 {
            if let Some(hit) = lrc_hits.remove(&idx) {
                dprintln!(
                    "┌───────────────────────────────────────────────────────────────────────────────┐"
                );
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: {} (Synced LRC) | Total Time: {}ms",
                    hit.provider,
                    overall_start.elapsed().as_millis()
                );
                dprintln!("│    Preview: \"{}\"", get_response_preview(&hit.content));
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((hit.content, hit.plain, hit.provider));
            }
        }

        dprintln!(
            "\n┌───────────────────────────────────────────────────────────────────────────────┐"
        );
        dprintln!(
            "│ ⚠️ [ALL PROVIDERS FAILED] No synced lyrics available | Time: {}ms",
            overall_start.elapsed().as_millis()
        );
        dprintln!(
            "└───────────────────────────────────────────────────────────────────────────────┘\n"
        );

        Err("All lyric providers failed to return synced lyrics".into())
    }

    pub async fn translate_text(&self, text: &str) -> Option<String> {
        translate_text(&self.reqwest_client, text).await
    }
}
