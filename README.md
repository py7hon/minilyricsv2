# Mini Lyric v2 🎵✨

> **A modern, lightweight, transparent floating lyrics overlay for Windows.**  
> Powered by Rust, Direct2D/DirectWrite, Windows System Media Transport Controls (GSMTC), and multi-source TTML/LRC providers.

![CI](https://github.com/py7hon/minilyricsv2/actions/workflows/ci.yml/badge.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows)
![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)

<img width="1680" height="894" alt="image" src="https://github.com/user-attachments/assets/df010561-35db-4487-bf44-524d91241e5c" />

[![Alt text](https://github.com/user-attachments/assets/31968c2b-b999-4cd6-8f8b-6fad8bfbd198)](https://github.com/user-attachments/assets/31968c2b-b999-4cd6-8f8b-6fad8bfbd198)




---

## 🌟 Key Features

- 💎 **Transparent Floating Overlay**: Sleek, borderless, glasslike Direct2D overlay floating over any application or media player (YouTube Music, Spotify, Apple Music, Chrome, Edge, etc.).
- ⚡ **Instant 0ms GSMTC Media Polling**: Instantaneous zero-delay playback state & position monitoring via Windows System Media Transport Controls.
- 🎤 **Native TTML XML & Word-by-Word Karaoke**: Real-time syllable pop-scaling, wave animations, and smooth line scrolling.
- 🥇 **LyricsPlus Primary Priority Pipeline**:
  - **Pass 1 (Word-by-Word TTML Karaoke)**: **LyricsPlus** (Primary) ➔ AMLL Dev ➔ LRCMux ➔ Unison ➔ TTMLLIB.
  - **Pass 2 (Synced Line LRC Fallback)**: **LyricsPlus** (Primary) ➔ LRCMux ➔ Unison ➔ TTMLLIB ➔ LRCLIB ➔ NetEase.
- 🌐 **Automatic Translation & Dual-Line Merger**: Native merging of dual-line translations (e.g., Javanese, Japanese, Korean, Chinese) into unified main text + sub-text translation lines.
- 🔤 **Dynamic Word Spacing**: Intelligent XML whitespace preservation and alphanumeric token padding to ensure English words never clump together (`We don't gotta be in love, no`).
- ⏱️ **Sub-Second 0.1s Fine Sync**: Precise `+100ms` / `-100ms` offset controls available via the system tray context menu.
- 🍃 **Ultra-Low ~0.1 MB RAM & 0.0% Idle GPU/CPU Footprint**:
  - **~0.1 MB Working Set RAM**: Powered by Win32 `SetProcessWorkingSetSize` and `EmptyWorkingSet` memory working set trimming, maintaining reported Task Manager RAM at **~0.1 MB – 0.5 MB**.
  - **0.0% Idle CPU & GPU Usage**: Smart frame-skipping guard skips Direct2D rendering, layout calculations, and window updates when paused or static.
  - **71% Cut in Direct2D GPU Draw Calls**: High-performance single-pass drop shadow rendering.
  - **Manual RAM Release**: Instant "Trim Memory (Release RAM)" action in the system tray menu.
- 📌 **Lock / Click-Through Mode**: Lock position (`WS_EX_TRANSPARENT`) for seamless click-through workflow while listening.

---

## 🛠️ Installation & Building

### Prerequisites
- **Windows 10 / 11**
- **Rust toolchain** (1.75+)

### Building from Source

```bash
# Clone repository
git clone https://github.com/py7hon/minilyricv2.git
cd minilyricv2

# Build debug binary (includes real-time HTTP debug output)
cargo build

# Build release binary (Optimized, zero-console release executable)
cargo build --release
```

The compiled executable will be at `target/release/minilyricv2.exe`.

---

## 🎛️ Controls & Usage

| Action | Control |
| :--- | :--- |
| **Move Overlay** | Click and drag the track title/artist header (when unlocked). |
| **Toggle Click-Through Lock** | Click the 🔒 / 🔓 icon in the top-right corner, or right-click the system tray icon and select **Unlock / Lock Position**. |
| **Tray Context Menu** | Right-click the system tray icon in the Windows taskbar. |
| **Fine Sync Offset Adjustment** | Select **Sync: Faster (+100ms)** or **Sync: Slower (-100ms)** in the tray menu. |

---

## ⚙️ Configuration (`config.toml`)

Mini Lyric v2 automatically creates `config.toml` on first launch:

```toml
# Typography & Sizing
font_family = "Inter"
font_size_active = 30
font_size_side = 14
font_size_sub = 12
font_size_title = 20
font_size_artist = 15
line_spacing = 75.0
base_center_y = 85.0

# Sync Offset & Window Transparency
offset_ms = 0
opacity = 1.0

# Color Palette (HEX)
active_hex = "ffffff"
karaoke_hex = "cba6f7"
side_hex = "cbd5e1"
sub_hex = "f8fafc"
title_hex = "ffffff"
artist_hex = "e2e8f0"
card_bg_hex = "141420"
show_card = false

# Karaoke Animation Effect ("wave", "pop", "fade", "sweep", "glow", "none")
karaoke_effect = "wave"

# Drop Shadow Customization
shadow_enabled = false
shadow_hex = "000000"
shadow_opacity = 0.45
shadow_offset_x = 1.5
shadow_offset_y = 1.5
shadow_blur = 3.0

# Memory Trimming & Working Set Optimization
auto_trim_memory = true
trim_interval_secs = 5
```

### 🎭 Karaoke Animation Modes (`karaoke_effect`)
- `"star_bounce"` (alias `"star"`, `"ball"`): KaraFX / ASS-style bouncing star indicator with 360° rotating particle explosion sparkles *(Recommended, anime-style)*.
- `"zoom"`: Smooth zoom expansion peak.
- `"pulse"`: Breathing scale pulse in/out.
- `"wave"`: Smooth vertical bounce & color transition.
- `"bounce"`: Playful spring drop from above.
- `"slide"`: Glides in smoothly from the left into place.
- `"rise"`: Slides up from below while fading to color.
- `"tilt"`: Playful rotation tilt angle while sung.
- `"stretch"`: Cartoony squish & stretch elastic effect.
- `"shake"`: Horizontal jitter that settles into place.
- `"shimmer"`: Bright flash of light fading to highlight color.
- `"neon"`: Vibrant RGB rainbow spectrum shift.
- `"float"`: Ethereal floating vertical hover wave.
- `"pop"`: Dynamic scale-up and lift transform effect.
- `"fade"`: Smooth color crossfade transition.
- `"sweep"` (or `"kf"`): ASS/SSA `\kf`-style left-to-right fill wipe.
- `"glow"`: Highlighted halo effect surrounding active syllables.
- `"none"`: Instant color swap with zero motion animation.

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for details.
