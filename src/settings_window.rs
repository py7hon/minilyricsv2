#![allow(static_mut_refs)]
use crate::app_state::APP_STATE;
use crate::config::save_config;
use std::sync::Once;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
// BENAR buat windows 0.58
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::UI::Controls::Dialogs::{ChooseColorW, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW};
use windows::Win32::UI::WindowsAndMessaging::*;

const IDC_SAVE: usize = 300;
const IDC_CANCEL: usize = 301;
const IDC_COLOR_ACTIVE_BTN: usize = 302;
const IDC_COLOR_KARAOKE_BTN: usize = 303;

static mut SETTINGS_HWND: Option<HWND> = None;
static mut H_EDITS: [HWND; 9] = [HWND(0 as _); 9];
static mut H_COMBO_MODE: HWND = HWND(0 as _);
static mut H_COMBO_EFFECT: HWND = HWND(0 as _);
static mut H_CHECKS: [HWND; 3] = [HWND(0 as _); 3];
static REGISTER_ONCE: Once = Once::new();

const BST_UNCHECKED: isize = 0;
const BST_CHECKED: isize = 1;
const COLOR_WINDOW: u32 = 5;

const KARAOKE_MODES: &[&str] = &["auto", "always", "never"];
const KARAOKE_EFFECTS: &[&str] = &[
    "star_bounce",
    "pop",
    "pulse",
    "zoom",
    "wave",
    "bounce",
    "slide",
    "rise",
    "tilt",
    "stretch",
    "shake",
    "shimmer",
    "neon",
    "float",
    "fade",
    "sweep",
    "glow",
    "none",
];

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn hex_to_colorref(hex: &str) -> COLORREF {
    let h = hex.trim_start_matches('#');
    if h.len() >= 6 {
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(255) as u32;
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(255) as u32;
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(255) as u32;
        COLORREF((b << 16) | (g << 8) | r)
    } else {
        COLORREF(0xFFFFFF)
    }
}
fn colorref_to_hex(c: COLORREF) -> String {
    let r = (c.0 & 0xFF) as u8;
    let g = ((c.0 >> 8) & 0xFF) as u8;
    let b = ((c.0 >> 16) & 0xFF) as u8;
    format!("{:02x}{:02x}{:02x}", r, g, b)
}
unsafe fn pick_color(parent: HWND, initial_hex: &str) -> Option<String> {
    let mut custom_colors = [COLORREF(0); 16];
    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: parent,
        rgbResult: hex_to_colorref(initial_hex),
        lpCustColors: custom_colors.as_mut_ptr(),
        Flags: CC_FULLOPEN | CC_RGBINIT,
        ..Default::default()
    };
    if ChooseColorW(&mut cc).as_bool() {
        Some(colorref_to_hex(cc.rgbResult))
    } else {
        None
    }
}

pub fn open_settings_window() {
    unsafe {
        if let Some(h) = SETTINGS_HWND {
            if !h.0.is_null() && IsWindow(h).as_bool() {
                let _ = ShowWindow(h, SW_SHOW);
                let _ = SetForegroundWindow(h);
                return;
            }
        }
        REGISTER_ONCE.call_once(|| {
            let hinst = HINSTANCE(std::ptr::null_mut());
            let wc = WNDCLASSW {
                lpfnWndProc: Some(settings_wnd_proc),
                hInstance: hinst,
                lpszClassName: w!("MiniLyricSettingsClass"),
                hbrBackground: HBRUSH((COLOR_WINDOW as isize + 1) as *mut _),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);
        });
        let hinst = HINSTANCE(std::ptr::null_mut());
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("MiniLyricSettingsClass"),
            w!("Mini Lyric - Settings Pro"),
            WS_OVERLAPPEDWINDOW & !WS_THICKFRAME & !WS_MAXIMIZEBOX,
            200,
            200,
            520,
            650,
            None,
            HMENU(0 as _),
            hinst,
            None,
        )
        .unwrap_or(HWND(0 as _));
        if hwnd.0.is_null() {
            return;
        }
        SETTINGS_HWND = Some(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOW);
    }
}

#[allow(clippy::unnecessary_cast)]
unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            create_controls(hwnd);
            load_values();
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as usize;
            let notif = ((wparam.0 >> 16) & 0xFFFF) as u32;
            if notif == BN_CLICKED as u32 {
                match id {
                    IDC_SAVE => {
                        if save_values() {
                            MessageBoxW(
                                hwnd,
                                w!("Settings saved!"),
                                w!("Saved"),
                                MB_OK | MB_ICONINFORMATION,
                            );
                            let _ = DestroyWindow(hwnd);
                        }
                    }
                    IDC_CANCEL => {
                        let _ = DestroyWindow(hwnd);
                    }
                    IDC_COLOR_ACTIVE_BTN => {
                        let cur = get_text(H_EDITS[7]);
                        if let Some(new_hex) = pick_color(hwnd, &cur) {
                            let _ = SetWindowTextW(H_EDITS[7], PCWSTR(to_wide(&new_hex).as_ptr()));
                        }
                    }
                    IDC_COLOR_KARAOKE_BTN => {
                        let cur = get_text(H_EDITS[8]);
                        if let Some(new_hex) = pick_color(hwnd, &cur) {
                            let _ = SetWindowTextW(H_EDITS[8], PCWSTR(to_wide(&new_hex).as_ptr()));
                        }
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            SETTINGS_HWND = None;
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn create_controls(parent: HWND) {
    let hinst = HINSTANCE(std::ptr::null_mut());
    let labels = [
        ("Font Family:", 15, 15),
        ("Font Active Size:", 15, 70),
        ("Font Side Size:", 15, 125),
        ("Font Title Size:", 15, 180),
        ("Opacity:", 15, 235),
        ("Active Hex:", 15, 290),
        ("Karaoke Hex:", 15, 345),
    ];
    for (i, (text, x, y)) in labels.iter().enumerate() {
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            PCWSTR(to_wide(text).as_ptr()),
            WS_CHILD | WS_VISIBLE,
            *x,
            *y,
            220,
            16,
            parent,
            HMENU(0 as _),
            hinst,
            None,
        );
        let ey = y + 18;
        let hed = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("EDIT"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_BORDER,
            *x,
            ey,
            140,
            24,
            parent,
            HMENU(0 as _),
            hinst,
            None,
        )
        .unwrap_or(HWND(0 as _));
        let map_idx = match i {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 6,
            5 => 7,
            6 => 8,
            _ => 0,
        };
        if i < 7 {
            H_EDITS[map_idx] = hed;
        }
    }
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        w!("Pick..."),
        WS_CHILD | WS_VISIBLE,
        160,
        308,
        60,
        24,
        parent,
        HMENU(IDC_COLOR_ACTIVE_BTN as _),
        hinst,
        None,
    );
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        w!("Pick..."),
        WS_CHILD | WS_VISIBLE,
        160,
        363,
        60,
        24,
        parent,
        HMENU(IDC_COLOR_KARAOKE_BTN as _),
        hinst,
        None,
    );

    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("STATIC"),
        w!("Karaoke Mode:"),
        WS_CHILD | WS_VISIBLE,
        250,
        15,
        200,
        16,
        parent,
        HMENU(0 as _),
        hinst,
        None,
    );
    H_COMBO_MODE = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("COMBOBOX"),
        w!(""),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        250,
        33,
        200,
        120,
        parent,
        HMENU(0 as _),
        hinst,
        None,
    )
    .unwrap_or(HWND(0 as _));
    for mode in KARAOKE_MODES {
        let w = to_wide(mode);
        SendMessageW(
            H_COMBO_MODE,
            CB_ADDSTRING,
            WPARAM(0),
            LPARAM(w.as_ptr() as isize),
        );
    }

    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("STATIC"),
        w!("Karaoke Effect:"),
        WS_CHILD | WS_VISIBLE,
        250,
        70,
        200,
        16,
        parent,
        HMENU(0 as _),
        hinst,
        None,
    );
    H_COMBO_EFFECT = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("COMBOBOX"),
        w!(""),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        250,
        88,
        200,
        300,
        parent,
        HMENU(0 as _),
        hinst,
        None,
    )
    .unwrap_or(HWND(0 as _));
    for eff in KARAOKE_EFFECTS {
        let w = to_wide(eff);
        SendMessageW(
            H_COMBO_EFFECT,
            CB_ADDSTRING,
            WPARAM(0),
            LPARAM(w.as_ptr() as isize),
        );
    }

    let checks = [
        ("Show Card Background", 250, 135),
        ("Enable Shadow", 250, 160),
        ("Auto Trim Memory", 250, 185),
    ];
    for (i, (txt, x, y)) in checks.iter().enumerate() {
        let h = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            PCWSTR(to_wide(txt).as_ptr()),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            *x,
            *y,
            200,
            22,
            parent,
            HMENU(0 as _),
            hinst,
            None,
        )
        .unwrap_or(HWND(0 as _));
        H_CHECKS[i] = h;
    }
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        w!("Save"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        15,
        530,
        220,
        36,
        parent,
        HMENU(IDC_SAVE as _),
        hinst,
        None,
    );
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        w!("Cancel"),
        WS_CHILD | WS_VISIBLE,
        250,
        530,
        200,
        36,
        parent,
        HMENU(IDC_CANCEL as _),
        hinst,
        None,
    );
}

unsafe fn get_text(h: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(h, &mut buf);
    String::from_utf16_lossy(&buf[..len as usize])
}
unsafe fn set_check(h: HWND, checked: bool) {
    SendMessageW(
        h,
        BM_SETCHECK,
        WPARAM(if checked {
            BST_CHECKED as usize
        } else {
            BST_UNCHECKED as usize
        }),
        LPARAM(0),
    );
}
unsafe fn get_check(h: HWND) -> bool {
    SendMessageW(h, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == BST_CHECKED
}
unsafe fn get_combo_text(h: HWND) -> String {
    let idx = SendMessageW(h, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
    if idx < 0 {
        return String::new();
    }
    let mut buf = [0u16; 128];
    SendMessageW(
        h,
        CB_GETLBTEXT,
        WPARAM(idx as usize),
        LPARAM(buf.as_mut_ptr() as isize),
    );
    let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
    String::from_utf16_lossy(&buf[..len])
}
unsafe fn set_combo_by_text(h: HWND, text: &str) {
    let w = to_wide(text);
    let idx = SendMessageW(
        h,
        CB_FINDSTRINGEXACT,
        WPARAM(0),
        LPARAM(w.as_ptr() as isize),
    )
    .0;
    if idx >= 0 {
        SendMessageW(h, CB_SETCURSEL, WPARAM(idx as usize), LPARAM(0));
    }
}
unsafe fn load_values() {
    let Some(state_arc) = APP_STATE.as_ref() else {
        return;
    };
    let Ok(s) = state_arc.lock() else { return };
    let c = &s.config;
    let _ = SetWindowTextW(H_EDITS[0], PCWSTR(to_wide(&c.font_family).as_ptr()));
    let _ = SetWindowTextW(
        H_EDITS[1],
        PCWSTR(to_wide(&c.font_size_active.to_string()).as_ptr()),
    );
    let _ = SetWindowTextW(
        H_EDITS[2],
        PCWSTR(to_wide(&c.font_size_side.to_string()).as_ptr()),
    );
    let _ = SetWindowTextW(
        H_EDITS[3],
        PCWSTR(to_wide(&c.font_size_title.to_string()).as_ptr()),
    );
    let _ = SetWindowTextW(H_EDITS[6], PCWSTR(to_wide(&c.opacity.to_string()).as_ptr()));
    let _ = SetWindowTextW(H_EDITS[7], PCWSTR(to_wide(&c.active_hex).as_ptr()));
    let _ = SetWindowTextW(H_EDITS[8], PCWSTR(to_wide(&c.karaoke_hex).as_ptr()));
    set_combo_by_text(H_COMBO_MODE, &c.karaoke_mode);
    set_combo_by_text(H_COMBO_EFFECT, &c.karaoke_effect);
    if !H_CHECKS[0].0.is_null() {
        set_check(H_CHECKS[0], c.show_card.unwrap_or(false));
    }
    if !H_CHECKS[1].0.is_null() {
        set_check(H_CHECKS[1], c.shadow_enabled);
    }
    if !H_CHECKS[2].0.is_null() {
        set_check(H_CHECKS[2], c.auto_trim_memory);
    }
}
unsafe fn save_values() -> bool {
    let Some(state_arc) = APP_STATE.as_ref() else {
        return false;
    };
    let Ok(mut s) = state_arc.lock() else {
        return false;
    };
    let font_family = get_text(H_EDITS[0]);
    let font_active: i32 = get_text(H_EDITS[1])
        .parse()
        .unwrap_or(s.config.font_size_active);
    let font_side: i32 = get_text(H_EDITS[2])
        .parse()
        .unwrap_or(s.config.font_size_side);
    let font_title: i32 = get_text(H_EDITS[3])
        .parse()
        .unwrap_or(s.config.font_size_title);
    let opacity: f32 = get_text(H_EDITS[6])
        .parse::<f32>()
        .unwrap_or(s.config.opacity)
        .clamp(0.1, 1.0);
    let active_hex = get_text(H_EDITS[7]);
    let karaoke_hex = get_text(H_EDITS[8]);
    let mode = get_combo_text(H_COMBO_MODE);
    let effect = get_combo_text(H_COMBO_EFFECT);
    if !font_family.is_empty() {
        s.config.font_family = font_family;
    }
    s.config.font_size_active = font_active.clamp(8, 80);
    s.config.font_size_side = font_side.clamp(8, 40);
    s.config.font_size_title = font_title.clamp(8, 50);
    if !mode.is_empty() {
        s.config.karaoke_mode = mode.to_lowercase();
    }
    if !effect.is_empty() {
        s.config.karaoke_effect = effect.to_lowercase();
    }
    s.config.opacity = opacity;
    if !active_hex.is_empty() {
        s.config.active_hex = active_hex.trim_start_matches('#').to_string();
    }
    if !karaoke_hex.is_empty() {
        s.config.karaoke_hex = karaoke_hex.trim_start_matches('#').to_string();
    }
    s.config.show_card = Some(get_check(H_CHECKS[0]));
    s.config.shadow_enabled = get_check(H_CHECKS[1]);
    s.config.auto_trim_memory = get_check(H_CHECKS[2]);
    s.layout_cache_dirty = true;
    save_config(&s.config);
    true
}
