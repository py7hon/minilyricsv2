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
    #[serde(default = "default_karaoke_v2_hex")]
    pub karaoke_v2_hex: String,
    #[serde(default = "default_karaoke_group_hex")]
    pub karaoke_group_hex: String,
    pub side_hex: String,
    pub sub_hex: String,
    pub title_hex: String,
    pub artist_hex: String,
    #[serde(default)]
    pub card_bg_hex: Option<String>,
    #[serde(default)]
    pub show_card: Option<bool>,

    /// Karaoke display mode: "auto", "always", or "never"
    #[serde(default = "default_karaoke_mode")]
    pub karaoke_mode: String,

    /// "star_bounce" (alias "star", "ball") - Bouncing star/ball indicator overhead above active word
    /// "pop"       - scale+lift the active word (heaviest, uses a D2D transform)
    /// "pulse"     - breathing scale in/out, no lift (uses a D2D transform)
    /// "zoom"      - smooth zoom expansion peak then settles
    /// "wave"      - vertical bounce wave curve
    /// "bounce"    - playful spring drop bounce from above
    /// "slide"     - slides in smoothly from the left
    /// "rise"      - slides up into place from below while fading to color
    /// "tilt"      - playful rotation tilt angle while sung
    /// "stretch"   - cartoony squish & stretch elastic effect
    /// "shake"     - horizontal jitter that settles as the syllable finishes
    /// "shimmer"   - bright flash of light fading to highlight color
    /// "neon"      - vibrant rainbow color spectrum shift
    /// "float"     - floating vertical hover wave
    /// "fade"      - crossfades base color -> karaoke color
    /// "sweep"     (alias "kf") - ASS/SSA \kf-style left-to-right fill wipe
    /// "glow"      - soft highlighted glow around the active word
    /// "none"      - instant color swap only, no animation at all
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

    /// Enable/disable karaoke animation on sub-text lines (Romaji, Romaja, Pinyin, translation)
    #[serde(default)]
    pub sub_karaoke_enabled: Option<bool>,

    /// Karaoke effect for sub-text: "fade" (default), "sweep", "pulse", "wave", "star_bounce", "none", or "auto"
    #[serde(default = "default_sub_karaoke_effect")]
    pub sub_karaoke_effect: Option<String>,

    /// Optional active highlight color hex for sub-text karaoke (defaults to karaoke_hex if None)
    #[serde(default)]
    pub sub_active_hex: Option<String>,

    /// Text horizontal alignment: "center" (default), "duet", "left", or "right"
    #[serde(default = "default_alignment")]
    pub alignment: String,

    /// Selected theme preset name, e.g. "catppuccin_mocha", "cute_kawaii", "tokyo_night", "dracula", "nord", "rose_pine", "cyberpunk", "gruvbox", or "custom"
    #[serde(default)]
    pub theme: Option<String>,

    /// Automatically trim Working Set memory on song change, pause, and startup
    #[serde(default = "default_auto_trim_memory")]
    pub auto_trim_memory: bool,
    /// Periodic working set memory trim interval in seconds (0 to disable)
    #[serde(default = "default_trim_interval_secs")]
    pub trim_interval_secs: u64,
    /// Automatically check for software updates on startup and periodically
    #[serde(default = "default_auto_check_updates")]
    pub auto_check_updates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePreset {
    pub name: &'static str,
    pub label: &'static str,
    pub active_hex: &'static str,
    pub karaoke_hex: &'static str,
    pub karaoke_v2_hex: &'static str,
    pub karaoke_group_hex: &'static str,
    pub side_hex: &'static str,
    pub sub_hex: &'static str,
    pub card_bg_hex: &'static str,
}

pub const THEME_PRESETS: &[ThemePreset] = &[
    ThemePreset {
        name: "custom",
        label: "Custom",
        active_hex: "ffffff",
        karaoke_hex: "cba6f7",
        karaoke_v2_hex: "f38ba8",
        karaoke_group_hex: "89b4fa",
        side_hex: "cbd5e1",
        sub_hex: "f8fafc",
        card_bg_hex: "141420",
    },
    ThemePreset {
        name: "catppuccin_mocha",
        label: "Catppuccin Mocha",
        active_hex: "cdd6f4",
        karaoke_hex: "cba6f7",
        karaoke_v2_hex: "f38ba8",
        karaoke_group_hex: "89b4fa",
        side_hex: "a6adc8",
        sub_hex: "bac2de",
        card_bg_hex: "1e1e2e",
    },
    ThemePreset {
        name: "catppuccin_macchiato",
        label: "Catppuccin Macchiato",
        active_hex: "cad3f5",
        karaoke_hex: "c6a0f6",
        karaoke_v2_hex: "f5bde6",
        karaoke_group_hex: "8aadf4",
        side_hex: "a5adcb",
        sub_hex: "b8c0e0",
        card_bg_hex: "24273a",
    },
    ThemePreset {
        name: "cute_kawaii",
        label: "Cute Kawaii Pastel",
        active_hex: "ffffff",
        karaoke_hex: "ffb7b2",
        karaoke_v2_hex: "ffdac1",
        karaoke_group_hex: "e2f0cb",
        side_hex: "e0c3fc",
        sub_hex: "8eecf5",
        card_bg_hex: "2b1e2a",
    },
    ThemePreset {
        name: "tokyo_night",
        label: "Tokyo Night",
        active_hex: "c0caf5",
        karaoke_hex: "bb9af7",
        karaoke_v2_hex: "f7768e",
        karaoke_group_hex: "7aa2f7",
        side_hex: "a9b1d6",
        sub_hex: "7dcfff",
        card_bg_hex: "1a1b26",
    },
    ThemePreset {
        name: "dracula",
        label: "Dracula",
        active_hex: "f8f8f2",
        karaoke_hex: "bd93f9",
        karaoke_v2_hex: "ff79c6",
        karaoke_group_hex: "8be9fd",
        side_hex: "6272a4",
        sub_hex: "f1fa8c",
        card_bg_hex: "282a36",
    },
    ThemePreset {
        name: "nord",
        label: "Nord Arctic",
        active_hex: "eceff4",
        karaoke_hex: "b48ead",
        karaoke_v2_hex: "ebcb8b",
        karaoke_group_hex: "88c0d0",
        side_hex: "4c566a",
        sub_hex: "81a1c1",
        card_bg_hex: "2e3440",
    },
    ThemePreset {
        name: "rose_pine",
        label: "Rosé Pine",
        active_hex: "e0def4",
        karaoke_hex: "c4a7e7",
        karaoke_v2_hex: "eb6f92",
        karaoke_group_hex: "f6c177",
        side_hex: "908caa",
        sub_hex: "ebbcba",
        card_bg_hex: "191724",
    },
    ThemePreset {
        name: "cyberpunk",
        label: "Cyberpunk 2077",
        active_hex: "ffffff",
        karaoke_hex: "ff0055",
        karaoke_v2_hex: "ffe600",
        karaoke_group_hex: "00f0ff",
        side_hex: "b900ff",
        sub_hex: "00ff99",
        card_bg_hex: "0d0f18",
    },
    ThemePreset {
        name: "gruvbox",
        label: "Gruvbox Dark",
        active_hex: "fbf1c7",
        karaoke_hex: "d3869b",
        karaoke_v2_hex: "fabd2f",
        karaoke_group_hex: "83a598",
        side_hex: "a89984",
        sub_hex: "8ec07c",
        card_bg_hex: "282828",
    },
];

pub fn get_theme_preset(name: &str) -> Option<&'static ThemePreset> {
    let lower = name.trim().to_lowercase();
    THEME_PRESETS
        .iter()
        .find(|t| t.name == lower || t.label.to_lowercase() == lower)
}

fn default_alignment() -> String {
    "auto".to_string()
}

fn default_sub_karaoke_effect() -> Option<String> {
    Some("fade".to_string())
}

fn default_auto_trim_memory() -> bool {
    true
}

fn default_auto_check_updates() -> bool {
    true
}

fn default_trim_interval_secs() -> u64 {
    5
}

fn default_karaoke_v2_hex() -> String {
    "f38ba8".to_string()
}

fn default_karaoke_group_hex() -> String {
    "89b4fa".to_string()
}

fn default_karaoke_mode() -> String {
    "auto".to_string()
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
            font_family: "Segoe UI".into(),
            font_size_active: 28,
            font_size_side: 18,
            font_size_sub: 14,
            font_size_title: 20,
            font_size_artist: 15,
            line_spacing: 75.0,
            base_center_y: 85.0,
            offset_ms: 0,
            opacity: 1.0,
            active_hex: "ffffff".into(),
            karaoke_hex: "cba6f7".into(),
            karaoke_v2_hex: "f38ba8".into(),
            karaoke_group_hex: "89b4fa".into(),
            side_hex: "cbd5e1".into(),
            sub_hex: "f8fafc".into(),
            title_hex: "ffffff".into(),
            artist_hex: "e2e8f0".into(),
            card_bg_hex: Some("141420".into()),
            show_card: Some(false),
            karaoke_mode: "auto".into(),
            karaoke_effect: "wave".into(),
            alignment: "auto".into(),
            theme: Some("catppuccin_mocha".into()),
            sub_karaoke_enabled: Some(true),
            sub_karaoke_effect: Some("fade".into()),
            sub_active_hex: None,
            shadow_enabled: true,
            shadow_hex: "000000".into(),
            shadow_opacity: 0.45,
            shadow_offset_x: 1.5,
            shadow_offset_y: 1.5,
            shadow_blur: 3.0,
            auto_trim_memory: true,
            trim_interval_secs: 5,
            auto_check_updates: true,
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
    save_config(&default_config);
    default_config
}
pub fn save_config(config: &StyleConfig) {
    if let Ok(toml_str) = toml::to_string_pretty(config) {
        let _ = fs::write("config.toml", toml_str);
    }
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
