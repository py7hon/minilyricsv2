# Release Notes - v0.1.2 📜✨

## 🌟 Highlights & Major Improvements

- **🥇 LyricsPlus Primary Priority**: **LyricsPlus** (`lyricsplus.prjktla.my.id`) is set as the #1 primary provider for both **Word-by-Word TTML Karaoke** (Pass 1) and **Line-Synced LRC** (Pass 2).
- **🎯 Native TTML XML Engine**: Full parsing support for Apple Music & AMLL TTML XML format with sub-second word span tags (`<span begin="..." end="...">`).
- **🌐 Dual-Line Translation Merger**: Merges dual-line LRC/TTML entries sharing matching timestamps into single unified lines with main text + sub-text translation (`sub_text`).
- **🔤 Dynamic Word Spacing**: XML whitespace preservation and alphanumeric token padding to ensure English/Latin words are naturally formatted (`We don't gotta be in love, no`).
- **⚡ Instant 0ms GSMTC Media Polling**: Microsecond-accurate Windows Media Transport Control polling with zero lag.
- **⏱️ Sub-Second 0.1s Fine Sync**: System tray controls (`+` / `-`) for fine-tuning audio sync in `+100ms` / `-100ms` steps.
- **🛡️ 0% Resource Leaks & 0 Compiler Warnings**: Strict GDI DC/Bitmap deselection (`DeleteObject`, `DeleteDC`), DirectWrite layout bounds checking, and 100% clean `cargo clippy -D warnings` CI pass.

---

## 🛠️ Detailed Change Log

### Providers & Lyrics Pipeline
- Set LyricsPlus as #1 primary provider.
- Added mirror endpoint fallbacks (`lyricsplus-seven.vercel.app`) and exact recording length query parameters (`album`, `duration`).
- Added LRCMux (`api.lrcmux.dev`) and Unison (`unison.boidu.dev`) provider support.

### Rendering & Direct2D Engine
- Direct2D hardware-accelerated text layout rendering (`ID2D1RenderTarget`, `IDWriteTextLayout`).
- Supports `wave`, `pop`, `fade`, `sweep`, `glow`, and `none` karaoke animation modes.
- Automated layout cache invalidation (`clear_layout_cache`) on track changes.

### Bug Fixes
- Fixed KPOE timestamp second-to-millisecond float conversion (`14.5s -> 14,500ms`).
- Fixed zero-drop position skew protection when GSMTC or web players buffer.
- Fixed UTF-8 string slicing boundary panics on Japanese/CJK characters.
- Fixed manual-strip and manual-map clippy warnings for `cargo clippy --all-targets -- -D warnings`.
