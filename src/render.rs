use crate::app_state::APP_STATE;
use crate::config::hex_to_colorref;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, FillRect,
    GetTextExtentPoint32W, ModifyWorldTransform, RoundRect, SelectObject, SetBkMode, SetTextColor,
    SetWorldTransform, ANTIALIASED_QUALITY, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DEFAULT_PITCH, DRAW_TEXT_FORMAT, DT_LEFT, DT_NOCLIP, DT_NOPREFIX, DT_RIGHT,
    DT_SINGLELINE, DT_WORDBREAK, DT_WORD_ELLIPSIS, FF_DONTCARE, FW_BOLD, FW_NORMAL, HDC, HGDIOBJ,
    MWT_IDENTITY, OUT_DEFAULT_PRECIS, PS_SOLID, TRANSPARENT, XFORM,
};

pub fn get_font_face_for_text(text: &str, default_font: &str) -> String {
    let has_cjk = text.chars().any(|c| {
        let u = c as u32;
        (u >= 0x4E00 && u <= 0x9FFF)
            || (u >= 0x3040 && u <= 0x309F)
            || (u >= 0x30A0 && u <= 0x30FF)
            || (u >= 0xAC00 && u <= 0xD7AF)
    });

    if has_cjk {
        "Noto Sans JP".to_string()
    } else {
        default_font.to_string()
    }
}

pub unsafe fn draw_text_outlined(
    hdc: HDC,
    text: &[u16],
    rect: RECT,
    format: DRAW_TEXT_FORMAT,
    color: COLORREF,
    _is_small_font: bool,
) {
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, color);
    let mut front = text.to_vec();
    let mut r = rect;
    let _ = DrawTextW(hdc, &mut front, &mut r, format);
}

struct RenderSyl {
    x: i32,
    y: i32,
    width: i32,
    progress: f32,
    text_utf16: Vec<u16>,
}

pub unsafe fn render_window(mem_dc: HDC, rect: RECT) {
    let bg_brush = CreateSolidBrush(COLORREF(0x000000));
    FillRect(mem_dc, &rect, bg_brush);
    let _ = DeleteObject(HGDIOBJ(bg_brush.0));

    if let Some(state_arc) = APP_STATE.as_ref() {
        if let Ok(s) = state_arc.lock() {
            let cfg = &s.config;
            if cfg.show_card.unwrap_or(false) {
                let card_color = hex_to_colorref(cfg.card_bg_hex.as_deref().unwrap_or("12121a"));
                let card_brush = CreateSolidBrush(card_color);
                let border_pen = CreatePen(PS_SOLID, 1, COLORREF(0x262636));
                let old_pen = SelectObject(mem_dc, HGDIOBJ(border_pen.0));
                let old_brush = SelectObject(mem_dc, HGDIOBJ(card_brush.0));

                let _ = RoundRect(mem_dc, 2, 2, rect.right - 2, rect.bottom - 2, 16, 16);

                SelectObject(mem_dc, old_pen);
                SelectObject(mem_dc, old_brush);
                let _ = DeleteObject(HGDIOBJ(border_pen.0));
                let _ = DeleteObject(HGDIOBJ(card_brush.0));
            }
        }
    }

    if let Some(state_arc) = APP_STATE.as_ref() {
        if let Ok(s) = state_arc.lock() {
            SetBkMode(mem_dc, TRANSPARENT);
            let cfg = &s.config;

            let scale = (rect.bottom as f32 / 200.0).max(0.6);
            let base_center_y = cfg.base_center_y * scale;
            let base_step = cfg.line_spacing * scale;

            let mut cursor_y = (8.0 * scale) as i32;
            let default_face_w: Vec<u16> =
                format!("{}\0", cfg.font_family).encode_utf16().collect();

            let lock_status = if s.is_locked { "🔒" } else { "🔓" };
            let mut lock_utf16: Vec<u16> = format!("{}\0", lock_status).encode_utf16().collect();
            let font_lock = CreateFontW(
                (11.0 * scale) as i32,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                PCWSTR(default_face_w.as_ptr()),
            );
            let old_font_lock = SelectObject(mem_dc, HGDIOBJ(font_lock.0));
            SetTextColor(mem_dc, COLORREF(0xAAAAAA));
            let mut lock_rect = RECT {
                left: rect.right - 35,
                top: 2,
                right: rect.right - 10,
                bottom: 20,
            };
            let _ = DrawTextW(
                mem_dc,
                &mut lock_utf16,
                &mut lock_rect,
                DT_RIGHT | DT_SINGLELINE,
            );
            SelectObject(mem_dc, old_font_lock);
            let _ = DeleteObject(HGDIOBJ(font_lock.0));

            if !s.media.title.is_empty() {
                let title_utf16: Vec<u16> = format!("{}\0", s.media.title).encode_utf16().collect();
                let artist_utf16: Vec<u16> =
                    format!("{}\0", s.media.artist).encode_utf16().collect();

                let font_size_title_capped = cfg.font_size_title.min(40);
                let resolved_title_font = get_font_face_for_text(&s.media.title, &cfg.font_family);
                let title_face_w: Vec<u16> = format!("{}\0", resolved_title_font)
                    .encode_utf16()
                    .collect();

                let font_title = CreateFontW(
                    font_size_title_capped,
                    0,
                    0,
                    0,
                    FW_BOLD.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    ANTIALIASED_QUALITY.0 as u32,
                    (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                    PCWSTR(title_face_w.as_ptr()),
                );
                let old_font_title = SelectObject(mem_dc, HGDIOBJ(font_title.0));
                let title_rect = RECT {
                    left: 15,
                    top: cursor_y,
                    right: rect.right - 45,
                    bottom: cursor_y + font_size_title_capped + 10,
                };
                draw_text_outlined(
                    mem_dc,
                    &title_utf16,
                    title_rect,
                    DT_LEFT | DT_SINGLELINE | DT_WORD_ELLIPSIS,
                    hex_to_colorref(&cfg.title_hex),
                    false,
                );
                cursor_y += font_size_title_capped + 2;
                SelectObject(mem_dc, old_font_title);
                let _ = DeleteObject(HGDIOBJ(font_title.0));

                let font_size_artist_capped = cfg.font_size_artist.min(40);
                let resolved_artist_font =
                    get_font_face_for_text(&s.media.artist, &cfg.font_family);
                let artist_face_w: Vec<u16> = format!("{}\0", resolved_artist_font)
                    .encode_utf16()
                    .collect();

                let font_artist = CreateFontW(
                    font_size_artist_capped,
                    0,
                    0,
                    0,
                    FW_NORMAL.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    ANTIALIASED_QUALITY.0 as u32,
                    (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                    PCWSTR(artist_face_w.as_ptr()),
                );
                let old_font_artist = SelectObject(mem_dc, HGDIOBJ(font_artist.0));
                let artist_rect = RECT {
                    left: 15,
                    top: cursor_y,
                    right: rect.right - 45,
                    bottom: cursor_y + font_size_artist_capped + 10,
                };
                draw_text_outlined(
                    mem_dc,
                    &artist_utf16,
                    artist_rect,
                    DT_LEFT | DT_SINGLELINE | DT_WORD_ELLIPSIS,
                    hex_to_colorref(&cfg.artist_hex),
                    true,
                );
                SelectObject(mem_dc, old_font_artist);
                let _ = DeleteObject(HGDIOBJ(font_artist.0));
            }

            let header_bottom = cursor_y + 8;

            let real_pos_ms = if s.media.is_playing {
                s.last_pos_ms + s.last_pos_update.elapsed().as_millis() as u64
            } else {
                s.last_pos_ms
            };
            let adjusted_ms = (real_pos_ms as i64 + s.offset_ms).max(0) as u64;
            let float_idx = s.float_index;

            let active_font_size = cfg.font_size_active.min(40);
            let active_karaoke_color = hex_to_colorref(&cfg.karaoke_hex);
            let active_text_color = hex_to_colorref(&cfg.active_hex);
            let line_font_family = cfg.font_family.clone();

            let max_w = rect.right - 30;
            let mut active_h = active_font_size + 4;
            let current_idx = s.current_index;

            let temp_resolved_font = if current_idx < s.lyrics_lines.len() {
                get_font_face_for_text(&s.lyrics_lines[current_idx].text, &line_font_family)
            } else {
                line_font_family.clone()
            };
            let temp_face_w: Vec<u16> =
                format!("{}\0", temp_resolved_font).encode_utf16().collect();
            let font_active_measure = CreateFontW(
                active_font_size,
                0,
                0,
                0,
                FW_BOLD.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                PCWSTR(temp_face_w.as_ptr()),
            );
            let old_measure_font = SelectObject(mem_dc, HGDIOBJ(font_active_measure.0));

            if current_idx < s.lyrics_lines.len() {
                let active_line = &s.lyrics_lines[current_idx];
                let mut cx = 15;
                let mut clines = 1;
                let lh = active_font_size + 4;

                for syl in &active_line.syllables {
                    let w_utf16_measure: Vec<u16> = syl.text.encode_utf16().collect();
                    let mut size = SIZE::default();
                    let _ = GetTextExtentPoint32W(mem_dc, &w_utf16_measure, &mut size);

                    let padding = if syl.text.ends_with(' ') { 0 } else { 2 };
                    let syl_width = size.cx + padding;

                    if cx + syl_width > max_w && cx > 15 {
                        cx = 15;
                        clines += 1;
                    }
                    cx += syl_width;
                }
                active_h = clines * lh;
                if active_line
                    .sub_text
                    .as_ref()
                    .map_or(false, |sub| !sub.is_empty())
                {
                    active_h += (cfg.font_size_sub.min(40) * 2) + 16;
                }
            }

            SelectObject(mem_dc, old_measure_font);
            let _ = DeleteObject(HGDIOBJ(font_active_measure.0));

            if s.is_loading {
                let mut loading_utf16: Vec<u16> = "Loading lyrics...\0".encode_utf16().collect();
                let font = CreateFontW(
                    12,
                    0,
                    0,
                    0,
                    FW_NORMAL.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    CLEARTYPE_QUALITY.0 as u32,
                    (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                    PCWSTR(default_face_w.as_ptr()),
                );
                let old_font = SelectObject(mem_dc, HGDIOBJ(font.0));
                SetTextColor(mem_dc, COLORREF(0x888888));
                let mut r = RECT {
                    left: 15,
                    top: base_center_y as i32,
                    right: rect.right - 15,
                    bottom: base_center_y as i32 + 30,
                };
                let _ = DrawTextW(mem_dc, &mut loading_utf16, &mut r, DT_LEFT | DT_SINGLELINE);
                SelectObject(mem_dc, old_font);
                let _ = DeleteObject(HGDIOBJ(font.0));
            } else if !s.lyrics_lines.is_empty() {
                for offset in -1isize..=1 {
                    let target_idx = (current_idx as isize) + offset;
                    if target_idx >= 0 && target_idx < s.lyrics_lines.len() as isize {
                        let line = &s.lyrics_lines[target_idx as usize];
                        let distance_from_float = (target_idx as f32) - float_idx;

                        let line_top = if offset <= 0 {
                            (base_center_y + distance_from_float * base_step) as i32
                        } else {
                            (base_center_y
                                + active_h as f32
                                + 12.0
                                + (distance_from_float - 1.0) * base_step)
                                as i32
                        };

                        if line_top < header_bottom {
                            continue;
                        }

                        let is_active = offset == 0;
                        let is_instrumental = line.text.trim() == "♪"
                            || line.text.to_lowercase().contains("instrumental");

                        let resolved_font_face =
                            get_font_face_for_text(&line.text, &line_font_family);
                        let line_face_w: Vec<u16> =
                            format!("{}\0", resolved_font_face).encode_utf16().collect();

                        if is_active {
                            let font_base = CreateFontW(
                                active_font_size,
                                0,
                                0,
                                0,
                                FW_BOLD.0 as i32,
                                0,
                                0,
                                0,
                                DEFAULT_CHARSET.0 as u32,
                                OUT_DEFAULT_PRECIS.0 as u32,
                                CLIP_DEFAULT_PRECIS.0 as u32,
                                CLEARTYPE_QUALITY.0 as u32,
                                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                                PCWSTR(line_face_w.as_ptr()),
                            );
                            let old_font_base = SelectObject(mem_dc, HGDIOBJ(font_base.0));

                            if is_instrumental {
                                let mut note_utf16: Vec<u16> = "♪\0".encode_utf16().collect();
                                let mut note_rect = RECT {
                                    left: 15,
                                    top: line_top,
                                    right: rect.right - 15,
                                    bottom: line_top + active_font_size + 20,
                                };
                                SetTextColor(mem_dc, COLORREF(0x888888));
                                let _ = DrawTextW(
                                    mem_dc,
                                    &mut note_utf16,
                                    &mut note_rect,
                                    DT_LEFT | DT_SINGLELINE | DT_NOPREFIX | DT_NOCLIP,
                                );
                            } else {
                                let start_ms = line.time.as_millis() as u64;
                                let elapsed_line = adjusted_ms.saturating_sub(start_ms);

                                let mut current_x = 15;
                                let mut current_y = line_top;
                                let line_height = active_font_size + 4;
                                let mut accumulated_syl_time = 0u64;

                                let mut render_data = Vec::new();

                                for syl in &line.syllables {
                                    let syl_start = accumulated_syl_time;
                                    let syl_dur = syl.duration.as_millis().max(50) as u64;
                                    accumulated_syl_time += syl_dur;

                                    let syl_progress = if elapsed_line >= syl_start {
                                        ((elapsed_line - syl_start) as f32 / syl_dur as f32)
                                            .clamp(0.0, 1.0)
                                    } else {
                                        0.0
                                    };

                                    let w_utf16_measure: Vec<u16> =
                                        syl.text.encode_utf16().collect();
                                    let mut size = SIZE::default();
                                    let _ =
                                        GetTextExtentPoint32W(mem_dc, &w_utf16_measure, &mut size);

                                    let padding = if syl.text.ends_with(' ') { 0 } else { 2 };
                                    let syl_width = size.cx + padding;

                                    if current_x + syl_width > max_w && current_x > 15 {
                                        current_x = 15;
                                        current_y += line_height;
                                    }

                                    render_data.push(RenderSyl {
                                        x: current_x,
                                        y: current_y,
                                        width: syl_width,
                                        progress: syl_progress,
                                        text_utf16: w_utf16_measure,
                                    });

                                    current_x += syl_width;
                                }

                                let total_box_height = (current_y - line_top) + line_height;

                                for rs in &render_data {
                                    let is_current_syl = rs.progress > 0.0 && rs.progress < 1.0;
                                    if is_current_syl {
                                        continue;
                                    }

                                    let syl_rect = RECT {
                                        left: rs.x,
                                        top: rs.y,
                                        right: rs.x + rs.width,
                                        bottom: rs.y + line_height,
                                    };
                                    let syl_color = if rs.progress >= 1.0 {
                                        active_karaoke_color
                                    } else {
                                        active_text_color
                                    };
                                    draw_text_outlined(
                                        mem_dc,
                                        &rs.text_utf16,
                                        syl_rect,
                                        DT_LEFT | DT_SINGLELINE | DT_NOPREFIX | DT_NOCLIP,
                                        syl_color,
                                        false,
                                    );
                                }

                                for rs in &render_data {
                                    let is_current_syl = rs.progress > 0.0 && rs.progress < 1.0;
                                    if !is_current_syl {
                                        continue;
                                    }

                                    let pop_factor = if rs.progress < 0.25 {
                                        let t = rs.progress / 0.25;
                                        (t * std::f32::consts::FRAC_PI_2).sin()
                                    } else {
                                        let t = (rs.progress - 0.25) / 0.75;
                                        (1.0 - t).powi(3)
                                    };

                                    let scale_val = 1.0 + (pop_factor * 0.08);
                                    let y_shift = -(pop_factor * 2.0);

                                    let cx = rs.x as f32 + (rs.width as f32 / 2.0);
                                    let cy = rs.y as f32 + (line_height as f32 / 2.0);

                                    let xform = XFORM {
                                        eM11: scale_val,
                                        eM12: 0.0,
                                        eM21: 0.0,
                                        eM22: scale_val,
                                        eDx: cx - (cx * scale_val),
                                        eDy: cy - (cy * scale_val) + y_shift,
                                    };

                                    let _ = SetWorldTransform(mem_dc, &xform);

                                    let mut syl_rect = RECT {
                                        left: rs.x,
                                        top: rs.y,
                                        right: rs.x + rs.width,
                                        bottom: rs.y + line_height,
                                    };

                                    SetTextColor(mem_dc, active_karaoke_color);
                                    let mut active_text = rs.text_utf16.clone();
                                    let _ = DrawTextW(
                                        mem_dc,
                                        &mut active_text,
                                        &mut syl_rect,
                                        DT_LEFT | DT_SINGLELINE | DT_NOPREFIX | DT_NOCLIP,
                                    );

                                    let _ = ModifyWorldTransform(mem_dc, None, MWT_IDENTITY);
                                }

                                SelectObject(mem_dc, old_font_base);

                                if let Some(ref sub) = line.sub_text {
                                    if !sub.is_empty() {
                                        let sub_utf16: Vec<u16> =
                                            format!("{}\0", sub).encode_utf16().collect();
                                        let font_size_sub_capped = cfg.font_size_sub.min(40);

                                        let resolved_sub_font =
                                            get_font_face_for_text(sub, &cfg.font_family);
                                        let sub_face_w: Vec<u16> =
                                            format!("{}\0", resolved_sub_font)
                                                .encode_utf16()
                                                .collect();

                                        let font_sub = CreateFontW(
                                            font_size_sub_capped,
                                            0,
                                            0,
                                            0,
                                            FW_NORMAL.0 as i32,
                                            0,
                                            0,
                                            0,
                                            DEFAULT_CHARSET.0 as u32,
                                            OUT_DEFAULT_PRECIS.0 as u32,
                                            CLIP_DEFAULT_PRECIS.0 as u32,
                                            ANTIALIASED_QUALITY.0 as u32,
                                            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                                            PCWSTR(sub_face_w.as_ptr()),
                                        );
                                        let old_font_sub =
                                            SelectObject(mem_dc, HGDIOBJ(font_sub.0));

                                        let sub_x = 15;
                                        let sub_y = line_top + total_box_height + 2;
                                        let sub_rect = RECT {
                                            left: sub_x,
                                            top: sub_y,
                                            right: rect.right - 15,
                                            bottom: sub_y + font_size_sub_capped + 20,
                                        };
                                        draw_text_outlined(
                                            mem_dc,
                                            &sub_utf16,
                                            sub_rect,
                                            DT_LEFT | DT_WORDBREAK | DT_NOPREFIX,
                                            hex_to_colorref(&cfg.sub_hex),
                                            true,
                                        );
                                        SelectObject(mem_dc, old_font_sub);
                                        let _ = DeleteObject(HGDIOBJ(font_sub.0));
                                    }
                                }
                            }

                            let _ = DeleteObject(HGDIOBJ(font_base.0));
                        } else {
                            let display_text = line.text.clone();
                            let line_utf16: Vec<u16> =
                                format!("{}\0", display_text).encode_utf16().collect();
                            let font_size_side_capped = cfg.font_size_side.min(40);

                            let resolved_side_font =
                                get_font_face_for_text(&display_text, &cfg.font_family);
                            let side_face_w: Vec<u16> =
                                format!("{}\0", resolved_side_font).encode_utf16().collect();

                            let font_side = CreateFontW(
                                font_size_side_capped,
                                0,
                                0,
                                0,
                                FW_NORMAL.0 as i32,
                                0,
                                0,
                                0,
                                DEFAULT_CHARSET.0 as u32,
                                OUT_DEFAULT_PRECIS.0 as u32,
                                CLIP_DEFAULT_PRECIS.0 as u32,
                                ANTIALIASED_QUALITY.0 as u32,
                                (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                                PCWSTR(side_face_w.as_ptr()),
                            );
                            let old_font_side = SelectObject(mem_dc, HGDIOBJ(font_side.0));

                            let distance = (target_idx as f32 - float_idx).abs();
                            let fade_color = if distance > 0.8 {
                                COLORREF(0x333333)
                            } else {
                                hex_to_colorref(&cfg.side_hex)
                            };

                            let l_rect = RECT {
                                left: 15,
                                top: line_top,
                                right: rect.right - 15,
                                bottom: line_top + font_size_side_capped + 20,
                            };
                            draw_text_outlined(
                                mem_dc,
                                &line_utf16,
                                l_rect,
                                DT_LEFT | DT_WORDBREAK | DT_NOPREFIX | DT_NOCLIP,
                                fade_color,
                                true,
                            );

                            SelectObject(mem_dc, old_font_side);
                            let _ = DeleteObject(HGDIOBJ(font_side.0));
                        }
                    }
                }
            } else {
                let mut idle_utf16: Vec<u16> =
                    "Play music to see lyrics...\0".encode_utf16().collect();
                let font = CreateFontW(
                    12,
                    0,
                    0,
                    0,
                    FW_NORMAL.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET.0 as u32,
                    OUT_DEFAULT_PRECIS.0 as u32,
                    CLIP_DEFAULT_PRECIS.0 as u32,
                    CLEARTYPE_QUALITY.0 as u32,
                    (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
                    PCWSTR(default_face_w.as_ptr()),
                );
                let old_font = SelectObject(mem_dc, HGDIOBJ(font.0));
                SetTextColor(mem_dc, COLORREF(0x777777));
                let mut r = RECT {
                    left: 15,
                    top: base_center_y as i32,
                    right: rect.right - 15,
                    bottom: base_center_y as i32 + 30,
                };
                let _ = DrawTextW(mem_dc, &mut idle_utf16, &mut r, DT_LEFT | DT_SINGLELINE);
                SelectObject(mem_dc, old_font);
                let _ = DeleteObject(HGDIOBJ(font.0));
            }
        }
    }
}
