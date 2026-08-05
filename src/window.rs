#![allow(static_mut_refs)]
use crate::app_state::{APP_STATE, PAINT_CACHE};
use crate::render::render_window;
use crate::tray::{
    add_tray_icon, remove_tray_icon, show_tray_menu, toggle_lock_state, ID_MENU_EXIT, ID_MENU_LOCK,
    ID_MENU_OFFSET_MINUS, ID_MENU_OFFSET_PLUS, ID_MENU_OFFSET_RESET, ID_MENU_SIZE_LARGE,
    ID_MENU_SIZE_MEDIUM, ID_MENU_SIZE_SMALL, WM_TRAYICON,
};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    EndPaint, ScreenToClient, SelectObject, SetGraphicsMode, GM_ADVANCED, HGDIOBJ, PAINTSTRUCT,
    SRCCOPY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    LoadCursorW, RegisterClassW, SetLayeredWindowAttributes, SetTimer, SetWindowPos,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, HTCAPTION, HTCLIENT, IDC_ARROW, LWA_COLORKEY, MSG,
    SWP_NOMOVE, SWP_NOZORDER, SW_SHOW, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_NCHITTEST, WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

pub unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
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
                        s.float_index += diff * 0.25;
                        true
                    } else {
                        s.float_index = target;
                        false
                    };

                    let active_playing = s.media.is_playing && !s.media.title.is_empty();
                    should_invalidate = still_animating || s.is_loading || active_playing;
                }
            }
            if should_invalidate {
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(hwnd, None, false);
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
                        s.offset_ms += 500;
                    }
                }
            } else if id == ID_MENU_OFFSET_MINUS {
                if let Some(state_arc) = APP_STATE.as_ref() {
                    if let Ok(mut s) = state_arc.lock() {
                        s.offset_ms -= 500;
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
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);

            let (mem_dc, mem_bmp) = {
                let cached = PAINT_CACHE;
                match cached {
                    Some((dc, bmp, w, h)) if w == rect.right && h == rect.bottom => (dc, bmp),
                    _ => {
                        if let Some((old_dc, old_bmp, _, _)) = PAINT_CACHE.take() {
                            let _ = DeleteObject(HGDIOBJ(old_bmp.0));
                            let _ = DeleteDC(old_dc);
                        }
                        let dc = CreateCompatibleDC(hdc);
                        let bmp = CreateCompatibleBitmap(hdc, rect.right, rect.bottom);
                        PAINT_CACHE = Some((dc, bmp, rect.right, rect.bottom));
                        (dc, bmp)
                    }
                }
            };
            let old_bmp = SelectObject(mem_dc, HGDIOBJ(mem_bmp.0));

            SetGraphicsMode(mem_dc, GM_ADVANCED);

            render_window(mem_dc, rect);

            let _ = BitBlt(hdc, 0, 0, rect.right, rect.bottom, mem_dc, 0, 0, SRCCOPY);
            SelectObject(mem_dc, old_bmp);

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            if let Some((dc, bmp, _, _)) = PAINT_CACHE.take() {
                let _ = DeleteObject(HGDIOBJ(bmp.0));
                let _ = DeleteDC(dc);
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

        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0x000000), 255, LWA_COLORKEY);
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
