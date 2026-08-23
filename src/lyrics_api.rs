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
use std::time::{Duration, Instant};

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

        dprintln!("\n🚀 [FETCH] Querying all lyrics providers concurrently (single round):");

        let client = &self.reqwest_client;

        let betterlyrics_fut = async {
            let t = Instant::now();
            match fetch_betterlyrics_lyrics(client, title, artist, album, duration).await {
                Ok(res) => {
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
                        res.synced.clone().map(|content| LrcHit {
                            content,
                            plain: res.plain.clone(),
                            provider: "BetterLyrics".into(),
                        }),
                    )
                }
                Err(e) => {
                    dprintln!("  ├─ BetterLyrics ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    (None, None)
                }
            }
        };

        let boidu_fut = async {
            let t = Instant::now();
            match fetch_boidu_lyrics(client, title, artist, album, duration).await {
                Ok(res) => {
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
                        res.synced.clone().map(|content| LrcHit {
                            content,
                            plain: res.plain.clone(),
                            provider: "Boidu".into(),
                        }),
                    )
                }
                Err(e) => {
                    dprintln!("  ├─ Boidu ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    (None, None)
                }
            }
        };

        let lyricsplus_fut = async {
            let t = Instant::now();
            match fetch_lyricsplus_lyrics(client, title, artist, album, duration).await {
                Ok(res) => {
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
                        res.synced.clone().map(|content| LrcHit {
                            content,
                            plain: res.plain.clone(),
                            provider: "LyricsPlus".into(),
                        }),
                    )
                }
                Err(e) => {
                    dprintln!("  ├─ LyricsPlus ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    (None, None)
                }
            }
        };

        let amll_fut = async {
            let t = Instant::now();
            match fetch_amll_lyrics(client, title, artist).await {
                Ok(amll_ttml) if is_karaoke_or_ttml_format(&amll_ttml) => {
                    dprintln!(
                        "  ├─ AMLL TTML ✅ ({}ms) | {} bytes",
                        t.elapsed().as_millis(),
                        amll_ttml.len()
                    );
                    Some(TtmlHit {
                        content: amll_ttml,
                        plain: None,
                        provider: "AMLL".into(),
                    })
                }
                Ok(_) => {
                    dprintln!("  ├─ AMLL ❌ ({}ms) non-TTML", t.elapsed().as_millis());
                    None
                }
                Err(e) => {
                    dprintln!("  ├─ AMLL ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    None
                }
            }
        };

        let lrcmux_fut = async {
            let t = Instant::now();
            match fetch_lrcmux_lyrics(client, title, artist, album, duration).await {
                Ok(res) => {
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
                        res.synced.clone().map(|content| LrcHit {
                            content,
                            plain: res.plain.clone(),
                            provider: "LRCMux".into(),
                        }),
                    )
                }
                Err(e) => {
                    dprintln!("  ├─ LRCMux ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    (None, None)
                }
            }
        };

        let unison_fut = async { (None, None) };

        let ttmllib_fut = async {
            let t = Instant::now();
            match fetch_ttmllib_lyrics(client, title, artist, album, duration).await {
                Ok(res) => {
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
                        res.synced.clone().map(|content| LrcHit {
                            content,
                            plain: res.plain.clone(),
                            provider: "TTMLLIB".into(),
                        }),
                    )
                }
                Err(e) => {
                    dprintln!("  ├─ TTMLLIB ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    (None, None)
                }
            }
        };

        let musixmatch_fut = async {
            let t = Instant::now();
            match fetch_musixmatch_lyrics(client, title, artist, album, duration).await {
                Ok(res) => {
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
                        res.synced.clone().map(|content| LrcHit {
                            content,
                            plain: res.plain.clone(),
                            provider: "Musixmatch".into(),
                        }),
                    )
                }
                Err(e) => {
                    dprintln!("  ├─ Musixmatch ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    (None, None)
                }
            }
        };

        let binimum_fut = async {
            let t = Instant::now();
            match fetch_binimum_lyrics(client, title, artist, album, duration).await {
                Ok(res) => {
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
                        res.synced.clone().map(|content| LrcHit {
                            content,
                            plain: res.plain.clone(),
                            provider: "Binimum".into(),
                        }),
                    )
                }
                Err(e) => {
                    dprintln!("  ├─ Binimum ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    (None, None)
                }
            }
        };

        let lrclib_fut = async {
            let t = Instant::now();
            match fetch_lrclib_lyrics(client, title, artist, album, duration).await {
                Ok(res) if res.synced.is_some() => {
                    let synced = res.synced.unwrap();
                    dprintln!(
                        "  ├─ LRCLIB LRC ✅ ({}ms) | {} bytes",
                        t.elapsed().as_millis(),
                        synced.len()
                    );
                    Some(LrcHit {
                        content: synced,
                        plain: res.plain,
                        provider: "LRCLIB".into(),
                    })
                }
                Ok(_) => {
                    dprintln!("  ├─ LRCLIB ❌ ({}ms) no synced", t.elapsed().as_millis());
                    None
                }
                Err(e) => {
                    dprintln!("  ├─ LRCLIB ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    None
                }
            }
        };

        let netease_fut = async {
            let t = Instant::now();
            match fetch_netease_lyrics(client, title, artist).await {
                Ok(res) if res.synced.is_some() => {
                    let synced = res.synced.unwrap();
                    dprintln!(
                        "  └─ NetEase LRC ✅ ({}ms) | {} bytes",
                        t.elapsed().as_millis(),
                        synced.len()
                    );
                    Some(LrcHit {
                        content: synced,
                        plain: res.plain,
                        provider: "NetEase".into(),
                    })
                }
                Ok(_) => {
                    dprintln!("  └─ NetEase ❌ ({}ms) no synced", t.elapsed().as_millis());
                    None
                }
                Err(e) => {
                    dprintln!("  └─ NetEase ❌ ({}ms) {}", t.elapsed().as_millis(), e);
                    None
                }
            }
        };

        let (
            bl_res,
            boidu_res,
            lp_res,
            amll_res,
            mx_res,
            bini_res,
            lm_res,
            un_res,
            tt_res,
            lr_res,
            ne_res,
        ) = tokio::join!(
            betterlyrics_fut,
            boidu_fut,
            lyricsplus_fut,
            amll_fut,
            musixmatch_fut,
            binimum_fut,
            lrcmux_fut,
            unison_fut,
            ttmllib_fut,
            lrclib_fut,
            netease_fut
        );

        let (bl_ttml, bl_lrc) = bl_res;
        let (boidu_ttml, boidu_lrc) = boidu_res;
        let (lp_ttml, lp_lrc) = lp_res;
        let (mx_ttml, mx_lrc) = mx_res;
        let (bini_ttml, bini_lrc) = bini_res;
        let (lm_ttml, lm_lrc) = lm_res;
        let (un_ttml, un_lrc) = un_res;
        let (tt_ttml, tt_lrc) = tt_res;

        let ttml_result = bl_ttml
            .or(boidu_ttml)
            .or(lp_ttml)
            .or(amll_res)
            .or(mx_ttml)
            .or(bini_ttml)
            .or(lm_ttml)
            .or(un_ttml)
            .or(tt_ttml);

        if let Some(hit) = ttml_result {
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

        let lrc_result = bl_lrc
            .or(boidu_lrc)
            .or(lp_lrc)
            .or(mx_lrc)
            .or(bini_lrc)
            .or(lm_lrc)
            .or(un_lrc)
            .or(tt_lrc)
            .or(lr_res)
            .or(ne_res);

        if let Some(hit) = lrc_result {
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
