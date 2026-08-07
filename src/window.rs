#![allow(static_mut_refs)]
use crate::app_state::APP_STATE;
use crate::d2d_engine::get_d2d_engine;
use crate::render::render_window_d2d;
use crate::tray::{
    add_tray_icon, remove_tray_icon, show_tray_menu, toggle_lock_state, ID_MENU_EXIT, ID_MENU_LOCK,
    ID_MENU_OFFSET_MINUS, ID_MENU_OFFSET_PLUS, ID_MENU_OFFSET_RESET, ID_MENU_SIZE_LARGE,
    ID_MENU_SIZE_MEDIUM, ID_MENU_SIZE_SMALL, WM_TRAYICON,
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
    CS_HREDRAW, CS_VREDRAW, HTCAPTION, HTCLIENT, IDC_ARROW, MSG, SWP_NOMOVE, SWP_NOZORDER, SW_SHOW,
    ULW_ALPHA, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCHITTEST,
    WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
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
// Tracks what we last actually painted, so WM_TIMER can skip invalidating
// (and therefore skip the expensive UpdateLayeredWindow composite) when
// nothing on screen would actually change this tick.
static mut LAST_PAINTED_INDEX: usize = usize::MAX;

pub unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            // 33ms ≈ 30fps. Lyric overlay text doesn't need 60fps smoothness,
            // and this alone halves paint/GPU work on top of layout caching.
            SetTimer(hwnd, 1, 33, None);
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
                        // Was tuned for a 16ms tick; bumped up since we now
                        // tick at 33ms, so the scroll animation still
                        // converges at roughly the same real-world speed.
                        s.float_index += diff * 0.45;
                        true
                    } else {
                        s.float_index = target;
                        false
                    };

                    let active_playing = s.media.is_playing && !s.media.title.is_empty();

                    let index_changed = s.current_index != LAST_PAINTED_INDEX;

                    // Only the active line's *word-level* karaoke timing needs
                    // a fresh paint every tick (the fill color/pop animation
                    // progresses continuously). A plain single-syllable line
                    // just sitting on screen doesn't change frame to frame,
                    // so there's nothing worth compositing.
                    let active_line_is_karaoke = s
                        .lyrics_lines
                        .get(s.current_index)
                        .map(|line| line.syllables.len() > 1)
                        .unwrap_or(false);

                    should_invalidate = still_animating
                        || s.is_loading
                        || index_changed
                        || (active_playing && active_line_is_karaoke);

                    if should_invalidate {
                        LAST_PAINTED_INDEX = s.current_index;
                    }
                }
            }
            if should_invalidate {
                let _ = InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

            if x >= rect.right - 35 && x <= rect.right - 10 && (4..=24).contains(&y) {
                toggle_lock_state(hwnd);
            }
            LRESULT(0)
        }
        WM_TRAYICON => {
            let lparam_u32 = lparam.0 as u32;
            if lparam_u32 == WM_LBUTTONUP {
                toggle_lock_state(hwnd);
            } else if lparam_u32 == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as u32;
            if id == ID_MENU_LOCK {
                toggle_lock_state(hwnd);
            } else if id == ID_MENU_SIZE_SMALL {
                let _ = SetWindowPos(hwnd, None, 0, 0, 480, 160, SWP_NOMOVE | SWP_NOZORDER);
            } else if id == ID_MENU_SIZE_MEDIUM {
                let _ = SetWindowPos(hwnd, None, 0, 0, 560, 200, SWP_NOMOVE | SWP_NOZORDER);
            } else if id == ID_MENU_SIZE_LARGE {
                let _ = SetWindowPos(hwnd, None, 0, 0, 680, 240, SWP_NOMOVE | SWP_NOZORDER);
            } else if id == ID_MENU_OFFSET_PLUS {
                if let Some(state_arc) = APP_STATE.as_ref() {
                    if let Ok(mut s) = state_arc.lock() {
                        s.offset_ms += 100;
                    }
                }
            } else if id == ID_MENU_OFFSET_MINUS {
                if let Some(state_arc) = APP_STATE.as_ref() {
                    if let Ok(mut s) = state_arc.lock() {
                        s.offset_ms -= 100;
                    }
                }
            } else if id == ID_MENU_OFFSET_RESET {
                if let Some(state_arc) = APP_STATE.as_ref() {
                    if let Ok(mut s) = state_arc.lock() {
                        s.offset_ms = 0;
                    }
                }
            } else if id == ID_MENU_EXIT {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_NCHITTEST => {
            if let Some(state_arc) = APP_STATE.as_ref() {
                if let Ok(s) = state_arc.lock() {
                    if s.is_locked {
                        return DefWindowProcW(hwnd, msg, wparam, lparam);
                    }
                }
            }

            let screen_x = (lparam.0 & 0xFFFF) as i16 as i32;
            let screen_y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

            let mut client_point = POINT {
                x: screen_x,
                y: screen_y,
            };
            let _ = ScreenToClient(hwnd, &mut client_point);
            let mut client_rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut client_rect);

            if client_point.x >= client_rect.right - 40
                && client_point.x <= client_rect.right - 10
                && client_point.y >= 4
                && client_point.y <= 24
            {
                return LRESULT(HTCLIENT as isize);
            }

            if client_point.y <= 50
                && client_point.x >= 15
                && client_point.x <= client_rect.right - 45
            {
                return LRESULT(HTCAPTION as isize);
            }

            LRESULT(HTCLIENT as isize)
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

                // Track changed since last paint -> clear the cached
                // layouts here, on the UI thread, instead of from the
                // background fetch task (which isn't safe to touch D2D
                // from -- see AppState::layout_cache_dirty).
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
