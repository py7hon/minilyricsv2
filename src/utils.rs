// src/utils.rs
#[cfg(windows)]
use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
#[cfg(windows)]
use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};

use regex::Regex;
use std::sync::OnceLock;

/// Ultra-aggressive memory working set trimmer.
/// Uses `EmptyWorkingSet` and `SetProcessWorkingSetSize` to flush all process heap,
/// stack, and D2D pages out of physical RAM into OS standby pool,
/// dropping reported Task Manager Working Set down to ~0.1 - 0.5 MB.
pub fn trim_working_set() {
    #[cfg(windows)]
    unsafe {
        let process_handle = GetCurrentProcess();
        let _ = EmptyWorkingSet(process_handle);
        let _ = SetProcessWorkingSetSize(process_handle, usize::MAX, usize::MAX);
    }
}

static YT_TITLE_RE: OnceLock<Regex> = OnceLock::new();
static YT_TITLE_RE_JP: OnceLock<Regex> = OnceLock::new();
static YT_TITLE_RE_BRACKET: OnceLock<Regex> = OnceLock::new();
static SUFFIX_JUNK_RE: OnceLock<Regex> = OnceLock::new();
static MD_LINK_RE: OnceLock<Regex> = OnceLock::new();

fn yt_title_re() -> &'static Regex {
    YT_TITLE_RE.get_or_init(|| {
        Regex::new(r"(?i)^(.*?)\s*[-–]\s*(.*?)\s*[\(\[](?:official\s*(?:music\s*)?video|official\s*audio|lyrics?(?:\s*video)?|mv|audio|visualizer|hd|4k)[\)\]].*$").unwrap()
    })
}

// Pola Jepang: "Artis / Judul -Music Video-" atau "Artis /「Judul」-MV-"
fn yt_title_re_jp() -> &'static Regex {
    YT_TITLE_RE_JP.get_or_init(|| {
        Regex::new(r"(?i)^(.*?)\s*/\s*(.*?)\s*-\s*(?:music\s*video|mv|lyric\s*video|short\s*ver\.?)\s*-.*$").unwrap()
    })
}

// Pola "Artist｢Title｣" di awal, sering dipakai buat collab/anime tie-in:
// "Creepy Nuts｢Bling-Bang-Bang-Born｣ × TV Anime｢...｣ Collaboration MV"
fn yt_title_re_bracket() -> &'static Regex {
    YT_TITLE_RE_BRACKET.get_or_init(|| Regex::new(r"^(.+?)[｢「](.+?)[｣」]").unwrap())
}

// Nangkep frasa umum "... Video/Audio/MV" di ekor title, apapun kata di
// depannya (Official/Collaboration/Lyric/Teaser/dll) — otomatis, gak perlu
// daftar suffix manual satu-satu.
fn suffix_junk_re() -> &'static Regex {
    SUFFIX_JUNK_RE.get_or_init(|| {
        Regex::new(
            r"(?i)\s*[\|\-–]?\s*\S*\s*(?:official\s+)?\S*\s*(?:music\s+)?(?:video|audio|mv)\b.*$",
        )
        .unwrap()
    })
}

// Markdown link generik: "[label](url)" -> buang seluruhnya
fn md_link_re() -> &'static Regex {
    MD_LINK_RE.get_or_init(|| Regex::new(r"\[[^\]]*\]\([^)]*\)").unwrap())
}

/// Parse title mentah dari YouTube (GSMTC) jadi (artist, title).
/// Dipakai kalau field `artist` dari GSMTC kosong.
pub fn parse_yt_title(raw: &str) -> Option<(String, String)> {
    if let Some(caps) = yt_title_re().captures(raw) {
        return Some((caps[1].trim().to_string(), caps[2].trim().to_string()));
    }
    if let Some(caps) = yt_title_re_jp().captures(raw) {
        let artist = caps[1].trim().to_string();
        let title = caps[2]
            .trim()
            .trim_start_matches('「')
            .trim_end_matches('」')
            .trim()
            .to_string();
        return Some((artist, title));
    }
    if let Some(caps) = yt_title_re_bracket().captures(raw) {
        let artist = caps[1].trim().to_string();
        let title = caps[2].trim().to_string();
        if !artist.is_empty() && !title.is_empty() {
            return Some((artist, title));
        }
    }
    if let Some(idx) = raw.find(" - ").or_else(|| raw.find(" – ")) {
        let (a, b) = raw.split_at(idx);
        let b = b.trim_start_matches(" - ").trim_start_matches(" – ");
        return Some((a.trim().to_string(), b.trim().to_string()));
    }
    if let Some(idx) = raw.find(" / ") {
        let (a, b) = raw.split_at(idx);
        let b = b.trim_start_matches(" / ");
        return Some((a.trim().to_string(), b.trim().to_string()));
    }
    None
}

/// Bersihin title dari markdown link, suffix "...Video/Audio/MV" (otomatis,
/// gak hardcode per-frasa), kurung Jepang, hashtag, dan normalisasi wave
/// dash, sebelum dikirim ke provider lirik.
pub fn clean_song_title(raw: &str) -> String {
    let mut t = raw.to_string();

    t = md_link_re().replace_all(&t, "").to_string();
    t = suffix_junk_re().replace(&t, "").to_string();

    t = t.replace(['『', '』', '「', '」', '｢', '｣'], "");
    t = t.split('(').next().unwrap_or(&t).trim().to_string();

    t = t
        .split_whitespace()
        .filter(|w| !w.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");

    t = t.replace('～', "〜");
    t.trim().to_string()
}

/// Bersihin artist dari embel-embel channel YouTube (nama channel official,
/// bukan artis lagu). Alih-alih dikosongin total, buang bagian yang jelas
/// bukan nama (kurung di depan, kata "channel"/"公式"/"チャンネル"/"VEVO"),
/// biar sisa teksnya masih bisa dipakai buat search.
pub fn clean_artist_name(artist: &str) -> String {
    let mut a = artist.to_string();

    if a.starts_with('(') {
        if let Some(end) = a.find(')') {
            a = a[end + 1..].to_string();
        }
    }

    for junk in [
        "公式チャンネル",
        "オフィシャルチャンネル",
        "Official Channel",
        "チャンネル",
        "VEVO",
        "Channel",
    ] {
        a = a.replace(junk, "");
    }

    a.trim().to_string()
}
