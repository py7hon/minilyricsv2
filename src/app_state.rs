use crate::config::StyleConfig;
use crate::gsmtc::MediaInfo;
use crate::lrc_parser::LrcLine;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use windows::Win32::Graphics::Gdi::{HBITMAP, HDC};

pub struct AppState {
    pub media: MediaInfo,
    pub lyrics_lines: Vec<LrcLine>,
    pub plain_lines: Vec<String>,
    pub current_index: usize,
    pub plain_lyrics: Option<String>,
    pub is_loading: bool,
    pub offset_ms: i64,
    pub is_locked: bool,
    pub float_index: f32,
    pub config: StyleConfig,
    pub last_pos_ms: u64,
    pub last_pos_update: Instant,
}

pub static mut APP_STATE: Option<Arc<Mutex<AppState>>> = None;
pub static mut PAINT_CACHE: Option<(HDC, HBITMAP, i32, i32)> = None;
