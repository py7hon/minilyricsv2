# Mini Lyric v2 Architecture 🏛️

Mini Lyric v2 is a high-performance, modular Windows application written in Rust. It utilizes Direct2D hardware acceleration, DirectWrite typography, Windows System Media Transport Controls (GSMTC), and a multi-provider TTML/LRC lyrics aggregation engine.

---

## 📐 High-Level Component Diagram

```
                 +-----------------------------------+
                 |    Windows GSMTC Media Listener   |
                 |         (src/gsmtc.rs)            |
                 +-----------------+-----------------+
                                   | Instant Polling (0ms)
                                   v
                 +-----------------+-----------------+
                 |       Global AppState (Mutex)     |
                 |        (src/app_state.rs)         |
                 +-----------------+-----------------+
                                   |
           +-----------------------+-----------------------+
           |                                               |
           v                                               v
+--------------------------+                   +-----------------------+
|  Lyrics Provider Client  |                   |   Direct2D / Win32    |
|   (src/lyrics_api.rs)    |                   |   Render & Window     |
+------------+-------------+                   |   (src/render.rs &    |
             |                                 |   src/d2d_engine.rs &  |
    +--------+--------+--------+--------+      |    src/window.rs)     |
    |        |        |        |        |      +-----------------------+
    v        v        v        v        v
LyricsPlus AMLL    LRCMux   Unison  TTMLLIB / LRCLIB / NetEase
```

---

## 🗂️ Module Responsibilities

### 1. Entry Point & Orchestration (`src/main.rs`)
- Loads user preferences via `load_or_create_config()` in `src/config.rs`.
- Spawns the GSMTC media monitor and initializes the `LyricsClient`.
- Runs a 30ms Tokio ticker loop querying playback state, triggering lyrics fetches on track changes, and dispatching automatic translation for missing sub-text.

### 2. Global State Management (`src/app_state.rs`)
- Maintained inside a thread-safe `Arc<Mutex<AppState>>` (aliased globally via `APP_STATE`).
- Holds track metadata (`MediaInfo`), parsed lyrics lines (`LrcLine`), scroll offsets (`float_index`), fine sync adjustments (`offset_ms`), click-through lock state (`is_locked`), and layout dirty flags (`layout_cache_dirty`).

### 3. Media Integration (`src/gsmtc.rs`)
- Interacts with `GlobalSystemMediaTransportControlsSessionManager` via WinRT (`windows::Media::Control`).
- Obtains real-time track metadata (Title, Artist, Album, Duration, Playback Status) with instantaneous 0ms polling.
- Calculates microsecond-accurate position interpolation using `timeline.Position()` and `timeline.LastUpdatedTime()`.

### 4. Multi-Provider Lyrics Pipeline (`src/lyrics_api.rs` & `src/providers/`)
- **`lyricsplus.rs` [PRIMARY]**: Primary provider for word-by-word TTML karaoke and synced line LRC lyrics (`lyricsplus.prjktla.my.id`). Includes KPOE JSON array converter and endpoint mirrors.
- **`amll.rs`**: AMLL Dev OpenAPI 3.2.0 provider (`api.amll.dev`) for Apple Music TTML XML lyrics.
- **`lrcmux.rs`**: LRCMux OpenAPI 3.1.0 provider (`api.lrcmux.dev`).
- **`unison.rs`**: Unison provider (`unison.boidu.dev`).
- **`ttmllib.rs`**: TTMLLIB provider (`ttmllib.xyz`).
- **`lrclib.rs`** & **`netease.rs`**: Open-source LRC & NetEase Cloud Music fallback providers.
- **`translation.rs`**: Google Translate engine (`dt=rm`), extracting Romaji (Japanese), Romaja (Korean), and Pinyin (Chinese) reading badges.

### 5. Native TTML XML Engine & Parser (`src/lrc_parser.rs`)
- **Native TTML Parser** (`parse_ttml`): Parses `<tt>`, `<p>`, and `<span begin="..." end="...">` XML tags into structured `LrcLine` and `Syllable` instances.
- **Multi-Format Timestamp Converter** (`parse_ttml_time_str`): Converts `HH:MM:SS.mmm`, `MM:SS.mmm`, seconds (`14.500s`), and milliseconds (`14500ms`).
- **Dual-Line Translation Merger**: Merges adjacent lines sharing matching timestamps (within 150ms) into single `LrcLine` objects with main text + sub-text translation (`sub_text`).
- **XML Whitespace Preservation**: Retains spaces outside `</span>` tags and applies dynamic word token padding so English words never clump together.

### 6. Hardware-Accelerated Direct2D & DirectWrite Engine (`src/d2d_engine.rs` & `src/render.rs`)
- Utilizes `ID2D1RenderTarget` and `IDWriteFactory` for hardware-accelerated text layout rendering.
- Features dynamic text layout caching (`get_cached_text_layout`) with automated layout cache invalidation (`layout_cache_dirty`) on song transitions.
- Supports multiple karaoke animation modes (`wave`, `pop`, `fade`, `sweep`, `glow`, `none`) with axis-aligned clipping (`PushAxisAlignedClip`).

### 7. System Tray & Window Management (`src/tray.rs` & `src/window.rs`)
- Creates a borderless layered popup window (`WS_EX_TOPMOST | WS_EX_LAYERED`).
- Handles window hit testing (`WM_NCHITTEST`), window dragging, and click-through lock mode (`WS_EX_TRANSPARENT`).
- Provides system tray menu controls for window resizing, lock toggling, and fine `+100ms` / `-100ms` sync offset adjustments.
- Manages memory-cached 32-bit DIB section bitmap surfaces (`UpdateLayeredWindow`) rendered at ~30fps.
