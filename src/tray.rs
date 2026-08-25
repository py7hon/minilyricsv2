#![allow(static_mut_refs)]
use crate::app_state::APP_STATE;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::InvalidateRect;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, GetWindowLongPtrW, LoadIconW,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, TrackPopupMenu, GWL_EXSTYLE,
    IDI_INFORMATION, MF_SEPARATOR, MF_STRING, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WS_EX_TRANSPARENT,
};
pub const WM_TRAYICON: u32 = 0x0400 + 1;
pub const ID_TRAY_ICON: u32 = 1001;
pub const ID_MENU_LOCK: u32 = 2001;
pub const ID_MENU_SIZE_SMALL: u32 = 2003;
pub const ID_MENU_SIZE_MEDIUM: u32 = 2004;
pub const ID_MENU_SIZE_LARGE: u32 = 2005;
pub const ID_MENU_OFFSET_PLUS: u32 = 2006;
pub const ID_MENU_OFFSET_MINUS: u32 = 2007;
pub const ID_MENU_OFFSET_RESET: u32 = 2008;
pub const ID_MENU_TRIM_MEMORY: u32 = 2009;
pub const ID_MENU_SETTINGS: u32 = 2010;
pub const ID_MENU_CHECK_UPDATE: u32 = 2011;
pub const ID_MENU_EXIT: u32 = 2002;

pub fn add_tray_icon(hwnd: HWND) {
    unsafe {
        let tip: Vec<u16> = "Mini Lyric v2 (Right-click for Menu)\0"
            .encode_utf16()
            .collect();
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: ID_TRAY_ICON,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: LoadIconW(None, IDI_INFORMATION).unwrap_or_default(),
            ..Default::default()
        };
        nid.szTip[..tip.len()].copy_from_slice(&tip);
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}
pub fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: ID_TRAY_ICON,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}
pub fn toggle_lock_state(hwnd: HWND) {
    let state_ref = unsafe { APP_STATE.as_ref() };
    if let Some(state_arc) = state_ref {
        if let Ok(mut s) = state_arc.lock() {
            s.is_locked = !s.is_locked;
            unsafe {
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let new_ex_style = if s.is_locked {
                    ex_style | WS_EX_TRANSPARENT.0 as isize
                } else {
                    ex_style & !(WS_EX_TRANSPARENT.0 as isize)
                };
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                );
                let _ = InvalidateRect(hwnd, None, false);
            }
        }
    }
}
pub fn show_tray_menu(hwnd: HWND) {
    unsafe {
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let hmenu = CreatePopupMenu().unwrap();
        let (lock_text, current_offset) = if let Some(state_arc) = APP_STATE.as_ref() {
            if let Ok(s) = state_arc.lock() {
                let l = if s.is_locked {
                    "Unlock Position (Draggable)\0"
                } else {
                    "Lock Position (Click-Through)\0"
                };
                (l, s.offset_ms)
            } else {
                ("Toggle Lock\0", 0)
            }
        } else {
            ("Toggle Lock\0", 0)
        };
        let lock_w: Vec<u16> = lock_text.encode_utf16().collect();
        let small_w: Vec<u16> = "Size: Compact (480x160)\0".encode_utf16().collect();
        let med_w: Vec<u16> = "Size: Normal (560x200)\0".encode_utf16().collect();
        let large_w: Vec<u16> = "Size: Large (680x240)\0".encode_utf16().collect();
        let offset_secs = current_offset as f32 / 1000.0;
        let offset_plus_w: Vec<u16> =
            format!("Sync: Faster (+100ms) [Current: {:.1}s]\0", offset_secs)
                .encode_utf16()
                .collect();
        let offset_minus_w: Vec<u16> = "Sync: Slower (-100ms)\0".encode_utf16().collect();
        let offset_reset_w: Vec<u16> = "Sync: Reset Offset (0.0s)\0".encode_utf16().collect();
        let trim_mem_w: Vec<u16> = "Trim Memory (Release RAM)\0".encode_utf16().collect();
        let settings_w: Vec<u16> = "Settings...\0".encode_utf16().collect();
        let update_w: Vec<u16> = "Check for Updates...\0".encode_utf16().collect();
        let exit_w: Vec<u16> = "Exit Mini Lyric\0".encode_utf16().collect();
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_LOCK as usize,
            PCWSTR(lock_w.as_ptr()),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_SIZE_SMALL as usize,
            PCWSTR(small_w.as_ptr()),
        );
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_SIZE_MEDIUM as usize,
            PCWSTR(med_w.as_ptr()),
        );
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_SIZE_LARGE as usize,
            PCWSTR(large_w.as_ptr()),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_OFFSET_PLUS as usize,
            PCWSTR(offset_plus_w.as_ptr()),
        );
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_OFFSET_MINUS as usize,
            PCWSTR(offset_minus_w.as_ptr()),
        );
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_OFFSET_RESET as usize,
            PCWSTR(offset_reset_w.as_ptr()),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_TRIM_MEMORY as usize,
            PCWSTR(trim_mem_w.as_ptr()),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_SETTINGS as usize,
            PCWSTR(settings_w.as_ptr()),
        );
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_CHECK_UPDATE as usize,
            PCWSTR(update_w.as_ptr()),
        );
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            ID_MENU_EXIT as usize,
            PCWSTR(exit_w.as_ptr()),
        );
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            hmenu,
            TPM_BOTTOMALIGN | TPM_LEFTALIGN,
            p.x,
            p.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(hmenu);
    }
}
