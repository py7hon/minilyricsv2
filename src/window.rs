#![allow(static_mut_refs)]
use crate::app_state::APP_STATE;
use crate::d2d_engine::get_d2d_engine;
use crate::render::render_window_d2d;
use crate::settings_window::open_settings_window;
use crate::tray::{
    add_tray_icon, remove_tray_icon, show_tray_menu, toggle_lock_state, ID_MENU_CHECK_UPDATE,
    ID_MENU_EXIT, ID_MENU_LOCK, ID_MENU_OFFSET_MINUS, ID_MENU_OFFSET_PLUS, ID_MENU_OFFSET_RESET,
    ID_MENU_PROVIDER_BASE, ID_MENU_SETTINGS, ID_MENU_SIZE_LARGE, ID_MENU_SIZE_MEDIUM,
    ID_MENU_SIZE_SMALL, ID_MENU_TRIM_MEMORY, WM_TRAYICON,
};
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Direct2D::ID2D1RenderTarget;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, EndPaint,
    InvalidateRect, ScreenToClient, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, PAINTSTRUCT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    LoadCursorW, RegisterClassW, SetTimer, SetWindowPos, TranslateMessage, UpdateLayeredWindow,
    CS_HREDRAW, CS_VREDRAW, HTCAPTION, HTCLIENT, HWND_TOPMOST, IDC_ARROW, MSG, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_SHOW, ULW_ALPHA, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCHITTEST, WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

struct SurfaceCache {
    dc: HDC,
    hbmp: HBITMAP,
    old_bmp: HGDIOBJ,
    target: ID2D1RenderTarget,
    width: i32,
    height: i32,
}

impl SurfaceCache {
    pub unsafe fn release(&mut self) {
        let _ = SelectObject(self.dc, self.old_bmp);
        let _ = DeleteObject(HGDIOBJ(self.hbmp.0));
        let _ = DeleteDC(self.dc);
    }
}

static mut SURFACE_CACHE: Option<SurfaceCache> = None;
static mut LAST_PAINTED_INDEX: usize = usize::MAX;
static mut LAST_PAINTED_POS_MS: u64 = u64::MAX;
static mut TOPMOST_TICK_COUNTER: u32 = 0;
const TOPMOST_REASSERT_TICKS: u32 = 60;
static mut IDLE_TRIM_COUNTER: u32 = 0;

pub static mut MAIN_HWND: Option<HWND> = None;

pub fn redraw_main_window() {
    unsafe {
        if let Some(hwnd) = MAIN_HWND {
            if !hwnd.0.is_null() {
                let _ = InvalidateRect(hwnd, None, false);
            }
        }
    }
}

pub fn switch_provider_by_index(idx: usize) {
    if let Some(state_arc) = unsafe { APP_STATE.as_ref() } {
        let lines = if let Ok(mut s) = state_arc.lock() {
            if idx < s.available_providers.len() {
                let hit = s.available_providers[idx].clone();
                s.active_provider_index = idx;
                s.provider_name = Some(hit.provider_name.clone());
                s.plain_lyrics = hit.plain.clone();
                let parsed_lines = crate::lrc_parser::parse_lrc(&hit.content);
                s.lyrics_lines = parsed_lines.clone();
                s.layout_cache_dirty = true;
                Some(parsed_lines)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(parsed_lines) = lines {
            crate::lyrics_api::LyricsClient::spawn_subtext_fill(state_arc, parsed_lines);
        }
        redraw_main_window();
    }
}

pub unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            MAIN_HWND = Some(hwnd);
            SetTimer(hwnd, 1, 16, None);
            add_tray_icon(hwnd);
            LRESULT(0)
        }
        WM_TIMER => {
            let mut should_invalidate = false;
            if let Some(state_arc) = APP_STATE.as_ref() {
                if let Ok(mut s) = state_arc.lock() {
                    let target = s.current_index as f32;
                    let diff = target - s.float_index;
                    let still_animating = if diff.abs() > 0.005 {
                        s.float_index += diff * 0.45;
                        true
                    } else {
                        s.float_index = target;
                        false
                    };
                    let active_playing = s.media.is_playing && !s.media.title.is_empty();
                    if active_playing && s.last_pos_ms > 0 {
                        s.media.position_ms =
                            s.last_pos_ms + s.last_pos_update.elapsed().as_millis() as u64;
                    }

                    let current_pos_ms = s.media.position_ms;
                    let index_changed = s.current_index != LAST_PAINTED_INDEX;
                    let active_line_is_karaoke = {
                        let mode = s.config.karaoke_mode.trim().to_lowercase();
                        match mode.as_str() {
                            "always" => true,
                            "never" => false,
                            _ => s
                                .lyrics_lines
                                .get(s.current_index)
                                .map(|line| line.is_karaoke)
                                .unwrap_or(false),
                        }
                    };
                    should_invalidate = still_animating
                        || s.is_loading
                        || index_changed
                        || (active_playing && active_line_is_karaoke);
                    if should_invalidate {
                        LAST_PAINTED_INDEX = s.current_index;
                        LAST_PAINTED_POS_MS = current_pos_ms;
                    }
                }
            }
            if should_invalidate {
                let _ = InvalidateRect(hwnd, None, false);
                IDLE_TRIM_COUNTER = 0;
            } else {
                IDLE_TRIM_COUNTER += 1;
                if IDLE_TRIM_COUNTER == 15
                    || (IDLE_TRIM_COUNTER > 15 && IDLE_TRIM_COUNTER.is_multiple_of(90))
                {
                    crate::utils::trim_working_set();
                }
            }
            TOPMOST_TICK_COUNTER += 1;
            if TOPMOST_TICK_COUNTER >= TOPMOST_REASSERT_TICKS {
                TOPMOST_TICK_COUNTER = 0;
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            if x >= rect.right - 40 && y <= 30 {
                toggle_lock_state(hwnd);
            } else {
                // langsung drag tanpa ReleaseCapture, ini work di 0.58
                let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageW(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN,
                    WPARAM(HTCAPTION as usize),
                    LPARAM(0),
                );
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => LRESULT(0),
        WM_NCHITTEST => {
            let pt = POINT {
                x: (lparam.0 & 0xFFFF) as i16 as i32,
                y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
            };
            let mut client_pt = pt;
            let _ = ScreenToClient(hwnd, &mut client_pt);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            if client_pt.x >= rect.right - 40 && client_pt.y <= 30 {
                LRESULT(HTCLIENT as isize)
            } else {
                LRESULT(HTCAPTION as isize)
            }
        }
        WM_RBUTTONUP => {
            show_tray_menu(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as u32;
            if (ID_MENU_PROVIDER_BASE..ID_MENU_PROVIDER_BASE + 50).contains(&id) {
                let idx = (id - ID_MENU_PROVIDER_BASE) as usize;
                switch_provider_by_index(idx);
            } else {
                match id {
                    ID_MENU_SETTINGS => {
                        open_settings_window();
                    }
                    ID_MENU_CHECK_UPDATE => {
                        crate::updater::check_for_updates_async(true);
                    }
                    ID_MENU_LOCK => {
                        toggle_lock_state(hwnd);
                    }
                    ID_MENU_SIZE_SMALL => {
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            0,
                            0,
                            480,
                            160,
                            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                    ID_MENU_SIZE_MEDIUM => {
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            0,
                            0,
                            560,
                            200,
                            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                    ID_MENU_SIZE_LARGE => {
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            0,
                            0,
                            680,
                            240,
                            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                    ID_MENU_OFFSET_PLUS => {
                        if let Some(state_arc) = APP_STATE.as_ref() {
                            if let Ok(mut s) = state_arc.lock() {
                                s.offset_ms += 100;
                                s.config.offset_ms = s.offset_ms;
                            }
                        }
                    }
                    ID_MENU_OFFSET_MINUS => {
                        if let Some(state_arc) = APP_STATE.as_ref() {
                            if let Ok(mut s) = state_arc.lock() {
                                s.offset_ms -= 100;
                                s.config.offset_ms = s.offset_ms;
                            }
                        }
                    }
                    ID_MENU_OFFSET_RESET => {
                        if let Some(state_arc) = APP_STATE.as_ref() {
                            if let Ok(mut s) = state_arc.lock() {
                                s.offset_ms = 0;
                                s.config.offset_ms = 0;
                            }
                        }
                    }
                    ID_MENU_TRIM_MEMORY => {
                        crate::utils::trim_working_set();
                    }
                    ID_MENU_EXIT => {
                        let _ = DestroyWindow(hwnd);
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        _ if msg == WM_TRAYICON => {
            let event = (lparam.0 & 0xFFFF) as u32;
            if event == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _hdc_screen = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            if w > 0 && h > 0 {
                let engine = get_d2d_engine();
                if let Some(state_arc) = APP_STATE.as_ref() {
                    if let Ok(mut s) = state_arc.lock() {
                        if s.layout_cache_dirty {
                            engine.clear_layout_cache();
                            s.layout_cache_dirty = false;
                        }
                    }
                }
                let mut needs_recreate = true;
                if let Some(ref cache) = SURFACE_CACHE {
                    if cache.width == w && cache.height == h {
                        needs_recreate = false;
                    }
                }
                if needs_recreate {
                    if let Some(mut old_cache) = SURFACE_CACHE.take() {
                        old_cache.release();
                    }
                    let mem_dc = CreateCompatibleDC(HDC::default());
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: w,
                            biHeight: -h,
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
                    let mut created_surface = None;
                    if let Ok(hbmp) =
                        CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                    {
                        let old_bmp = SelectObject(mem_dc, HGDIOBJ(hbmp.0));
                        if let Ok(dc_target) = engine.create_dc_render_target(mem_dc, &rect) {
                            if let Ok(target) = dc_target.cast::<ID2D1RenderTarget>() {
                                created_surface = Some(SurfaceCache {
                                    dc: mem_dc,
                                    hbmp,
                                    old_bmp,
                                    target,
                                    width: w,
                                    height: h,
                                });
                            }
                        }
                        if created_surface.is_none() {
                            let _ = SelectObject(mem_dc, old_bmp);
                            let _ = DeleteObject(HGDIOBJ(hbmp.0));
                            let _ = DeleteDC(mem_dc);
                        }
                    } else {
                        let _ = DeleteDC(mem_dc);
                    }
                    if let Some(cache) = created_surface {
                        SURFACE_CACHE = Some(cache);
                    }
                }
                if let Some(ref cache) = SURFACE_CACHE {
                    let _ = render_window_d2d(&cache.target, rect, engine);
                    let pt_src = POINT { x: 0, y: 0 };
                    let size = SIZE { cx: w, cy: h };
                    let blend = BLENDFUNCTION {
                        BlendOp: AC_SRC_OVER as u8,
                        BlendFlags: 0,
                        SourceConstantAlpha: 255,
                        AlphaFormat: AC_SRC_ALPHA as u8,
                    };
                    let _ = UpdateLayeredWindow(
                        hwnd,
                        HDC::default(),
                        None,
                        Some(&size),
                        cache.dc,
                        Some(&pt_src),
                        COLORREF(0),
                        Some(&blend),
                        ULW_ALPHA,
                    );
                }
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            if let Some(mut cache) = SURFACE_CACHE.take() {
                cache.release();
            }
            windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn create_main_window() -> HWND {
    unsafe {
        let instance = GetModuleHandleW(None).unwrap();
        let class_name: Vec<u16> = "MiniLyricWindow\0".encode_utf16().collect();
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassW(&wnd_class);
        let window_name: Vec<u16> = "Mini Lyric v2\0".encode_utf16().collect();
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_name.as_ptr()),
            WS_POPUP,
            100,
            100,
            560,
            200,
            None,
            None,
            instance,
            None,
        )
        .unwrap();
        let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, SW_SHOW);
        hwnd
    }
}

pub fn run_event_loop() {
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
