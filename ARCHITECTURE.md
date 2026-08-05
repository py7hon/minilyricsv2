# Mini Lyric v2 Architecture 🏛️

Mini Lyric v2 is designed as a high-performance, modular Windows application written in Rust. It utilizes native Win32 APIs for transparent layered rendering and Windows Media Transport Controls (GSMTC) for system-wide media detection.

---

## 📐 High-Level Component Diagram

```
                 +-----------------------------------+
                 |    Windows GSMTC Media Listener   |
                 |         (src/gsmtc.rs)            |
                 +-----------------+-----------------+
                                   | Polling (40ms)
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
|  Lyrics Provider Client  |                   |   Win32 Render & GUI  |
|   (src/lyrics_api.rs)    |                   |   (src/render.rs &    |
+------------+-------------+                   |    src/window.rs)     |
             |                                 +-----------------------+
    +--------+--------+--------+--------+
    |        |        |        |        |
    v        v        v        v        v
  AMLL    TTMLLIB   LRCLIB  NetEase Translate
```

---

## 🗂️ Module Responsibilities

### 1. Entry Point & Task Orchestration `src/main.rs`
- Loads user preferences via `src/config.rs`.
- Spawns the media monitor thread and initializes the `LyricsClient`.
- Runs an asynchronous Tokio ticker loop that queries playback state, triggers lyrics fetches on song changes, and dispatches automatic Romanization/transliteration for missing sub-text.

### 2. Global State Management `src/app_state.rs`
- Maintains `AppState` inside a thread-safe `Arc<Mutex<AppState>>`.
- Stores current track metadata (`MediaInfo`), parsed lyrics lines (`LrcLine`), scroll offsets (`float_index`), window lock state, and GDI paint caches (`PAINT_CACHE`).

### 3. Media Integration `src/gsmtc.rs`
- Interacts with `GlobalSystemMediaTransportControlsSessionManager` via WinRT (`windows::Win32::System::WinRT`).
- Obtains real-time track metadata (Title, Artist, Album, Duration, Playback Status).
- Calculates microsecond-accurate position interpolation using `get_current_windows_ticks()`.

### 4. Multi-Provider Lyrics Pipeline `src/lyrics_api.rs` & `src/providers/`
- **`amll.rs`**: Primary source for high-precision Apple Music-style TTML word-by-word karaoke lyrics.
- **`ttmllib.rs`**: Fallback tokenless TTML & LRC source.
- **`lrclib.rs`**: Open-source LRC fallback database.
- **`netease.rs`**: NetEase Cloud Music API & TTML XML string parser (`ttml_to_lrc`).
- **`translation.rs`**: Google Translate Romanization engine (`dt=rm`), extracting Romaji (Japanese), Romaja (Korean), and Pinyin (Chinese) reading badges.

### 5. Leak-Free Win32 GDI Rendering `src/render.rs`
- Renders text onto a double-buffered compatible DC (`mem_dc`).
- Handles dynamic CJK font face resolution (`get_font_face_for_text`).
- Renders active word pop-scaling using a sine ease-in pop + cubic decay curve `(1.0 - t)³`.
- **Memory Safety Guarantee**: Every GDI font handle (`CreateFontW`) and brush is strictly deselected with `SelectObject(mem_dc, old_font)` **before** `DeleteObject(hFont)` is called, guaranteeing **0% GDI handle leaks**.

### 6. System Tray & Window Controls `src/tray.rs` & `src/window.rs`
- Creates a borderless layered window (`WS_EX_TOPMOST | WS_EX_LAYERED`).
- Handles window hit testing (`WM_NCHITTEST`), dragging, and click-through lock mode (`WS_EX_TRANSPARENT`).
- Manages taskbar notification icon registration (`Shell_NotifyIconW`) and context menu dispatching (`TrackPopupMenu`).
