use crate::dprintln;
use crate::providers::amll::fetch_amll_lyrics;
use crate::providers::lrclib::fetch_lrclib_lyrics;
use crate::providers::lrcmux::fetch_lrcmux_lyrics;
use crate::providers::lyricsplus::fetch_lyricsplus_lyrics;
use crate::providers::netease::fetch_netease_lyrics;
use crate::providers::translation::translate_text;
use crate::providers::ttmllib::fetch_ttmllib_lyrics;
use crate::providers::unison::fetch_unison_lyrics;
use reqwest::Client;
use std::time::{Duration, Instant};

fn is_ttml_format(text: &str) -> bool {
    text.contains('<') && text.contains('>')
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

        // ====================================================================
        // PASS 1: TTML Syllable-Level Karaoke Priority Pass (LyricsPlus Primary)
        // ====================================================================
        dprintln!("\n🚀 [PASS 1] Querying Word-by-Word TTML Karaoke Providers:");

        // 1. LyricsPlus API (PRIMARY PROVIDER)
        let t0 = Instant::now();
        dprintln!("  ├─ [1/5] LyricsPlus (lyricsplus.prjktla.my.id) ..... ");
        match fetch_lyricsplus_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.as_ref().is_some_and(|s| is_ttml_format(s)) => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : TTML XML (Word-by-Word Karaoke)");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: LyricsPlus (TTML Karaoke) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "LyricsPlus (TTML)".to_string()));
            }
            Ok(res) => {
                let prev = res.synced.as_deref().unwrap_or("empty");
                dprintln!(
                    "❌ FAILED ({}ms) -> Non-TTML payload: \"{}\"",
                    t0.elapsed().as_millis(),
                    get_response_preview(prev)
                );
            }
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        // 2. AMLL Dev API
        let t0 = Instant::now();
        dprintln!("  ├─ [2/5] AMLL Dev (api.amll.dev) ................... ");
        match fetch_amll_lyrics(&self.reqwest_client, title, artist).await {
            Ok(amll_ttml) if is_ttml_format(&amll_ttml) => {
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&amll_ttml);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, amll_ttml.len());
                dprintln!("  │    ├─ Format  : TTML XML (Word-by-Word Karaoke)");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: AMLL (TTML Karaoke) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((amll_ttml, None, "AMLL".to_string()));
            }
            Ok(res) => {
                let prev = get_response_preview(&res);
                dprintln!(
                    "❌ FAILED ({}ms) -> Non-TTML payload: \"{}\"",
                    t0.elapsed().as_millis(),
                    prev
                );
            }
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        // 3. LRCMux API
        let t0 = Instant::now();
        dprintln!("  ├─ [3/5] LRCMux (api.lrcmux.dev) .................. ");
        match fetch_lrcmux_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.as_ref().is_some_and(|s| is_ttml_format(s)) => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : TTML XML (Word-by-Word Karaoke)");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: LRCMux (TTML Karaoke) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "LRCMux (TTML)".to_string()));
            }
            Ok(res) => {
                let prev = res.synced.as_deref().unwrap_or("empty");
                dprintln!(
                    "❌ FAILED ({}ms) -> Non-TTML payload: \"{}\"",
                    t0.elapsed().as_millis(),
                    get_response_preview(prev)
                );
            }
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        // 4. Unison API
        let t0 = Instant::now();
        dprintln!("  ├─ [4/5] Unison (unison.boidu.dev) ................. ");
        match fetch_unison_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.as_ref().is_some_and(|s| is_ttml_format(s)) => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : TTML XML (Word-by-Word Karaoke)");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: Unison (TTML Karaoke) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "Unison (TTML)".to_string()));
            }
            Ok(res) => {
                let prev = res.synced.as_deref().unwrap_or("empty");
                dprintln!(
                    "❌ FAILED ({}ms) -> Non-TTML payload: \"{}\"",
                    t0.elapsed().as_millis(),
                    get_response_preview(prev)
                );
            }
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        // 5. TTMLLIB API
        let t0 = Instant::now();
        dprintln!("  └─ [5/5] TTMLLIB (ttmllib.xyz) ..................... ");
        match fetch_ttmllib_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.as_ref().is_some_and(|s| is_ttml_format(s)) => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : TTML XML (Word-by-Word Karaoke)");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: TTMLLIB (TTML Karaoke) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "TTMLLIB (TTML)".to_string()));
            }
            Ok(res) => {
                let prev = res.synced.as_deref().unwrap_or("empty");
                dprintln!(
                    "❌ FAILED ({}ms) -> Non-TTML payload: \"{}\"",
                    t0.elapsed().as_millis(),
                    get_response_preview(prev)
                );
            }
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        // ====================================================================
        // PASS 2: Synced LRC Line-Level Fallback Pass (LyricsPlus Primary)
        // ====================================================================
        dprintln!("\n🔄 [PASS 2] Falling Back to Line-Synced LRC Providers:");

        let t0 = Instant::now();
        dprintln!("  ├─ [1/6] LyricsPlus (Synced LRC) .................. ");
        match fetch_lyricsplus_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.is_some() => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : Synced Line LRC");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: LyricsPlus (Synced LRC) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "LyricsPlus".to_string()));
            }
            Ok(_) => dprintln!(
                "❌ FAILED ({}ms) -> No synced LRC data",
                t0.elapsed().as_millis()
            ),
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        let t0 = Instant::now();
        dprintln!("  ├─ [2/6] LRCMux (Synced LRC) ....................... ");
        match fetch_lrcmux_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.is_some() => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : Synced Line LRC");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: LRCMux (Synced LRC) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "LRCMux".to_string()));
            }
            Ok(_) => dprintln!(
                "❌ FAILED ({}ms) -> No synced LRC data",
                t0.elapsed().as_millis()
            ),
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        let t0 = Instant::now();
        dprintln!("  ├─ [3/6] Unison (Synced LRC) ....................... ");
        match fetch_unison_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.is_some() => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : Synced Line LRC");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: Unison (Synced LRC) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "Unison".to_string()));
            }
            Ok(_) => dprintln!(
                "❌ FAILED ({}ms) -> No synced LRC data",
                t0.elapsed().as_millis()
            ),
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        let t0 = Instant::now();
        dprintln!("  ├─ [4/6] TTMLLIB (Synced LRC) ...................... ");
        match fetch_ttmllib_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.is_some() => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : Synced Line LRC");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: TTMLLIB (Synced LRC) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "TTMLLIB".to_string()));
            }
            Ok(_) => dprintln!(
                "❌ FAILED ({}ms) -> No synced LRC data",
                t0.elapsed().as_millis()
            ),
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        let t0 = Instant::now();
        dprintln!("  ├─ [5/6] LRCLIB (lrclib.net) ....................... ");
        match fetch_lrclib_lyrics(&self.reqwest_client, title, artist, album, duration).await {
            Ok(res) if res.synced.is_some() => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : Synced Line LRC");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: LRCLIB (Synced LRC) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "LRCLIB".to_string()));
            }
            Ok(_) => dprintln!(
                "❌ FAILED ({}ms) -> No synced LRC data",
                t0.elapsed().as_millis()
            ),
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        let t0 = Instant::now();
        dprintln!("  └─ [6/6] NetEase Cloud Music (music.163.com) ....... ");
        match fetch_netease_lyrics(&self.reqwest_client, title, artist).await {
            Ok(res) if res.synced.is_some() => {
                let synced = res.synced.unwrap();
                let dt = t0.elapsed().as_millis();
                let prev = get_response_preview(&synced);
                dprintln!("✅ SUCCESS ({}ms) | {} bytes", dt, synced.len());
                dprintln!("  │    ├─ Format  : Synced Line LRC");
                dprintln!("  │    └─ Preview : \"{}\"", prev);
                dprintln!("┌───────────────────────────────────────────────────────────────────────────────┐");
                dprintln!(
                    "│ 🎉 [MATCH FOUND] Provider: NetEase (Synced LRC) | Total Time: {}ms",
                    overall_start.elapsed().as_millis()
                );
                dprintln!("└───────────────────────────────────────────────────────────────────────────────┘\n");
                return Ok((synced, res.plain, "NetEase".to_string()));
            }
            Ok(_) => dprintln!(
                "❌ FAILED ({}ms) -> No synced LRC data",
                t0.elapsed().as_millis()
            ),
            Err(e) => dprintln!(
                "❌ FAILED ({}ms) -> Response: {}",
                t0.elapsed().as_millis(),
                e
            ),
        }

        dprintln!(
            "┌───────────────────────────────────────────────────────────────────────────────┐"
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
