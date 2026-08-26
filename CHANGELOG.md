# Release Notes - v0.1.12 📜✨

## 🌟 Highlights & Major Improvements

- **🎤 LyricsPlus Duet & Secondary Singer Fix**:
  - Updated `convert_kpoe_array_to_ttml` in `src/providers/ttmllib.rs` to extract the `singer` property from the `element` object inside LyricsPlus KPOE JSON. This ensures duet lines are properly assigned `ttm:agent="v2"` and correctly right-aligned on screen.
- **🔤 TTML Word Boundary Formatting Revert**:
  - Reverted `is_word_boundary_space` in `src/lrc_parser.rs` to its robust, original logic (ignoring XML formatting newlines). The missing word space issue is now natively handled during TTML generation (`ttmllib.rs`), preventing syllables from incorrectly splitting (e.g., `Me nya pa mu` -> `Menyapamu`).

---

# Release Notes - v0.1.11 📜✨

- **🎨 Header Duet Coloring & Background Syllables Fixes**:
  - Fixed single-word duet coloring bug in header lines by correcting byte-offset calculations in `DWRITE_TEXT_RANGE`.
  - Applied background vocal row split logic, extracting background syllables (e.g., parenthesized groups like `(oh, Kasihku)`) to a secondary row with distinct styling.
  - Handled LRCMux missing `isBackground` field by applying fallback regex pattern matching for parenthesized background syllables.

---

# Release Notes - v0.1.10 📜✨
## 🌟 Highlights & Major Improvements

- **🎵 Multi-Vocal Singer Alignment & Unison Mode**:
  - Added support for Unison (`singer_index = 2`, center-aligned `((width_f - text_width) / 2.0).max(15.0)`).
  - Added unison line detection in TTML/LRC (`v0`, `agent="v0"`, multiple different agents per `<p>` block, "unison", "together", "both").
  - Preserved left-alignment (`15.0`) for Singer 1 (`singer_index = 0`) and right-alignment for Singer 2 (`singer_index = 1`).
- **🔤 Word-Boundary Wrapping & Layout Scaling**:
  - Implemented syllable grouping into `WordUnit`s to prevent word splitting across lines during word wrapping.
  - Dynamically calculated `active_h` pre-measurement using `WordUnit` line packing so preview line spacing scales smoothly when lyric lines wrap.
- **🎤 SubText Overlap Protection & Vertical Clamping**:
  - Enforced `minimal_y > available_bottom` check to skip rendering `sub_text` when vertical space in compact overlay windows is insufficient, preventing overlap with main lyric lines.
- **🎼 Animated Instrumental Indicator**:
  - Replaced static instrumental placeholder text with a sleek musical note icon (`♪`).
  - Added a **Vertical Progressive Fill Animation (bottom-to-top sweep)** that fills the musical note icon smoothly in lockstep with instrumental line elapsed time.
  - Replaced the active lyric line display during instrumental gaps with the animated note icon to prevent old lyric text from lingering on screen.
- **⚡ Eager Early-Exit Provider Fetch & Concurrent Parallelization**:
  - Converted `fetch_lyrics` orchestration in `src/lyrics_api.rs` to eager background Tokio tasks (`tokio::spawn`) with `mpsc` channel communication and 6-second timeouts per provider. Returns immediately on the first valid TTML hit (~200–400ms) without waiting for slow providers.
  - Parallelized `fetch_betterlyrics_lyrics` simple and detailed HTTP queries using `tokio::join!`.
- **🔤 Precision TTML & KPOE Word Boundary Space Fixes**:
  - Fixed `parse_ttml()` space detection in `src/lrc_parser.rs` to ignore XML formatting indentation while preserving genuine single word boundary spaces.
  - Fixed `convert_kpoe_array_to_ttml()` in `src/providers/ttmllib.rs` to respect exact syllable trailing spaces from KPOE JSON response.
- **🧹 Clean Codebase & Zero Clippy Warnings**:
  - Cleaned up all temporary debug logs.
  - Resolved all `cargo clippy` warnings and formatted codebase with `cargo fmt`.

---

# Release Notes - v0.1.9 📜✨

## 🌟 Highlights & Major Improvements

- **🌐 Better Lyrics API (`lyrics.pyoi.eu.org`) Integration & Single-Round Speed Optimization**:
  - Added native provider module `src/providers/betterlyrics.rs` for `https://lyrics.pyoi.eu.org/openapi.json`.
  - Supports word-by-word Musixmatch & TTML karaoke parsing, converting absolute syllable timestamp tags `<00:13.50> ... <00:13.91>` into duration-accurate millisecond syllable tags.
  - **🛠️ Fixed JSON Stringified TTML Parsing (`{"ttml": "..."}`)**: Implemented recursive JSON string unwrapping in `betterlyrics.rs` and `lrc_parser.rs` so stringified JSON objects (`{"ttml": "<tt ...>"}`) are extracted to raw TTML XML instead of failing XML parsing and remaining stuck on `"Loading lyrics..."`.
  - **✨ LyricsPlus & Musixmatch Word-by-Word to Standard TTML XML Converter**: Converted word-by-word LRC lyrics strings (including delta timestamps `<00:00.60>`) in `lyricsplus.rs` and `betterlyrics.rs` into standard TTML XML format (`<tt xmlns="..."><body><div><p begin="..." end="..."><span begin="..." end="...">...</span></p></div></body></tt>`), ensuring all LyricsPlus responses produce true TTML XML markup with absolute word timestamps.
  - **🎵 Proportional TTML / Musixmatch Word-by-Word Duration Scaling**: Fixed compressed syllable durations (e.g. 20ms–80ms word-start gaps) by scaling syllable durations proportionally across the full line duration. The karaoke bouncing star and word highlight now move in perfect lockstep with the singer from start to end of each line.


  - **⚡ Instant 0ms UI Lyric Render**: Render matched lyrics lines immediately on screen while running sub-text translations asynchronously in background `tokio::spawn` tasks.
  - **🚀 Parallel Multi-Provider Race**: Query all providers concurrently in parallel single-round execution, ranking `BetterLyrics` as #1 priority while eliminating sequential blocking delays.




---

# Release Notes - v0.1.6 📜✨


## 🌟 Highlights & Major Improvements

- **🎤 AMLL / Apple Music / LyricsPlus / LRCMux Sub-Text Karaoke & Pill Container**:
  - Implemented word-by-word active karaoke fade/wave sweeping for Japanese Romaji, Korean Romaja, and Chinese Pinyin sub-text.
  - Added rounded pill capsule container `( mou sukoshi dake )` with subtle translucent background and dark border, matching modern Apple Music / AMLL lyric players.
  - Rendered active sub-text karaoke in bright white (`#ffffff`) with soft glow shadow matching target design aesthetic.
- **⚡ Priority Boidu & Concurrent Sub-100ms Fast Query Race**:
  - Set **Boidu (`lyrics-api.boidu.dev`)** as top priority provider alongside **LyricsPlus**.
  - Implemented a fast pre-flight query race that returns lyrics in ~80-150ms without waiting for slow secondary endpoints.
  - Included `https://lyrics-api.boidu.dev/getLyrics` directly in `lyricsplus.rs` concurrent fallback race.
- **🔤 Native LRCMux & KPOE JSON Syllable & Transliteration Extraction**:
  - Enhanced KPOE JSON converter to parse both `syllabus` array (`<00:19.10>いつか <00:19.47>僕らの`) and `transliteration` object (`itsuka bokura no ue o suresure ni`).
  - Added support for LRCMux native `api.lrcmux.dev/get` JSON schema (`{ "lines": [ { "start", "words": [ ... ] } ] }`).
  - Fixed timestamp scale threshold (`500.0` ms threshold in `parse_time_val`), eliminating 81-minute timestamp scaling bugs (`[81:46.00]` -> `[00:04.90]`).
- **🧠 Automatic Algorithmic Multilingual Transliteration Engine**:
  - **Korean (Romaja)**: Programmatically decomposes Korean Hangul Syllables (`0xAC00..=0xD7AF`) into Initial, Medial, and Final Jamo using exact **Unicode Math** (`hangul_to_romaja_char`) — zero hardcoded word lists (`사랑해` -> `sa rang hae`).
  - **Japanese (Romaji)**: Programmatically resolves Hiragana sokuon (`っ` / `ッ`) consonant gemination (`tte`, `kke`, `ppe`, `sse`) and corrects kanji reading misreadings (`笑っ` / `笑って` -> **`waratte`**, `出来` -> **`dekiru`**).
  - **Chinese (Pinyin)**: Preserves initial/final Pinyin consonant digraphs (`zh`, `ch`, `sh`, `ng`).
- **🛠️ Dynamic Google Translate `dt=rm` Parser**:
  - Refactored `translation.rs` to dynamically parse `dt=rm` romanization segments instead of using static array index assumptions.

---

## 🛠️ Detailed Change Log

### Providers & Lyrics Pipeline
- Integrated Boidu API (`lyrics-api.boidu.dev/getLyrics`) as top priority in `lyrics_api.rs` and `lyricsplus.rs`.
- Added LRCMux native `{ "lines": [...] }` word timing parser in `src/providers/lrcmux.rs`.
- Fixed timestamp scaling bug (`f < 10000.0` -> `500.0` ms threshold) in `src/providers/ttmllib.rs` and `src/providers/lrcmux.rs`.
- Reordered `parse_lyricsplus_response` field priority to always prefer word-by-word karaoke arrays over plain line-synced `syncedLyrics` strings.

### Transliteration & Text Engine
- Added `hangul_to_romaja_char` and `convert_hangul_text_to_romaja` using Unicode math in `src/lrc_parser.rs`.
- Implemented `fix_multilingual_transliteration_misreadings` for Japanese, Korean, and Chinese.
- Refactored `translate_text` in `src/providers/translation.rs` for dynamic `dt=rm` segment extraction.

### Rendering & Aesthetics
- Added sub-text active karaoke fade / wave sweeping (`sub_karaoke_effect = "wave"` or `"fade"`).
- Rendered sub-text pill container with Direct2D `FillRoundedRectangle` and `DrawRoundedRectangle`.
- Applied white text layout clipping for active sub-text karaoke animation sweep.

---

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
  - Fixed AMLL & Apple Music TTML missing suffix issue (`wasn'` -> `wasn't`, `you'` -> `you're`) by capturing untagged inter-span text between `</span>` and `<span` tags while normalizing multi-space XML indentation into single spaces.
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
