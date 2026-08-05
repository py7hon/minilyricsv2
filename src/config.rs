use serde::{Deserialize, Serialize};
use std::fs;
use windows::Win32::Foundation::COLORREF;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    pub font_family: String,
    pub font_size_active: i32,
    pub font_size_side: i32,
    pub font_size_sub: i32,
    pub font_size_title: i32,
    pub font_size_artist: i32,
    pub line_spacing: f32,
    pub base_center_y: f32,
    pub offset_ms: i64,
    pub opacity: f32,
    pub active_hex: String,
    pub karaoke_hex: String,
    pub side_hex: String,
    pub sub_hex: String,
    pub title_hex: String,
    pub artist_hex: String,
    #[serde(default)]
    pub card_bg_hex: Option<String>,
    #[serde(default)]
    pub show_card: Option<bool>,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            font_family: "Inter".into(),
            font_size_active: 30,
            font_size_side: 14,
            font_size_sub: 12,
            font_size_title: 20,
            font_size_artist: 15,
            line_spacing: 75.0,
            base_center_y: 85.0,
            offset_ms: 0,
            opacity: 1.0,
            active_hex: "ffffff".into(),
            karaoke_hex: "cba6f7".into(),
            side_hex: "cbd5e1".into(),
            sub_hex: "f8fafc".into(),
            title_hex: "ffffff".into(),
            artist_hex: "e2e8f0".into(),
            card_bg_hex: Some("141420".into()),
            show_card: Some(false),
        }
    }
}

pub fn load_or_create_config() -> StyleConfig {
    let path = "config.toml";
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(config) = toml::from_str(&data) {
            return config;
        }
    }
    let default_config = StyleConfig::default();
    if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
        let _ = fs::write(path, toml_str);
    }
    default_config
}

pub fn hex_to_colorref(hex: &str) -> COLORREF {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as u32;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as u32;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as u32;
        COLORREF((b << 16) | (g << 8) | r)
    } else {
        COLORREF(0xFFFFFF)
    }
}
