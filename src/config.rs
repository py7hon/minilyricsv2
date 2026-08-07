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
    /// Per-word karaoke animation style. One of:
    /// "pop"   - scale+lift the active word (heaviest, uses a D2D transform)
    /// "wave"  - vertical bounce, no matrix transform (cheap)
    /// "fade"  - crossfades base color -> karaoke color (cheapest with motion)
    /// "sweep" (alias "kf") - ASS/SSA \kf-style left-to-right fill wipe
    /// "glow"  - soft highlighted glow around the active word
    /// "none"  - instant color swap only, no animation at all (lightest possible)
    #[serde(default = "default_karaoke_effect")]
    pub karaoke_effect: String,

    /// Soft drop-shadow behind all text. Approximated with a few
    /// low-alpha offset copies (real Gaussian blur needs a hardware
    /// ID2D1DeviceContext, which this app's DC render target doesn't use).
    #[serde(default)]
    pub shadow_enabled: bool,
    #[serde(default = "default_shadow_hex")]
    pub shadow_hex: String,
    /// 0.0-1.0, base darkness of the shadow before falloff.
    #[serde(default = "default_shadow_opacity")]
    pub shadow_opacity: f32,
    #[serde(default = "default_shadow_offset")]
    pub shadow_offset_x: f32,
    #[serde(default = "default_shadow_offset")]
    pub shadow_offset_y: f32,
    /// How soft/spread out the shadow looks. Higher = softer but a bit
    /// more expensive (more sample copies drawn). Roughly 1.0-6.0 is sane.
    #[serde(default = "default_shadow_blur")]
    pub shadow_blur: f32,
}

fn default_karaoke_effect() -> String {
    "pop".to_string()
}

fn default_shadow_hex() -> String {
    "000000".to_string()
}

fn default_shadow_opacity() -> f32 {
    0.45
}

fn default_shadow_offset() -> f32 {
    1.5
}

fn default_shadow_blur() -> f32 {
    3.0
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
            karaoke_effect: "wave".into(),
            shadow_enabled: false,
            shadow_hex: "000000".into(),
            shadow_opacity: 0.45,
            shadow_offset_x: 1.5,
            shadow_offset_y: 1.5,
            shadow_blur: 3.0,
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

#[allow(dead_code)]
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
