# Release Notes - v0.1.3 📜✨

## 🌟 Highlights & Major Improvements

- **⚡ Supercharged Concurrent Provider Fetching**: Query all lyrics providers concurrently in a single-round pass for near-instant lyrics fetching response times.
- **🌐 Independent Better-Lyrics Providers**: Integrated dedicated native provider modules for **Musixmatch** (`apic-desktop.musixmatch.com`), **Binimum** (`lyrics-api.binimum.org`), and **BoiduLyricsApi** (`lyrics-api.boidu.dev`).
- **🚀 Zero Proxy Dependency**: Removed DaCubeKing provider and proxy dependency in favor of direct native API requests.
- **🔤 Precision TTML Word Spacing**: Perfected XML word span parsing. Syllable splits within words (`en` + `ding`) join seamlessly (`ending`), while preserving exact spaces between separate words (`I don't even wanna do this anymore`).
- **🎨 Expanded Karaoke Animations**: Added richer karaoke highlighting animation options (`pulse`, `wave`, `fade`, `shake`, `rise`, `glow`, `sweep`, `pop`, `none`) configurable in `config.toml`.
- **🎤 Duet & Multi-Vocal Singer Colors**: Added support for secondary singer / duet lines (`ttm:agent="v2"`, `agent="v2"`, or lines in parentheses/brackets) using a separate customizable highlight color (`karaoke_v2_hex`, default: `#f38ba8`).
- **📐 Dynamic Title & Artist Layout**: Replaced static 1-line text offset calculations with DirectWrite `IDWriteTextLayout::GetMetrics` text measuring. Long multi-line song titles now wrap gracefully without overlapping the artist name or lyrics.

---

## 🛠️ Detailed Change Log

### Providers & Lyrics Pipeline
- Accelerated provider query pipeline to fetch all providers in parallel in a single round.
- Integrated **Musixmatch**, **Binimum**, and **BoiduLyricsApi** as independent concurrent providers in the main lyrics search pipeline.
- Removed `DaCubeKing` provider and external proxy dependency.

### Rendering & Direct2D Engine
- Removed artificial word padding in Direct2D karaoke syllable layout renderer for 100% accurate character alignment.
- Added new karaoke animation effects (`pulse`, `shake`, `rise`).
- Dynamically calculated multi-line title and artist layout text bounds using `GetMetrics`.

### Bug Fixes
- Fixed TTML span parser to ignore XML formatting line breaks between syllable tags while preserving inline word spaces.
- Fixed song title and artist name text overlapping issue when song title wraps onto multiple lines.
- Filtered out embedded TTML translation and transliteration roles (Chinese translations & Pinyin) from AMLL TTML responses.
- Fixed active lyric lines being hidden/clipped when long song titles create tall header blocks.
- Fixed romaji/pinyin/romaja sub_text lines being clipped at the bottom of the window card by dynamically clamping active line vertical bounds.

---

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
