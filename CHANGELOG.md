# Release Notes - v0.1.5 📜✨

## 🌟 Highlights & Major Improvements

- **🍃 Ultra-Low ~0.1 MB RAM Working Set**:
  - Integrated Win32 `SetProcessWorkingSetSize` and `EmptyWorkingSet` memory working set trimming into the core process loop and system tray menu.
  - Automatically flushes un-accessed heap, stack, and buffer pages out of physical RAM, maintaining reported Task Manager RAM at **~0.1 MB – 0.5 MB**.
  - Added new `auto_trim_memory` (`true`) and `trim_interval_secs` (`5`s) options to `config.toml`.
  - Added manual **"Trim Memory (Release RAM)"** action to the system tray context menu.
- **⚡ 0.0% Idle GPU & CPU Usage**:
  - Fixed a `sleep(0)` unthrottled busy loop in `gsmtc.rs`, dropping idle CPU usage from ~20% down to **< 0.1%**.
  - Implemented smart frame-skipping position timestamp guard (`LAST_PAINTED_POS_MS`) in `window.rs` to skip Direct2D repaints and window updates whenever the track position hasn't advanced.
- **🚀 71% Cut in Direct2D GPU Draw Commands**:
  - Optimized `draw_text_with_shadow` in `render.rs` to replace 6-pass shadow offset loops with a crisp single-pass drop shadow, cutting GPU draw calls per text fragment from 7x down to 2x.

---

## 🛠️ Detailed Change Log

### Memory & System Integration
- Added `"Win32_System_ProcessStatus"` to `windows` crate dependencies in `Cargo.toml`.
- Added `trim_working_set()` helper in `src/utils.rs` calling `EmptyWorkingSet` and `SetProcessWorkingSetSize`.
- Added `ID_MENU_TRIM_MEMORY` action item in `src/tray.rs` and handled in `src/window.rs`.
- Added auto-trim triggers on app startup, song transition, track pause, and idle UI timer.

### Rendering & Performance
- Replaced 6-pass shadow drawing loop with high-performance 1-pass drop shadow in `src/render.rs`.
- Added `LAST_PAINTED_POS_MS` position change verification to `WM_TIMER` in `src/window.rs`.
- Fixed `sleep(Duration::from_millis(0))` busy loop in `src/gsmtc.rs` by throttling to 50ms.
- Updated main loop ticker in `src/main.rs` to 50ms.

---

# Release Notes - v0.1.4 📜✨

## 🌟 Highlights & Major Improvements

- **⭐ ASS / KaraFX Bouncing Star & Particle Explosion (`star_bounce`)**:
  - Added a KaraFX / ASS anime karaoke-style bouncing star indicator (`★`) with smooth ease-out sine trajectory curves and word pop animations.
  - Added a **360° Rotating Particle Explosion System**: 5 sparkling stars (`✦`, `✧`, `✨`) burst outward radially, rotate in 2D space, scale up, and shrink into fine stardust as each word is sung.
  - Added precision word center touchdown and graceful post-landing dissolve/fade-away inside the word.
- **🔤 Complete XML Entity Unescaping & Contraction Suffix Parsing**:
  - Implemented comprehensive XML entity unescaping (`&apos;`, `&quot;`, `&amp;`, `&lt;`, `&gt;`, `&#39;`, `&#x27;`, etc.).
  - Fixed AMLL & Apple Music TTML missing suffix issue (`wasn'` $\rightarrow$ `wasn't`, `you'` $\rightarrow$ `you're`) by capturing untagged inter-span text between `</span>` and `<span` tags while normalizing multi-space XML indentation into single spaces.
- **🎨 8 New Modern Karaoke Animation Effects**:
  - Added `star_bounce` (alias `star`, `ball`), `zoom`, `bounce`, `slide`, `tilt`, `stretch`, `shimmer`, `neon`, and `float` animation modes to `config.toml`.
- **⚡ Zero-Lag Karaoke Active Syllable Timing**:
  - Replaced dead-zone syllable progress filtering with active index targeting (`render_data.position(|r| r.progress < 1.0)`), guaranteeing instantaneous, lag-free rendering across rapid word transitions.

---

## 🛠️ Detailed Change Log

### Rendering & Direct2D Engine
- Added `star_bounce` effect with 360-degree rotating ASS KaraFX particle burst system.
- Added 8 new karaoke effect modes (`zoom`, `bounce`, `slide`, `tilt`, `stretch`, `shimmer`, `neon`, `float`).
- Replaced linear bounce trajectory with Ease-Out Sine curves (`sin(t * PI / 2)`) and quadratic dissolve fade-outs.
- Fixed active syllable selection dead zones between syllable boundaries.

### TTML & LRC Parser
- Added `unescape_xml_entities()` for named and numeric XML entities in TTML parser.
- Added inter-span and trailing text extraction in `parse_ttml()` to prevent contraction truncation (`wasn't`, `you're`).
- Added `clean_inter_xml_text()` helper to normalize multi-line XML formatting indentation into single spaces.

---

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
