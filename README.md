# MiniLyrics V2 🎵✨

A lightweight, modern, transparent floating lyrics overlay for Windows with real-time word-by-word Karaoke animations, multi-provider TTML/LRC support, Romaji/Romaja/Pinyin transliterations, and automatic background updates.

![CI](https://github.com/py7hon/minilyricsv2/actions/workflows/release.yml/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows)
![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)

---

## 🌟 Features

- 💎 **Transparent Floating Overlay**: Hardware-accelerated Direct2D / DirectWrite borderless overlay that floats over any application or media player (YouTube Music, Spotify, Apple Music, Chrome, Edge, etc.).
- ⚡ **Zero-Delay GSMTC Sync**: Real-time position and playback state tracking via Windows System Media Transport Controls.
- 🎤 **Word-by-Word Karaoke Animations**: Real-time syllable pop-scaling, wave animations, and ASS-style bouncing star indicators with particle effects.
- 🌐 **Automatic Transliteration & Translations**:
  - Automatically fetches **Romaji** (Japanese), **Romaja** (Korean), or **Pinyin** (Chinese) via Google Translate when missing from providers.
  - Native dual-line merger for main text + sub-text transliterations.
  - Overrides sentence-based translations with clean phonetic transliterations for AMLL provider lyrics.
- 🔄 **Built-in Auto Updater & Installer**:
  - Automatically checks for updates on startup or on demand via system tray.
  - One-click silent update downloader and installer via GitHub Releases API.
  - Easy setup wizard generated with Inno Setup.
- 🥇 **Multi-Source Provider Pipeline**:
  - Primary: **LyricsPlus** ➔ **Better Lyrics API** (`lyrics.pyoi.eu.org`) ➔ **AMLL Dev** ➔ **LRCMux** ➔ **Unison** ➔ **TTMLLIB**.
  - Fallback: **LRCLIB** ➔ **NetEase**.
- ⏱️ **Fine Sync Controls**: Precise `+100ms` / `-100ms` offset adjustments right from the system tray menu.
- 🍃 **Minimal Memory Footprint**: Low memory working set trimming (~0.1 MB – 1 MB RAM) and 0.0% idle GPU/CPU usage when music is paused.
- 📌 **Click-Through Lock Mode**: Lock position (`WS_EX_TRANSPARENT`) for uninterrupted click-through workflow.

---

## 💾 Installation & Setup

### Download Pre-built Installer
Download the latest `MiniLyricsV2_Setup.exe` from [GitHub Releases](https://github.com/py7hon/minilyricsv2/releases/latest).

### Building from Source

**Prerequisites**: Windows 10/11 & Rust 1.75+

```powershell
# Clone repository
git clone https://github.com/py7hon/minilyricsv2.git
cd minilyricsv2

# Run cargo test
cargo test

# Build release binary
cargo build --release

# Build Windows Installer (requires Inno Setup 6 / ISCC.exe)
.\build_installer.ps1
```

The compiled release executable will be saved in `target/release/minilyricv2.exe` and the setup executable in `dist/MiniLyricsV2_Setup.exe`.

---

## 🎛️ Controls & Shortcuts

| Action | How to Use |
| :--- | :--- |
| **Move Overlay** | Click and drag the title/artist header bar (when unlocked). |
| **Toggle Lock / Click-Through** | Click the 🔒 icon in top-right or right-click tray icon ➔ **Lock/Unlock Position**. |
| **Tray Context Menu** | Right-click the tray icon in the Windows taskbar. |
| **Check for Updates** | Right-click tray icon ➔ **Check for Updates...** |
| **Adjust Offset** | Right-click tray icon ➔ **Sync: Faster (+100ms)** / **Slower (-100ms)**. |

---

## ⚙️ Configuration (`config.toml`)

`config.toml` is created automatically on first launch in the app directory:

```toml
# Typography & Scaling
font_family = "Inter"
font_size_active = 30
font_size_side = 14
font_size_sub = 12
font_size_title = 20
font_size_artist = 15
line_spacing = 75.0
base_center_y = 85.0

# Sync Offset & Transparency
offset_ms = 0
opacity = 1.0

# Colors (HEX)
active_hex = "ffffff"
karaoke_hex = "cba6f7"
side_hex = "cbd5e1"
sub_hex = "f8fafc"
title_hex = "ffffff"
artist_hex = "e2e8f0"
card_bg_hex = "141420"
show_card = false

# Karaoke Animation Mode
# Options: "star_bounce", "zoom", "pulse", "wave", "bounce", "slide", "rise", "tilt", "stretch", "shake", "shimmer", "neon", "float", "pop", "fade", "sweep", "glow", "none"
karaoke_effect = "star_bounce"

# Memory Optimization
auto_trim_memory = true
trim_interval_secs = 5
```

---

## 📄 License

MIT License. See `LICENSE` for details.
