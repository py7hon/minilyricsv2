# Mini Lyric v2 🎵✨

> **A modern, lightweight, transparent floating lyrics overlay for Windows.**  
> Powered by Rust, Win32 GDI, Windows System Media Transport Controls (GSMTC), and multi-source TTML/LRC providers.

---

## 🌟 Key Features

- 💎 **Transparent Floating Overlay**: Sleek, borderless, glasslike overlay that floats over any application or media player (YouTube Music, Spotify, Apple Music, Chrome, etc.).
- 🎤 **Word-by-Word Karaoke Animation**: Real-time syllable pop-scaling and smooth vertical line scrolling.
- 🌐 **Automatic Romaji, Romaja & Pinyin**: Automated transliteration and translation sub-badges for Japanese, Korean, Chinese, and non-English tracks.
- ⚡ **Near-Zero CPU & Memory Footprint**:
  - ~0.0% CPU when media is paused or idle.
  - Strict Win32 GDI handle deselection ensuring **0% RAM growth or handle leaks**.
- 🚀 **Multi-Provider Fallback Pipeline**:
  1. **AMLL Dev API** (`https://api.amll.dev/`) — *Primary TTML Syllable Source*
  2. **TTMLLIB** (`https://ttmllib.xyz/`) — *Tokenless TTML & LRC Source*
  3. **LRCLIB** (`https://lrclib.net/`) — *Open-source LRC Database*
  4. **NetEase Cloud Music API** (`music.163.com`) — *Fallback LRC & Translation Source*
- 📌 **Lock / Click-Through Mode**: Lock the overlay in place to make it click-through (`WS_EX_TRANSPARENT`), allowing uninterrupted workflow.
- 🎛️ **System Tray Controls**: Adjust window size, lock position, or fine-tune audio sync timing on the fly.

---

## 🛠️ Installation & Building

### Prerequisites
- **Windows 10 / 11**
- **Rust toolchain** (1.70+)

### Building from Source

```bash
# Clone repository
git clone https://github.com/user/minilyricv2.git
cd minilyricv2

# Build debug binary
cargo build

# Build release binary (Optimized executable)
cargo build --release
```

The compiled binary will be available at `target/release/minilyricv2.exe`.

---

## 🎛️ Controls & Usage

| Action | Control |
| :--- | :--- |
| **Move Overlay** | Click and drag the track title/artist header (when unlocked). |
| **Toggle Click-Through Lock** | Click the 🔒 / 🔓 icon in the top-right corner, or right-click the system tray icon and select **Toggle Lock**. |
| **Tray Context Menu** | Right-click the system tray icon in the Windows taskbar overflow area. |
| **Sync Timing Adjustment** | Use **Sync: Faster (+500ms)** or **Sync: Slower (-500ms)** in the tray menu. |

---

## ⚙️ Configuration (`config.toml`)

Mini Lyric v2 automatically creates a `config.toml` file in its working directory upon first launch:

```toml
font_family = "Inter"
font_size_active = 30
font_size_side = 14
font_size_sub = 12
font_size_title = 20
font_size_artist = 15
line_spacing = 75.0
base_center_y = 85.0
offset_ms = 0
opacity = 1.0
active_hex = "ffffff"
karaoke_hex = "cba6f7"
side_hex = "cbd5e1"
sub_hex = "f8fafc"
title_hex = "ffffff"
artist_hex = "e2e8f0"
show_card = false
```

---

## 📄 License

Distributed under the [MIT License](file:///d:/projj/minilyricv2/LICENSE). See `LICENSE` for details.
