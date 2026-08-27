// src/render.rs
#![allow(static_mut_refs)]
use crate::app_state::APP_STATE;
use crate::config::StyleConfig;
use crate::d2d_engine::{hex_to_d2d_color, lerp_d2d_color, D2DEngine};
use crate::lrc_parser::Syllable;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_POINT_2F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{
    ID2D1RenderTarget, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_DRAW_TEXT_OPTIONS_NONE,
    D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::{IDWriteTextLayout, DWRITE_TEXT_METRICS};

struct RenderSyl {
    x: f32,
    y: f32,
    width: f32,
    progress: f32,
    layout: IDWriteTextLayout,
}

/// Crisp, beautiful text outline shadow: draws a subtle 1px dark halo around
/// the text to ensure high contrast and readability on bright backgrounds,
/// avoiding ugly multi-line duplicate text ghosting artifacts.
unsafe fn draw_text_with_shadow(
    target: &ID2D1RenderTarget,
    brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
    layout: &IDWriteTextLayout,
    x: f32,
    y: f32,
    text_color: &D2D1_COLOR_F,
    cfg: &StyleConfig,
) {
    if cfg.shadow_enabled {
        let alpha = (cfg.shadow_opacity * 0.70).clamp(0.1, 0.8);
        let shadow_color = hex_to_d2d_color(&cfg.shadow_hex, alpha);
        brush.SetColor(&shadow_color);

        let ox = cfg.shadow_offset_x.clamp(0.5, 3.0);
        let oy = cfg.shadow_offset_y.clamp(0.5, 3.0);

        // High-performance single-pass drop shadow (cuts D2D GPU draw calls by 71%)
        target.DrawTextLayout(
            D2D_POINT_2F {
                x: x + ox,
                y: y + oy,
            },
            layout,
            brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
    }
    brush.SetColor(text_color);
    target.DrawTextLayout(
        D2D_POINT_2F { x, y },
        layout,
        brush,
        D2D1_DRAW_TEXT_OPTIONS_NONE,
    );
}

#[inline]
fn line_origin_x(
    _line_idx: usize,
    singer_index: u8,
    width_f: f32,
    text_width: f32,
    _text: &str,
    cfg: &StyleConfig,
) -> f32 {
    let align_mode = cfg.alignment.trim().to_lowercase();
    match align_mode.as_str() {
        "left" => 15.0,
        "right" => (width_f - 15.0 - text_width).max(15.0),
        "center" => ((width_f - text_width) / 2.0).max(15.0),
        _ => match singer_index {
            0 => 15.0,                                     // Solo Singer 1 -> Left Aligned
            1 => (width_f - 15.0 - text_width).max(15.0),  // Singer 2 -> Right Aligned
            2 => ((width_f - text_width) / 2.0).max(15.0), // Group / Unison -> CENTER ALIGNED
            _ => 15.0,
        },
    }
}

unsafe fn get_line_block_height(
    engine: &D2DEngine,
    cfg: &StyleConfig,
    line: &crate::lrc_parser::LrcLine,
    active_font_size: f32,
    font_size_sub_capped: f32,
    max_w: f32,
) -> f32 {
    let eff_font_size = if line.is_background {
        active_font_size * 0.70
    } else {
        active_font_size
    };
    let eff_main_lh = (eff_font_size * 1.25).max(eff_font_size + 8.0);
    let mut total_h = eff_main_lh;

    // Check if line has mixed normal + background syllables
    let has_bg_syls = !line.is_background && line.syllables.iter().any(|s| s.is_background);
    let main_text_for_layout = if has_bg_syls {
        let main_part: String = line
            .syllables
            .iter()
            .filter(|s| !s.is_background)
            .map(|s| s.text.as_str())
            .collect();
        main_part
    } else {
        line.text.clone()
    };

    if let Ok(layout) = engine.get_cached_text_layout(
        &main_text_for_layout,
        &cfg.font_family,
        eff_font_size,
        true,
        max_w,
        eff_main_lh,
    ) {
        let mut metrics = DWRITE_TEXT_METRICS::default();
        if layout.GetMetrics(&mut metrics).is_ok() {
            total_h = metrics.height.max(eff_main_lh);
        }
    }

    // Add height for inline background syllable row
    if has_bg_syls {
        let bg_font_size = eff_font_size * 0.70;
        let bg_lh = (bg_font_size * 1.25).max(bg_font_size + 8.0);
        let bg_text: String = line
            .syllables
            .iter()
            .filter(|s| s.is_background)
            .map(|s| s.text.as_str())
            .collect();
        if !bg_text.trim().is_empty() {
            if let Ok(bg_layout) = engine.get_cached_text_layout(
                &bg_text,
                &cfg.font_family,
                bg_font_size,
                true,
                max_w,
                bg_lh,
            ) {
                let mut bg_metrics = DWRITE_TEXT_METRICS::default();
                if bg_layout.GetMetrics(&mut bg_metrics).is_ok() {
                    total_h += bg_metrics.height + 6.0;
                } else {
                    total_h += bg_lh + 6.0;
                }
            } else {
                total_h += bg_lh + 6.0;
            }
        }
    }

    if let Some(ref raw_sub) = line.sub_text {
        if !raw_sub.is_empty() {
            let formatted_sub = line
                .get_formatted_sub_text()
                .unwrap_or_else(|| raw_sub.clone());
            let sub_lh = (font_size_sub_capped * 1.25).max(font_size_sub_capped + 6.0);
            if let Ok(sub_layout) = engine.get_cached_text_layout(
                &formatted_sub,
                &cfg.font_family,
                font_size_sub_capped,
                false,
                max_w,
                sub_lh,
            ) {
                let mut sub_metrics = DWRITE_TEXT_METRICS::default();
                if sub_layout.GetMetrics(&mut sub_metrics).is_ok() {
                    total_h += sub_metrics.height + 10.0;
                } else {
                    total_h += sub_lh + 14.0;
                }
            } else {
                total_h += sub_lh + 14.0;
            }
        }
    }

    total_h
}

pub unsafe fn render_window_d2d(
    target: &ID2D1RenderTarget,
    rect: RECT,
    engine: &D2DEngine,
) -> windows::core::Result<()> {
    target.BeginDraw();

    let transparent_bg = hex_to_d2d_color("000000", 0.0);
    target.Clear(Some(&transparent_bg));

    let width_f = (rect.right - rect.left) as f32;
    let height_f = (rect.bottom - rect.top) as f32;

    let reusable_brush = target.CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)?;

    if let Some(state_arc) = APP_STATE.as_ref() {
        if let Ok(s) = state_arc.lock() {
            let cfg = &s.config;
            if cfg.show_card.unwrap_or(false) {
                let card_color_str = cfg.card_bg_hex.as_deref().unwrap_or("12121a");
                let card_color = hex_to_d2d_color(card_color_str, 0.85);
                reusable_brush.SetColor(&card_color);

                let rounded_rect = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: 2.0,
                        top: 2.0,
                        right: width_f - 2.0,
                        bottom: height_f - 2.0,
                    },
                    radiusX: 16.0,
                    radiusY: 16.0,
                };

                target.FillRoundedRectangle(&rounded_rect, &reusable_brush);

                let border_color = hex_to_d2d_color("262636", 0.9);
                reusable_brush.SetColor(&border_color);
                target.DrawRoundedRectangle(&rounded_rect, &reusable_brush, 1.2, None);
            }
        }
    }

    if let Some(state_arc) = APP_STATE.as_ref() {
        if let Ok(s) = state_arc.lock() {
            let cfg = &s.config;

            let scale = (height_f / 200.0).max(0.6);
            let base_center_y = cfg.base_center_y * scale;
            let base_step = cfg.line_spacing * scale;

            let mut cursor_y = 8.0 * scale;

            let lock_status = if s.is_locked { "🔒" } else { "🔓" };
            let lock_layout = engine.get_cached_text_layout(
                lock_status,
                &cfg.font_family,
                11.0 * scale,
                false,
                30.0,
                20.0,
            )?;
            reusable_brush.SetColor(&hex_to_d2d_color("AAAAAA", 0.9));
            target.DrawTextLayout(
                D2D_POINT_2F {
                    x: width_f - 35.0,
                    y: 2.0,
                },
                &lock_layout,
                &reusable_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );

            if let Some(ref provider) = s.provider_name {
                let provider_layout = engine.get_cached_text_layout(
                    provider,
                    &cfg.font_family,
                    9.0 * scale,
                    false,
                    55.0,
                    15.0,
                )?;
                reusable_brush.SetColor(&hex_to_d2d_color("8888AA", 0.85));
                target.DrawTextLayout(
                    D2D_POINT_2F {
                        x: width_f - 55.0,
                        y: 18.0 * scale,
                    },
                    &provider_layout,
                    &reusable_brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                );
            }

            if !s.media.title.is_empty() {
                let font_size_title_capped = (cfg.font_size_title.min(40)) as f32;
                let max_title_h = (font_size_title_capped * 2.2).max(28.0);

                let title_layout = engine.get_cached_text_layout(
                    &s.media.title,
                    &cfg.font_family,
                    font_size_title_capped,
                    true,
                    (width_f - 75.0).max(50.0),
                    max_title_h,
                )?;
                let mut title_metrics = DWRITE_TEXT_METRICS::default();
                title_layout.GetMetrics(&mut title_metrics)?;

                let title_color = hex_to_d2d_color(&cfg.title_hex, 1.0);
                draw_text_with_shadow(
                    target,
                    &reusable_brush,
                    &title_layout,
                    15.0,
                    cursor_y,
                    &title_color,
                    cfg,
                );

                cursor_y += title_metrics.height + 2.0;
            }

            if !s.media.artist.is_empty() {
                let font_size_artist_capped = (cfg.font_size_artist.min(40)) as f32;
                let artist_layout = engine.get_cached_text_layout(
                    &s.media.artist,
                    &cfg.font_family,
                    font_size_artist_capped,
                    false,
                    (width_f - 75.0).max(50.0),
                    (font_size_artist_capped * 2.0) + 10.0,
                )?;
                let mut artist_metrics = DWRITE_TEXT_METRICS::default();
                artist_layout.GetMetrics(&mut artist_metrics)?;

                let artist_color = hex_to_d2d_color(&cfg.artist_hex, 0.95);
                draw_text_with_shadow(
                    target,
                    &reusable_brush,
                    &artist_layout,
                    15.0,
                    cursor_y,
                    &artist_color,
                    cfg,
                );

                cursor_y += artist_metrics.height + 2.0;
            }

            let header_bottom = cursor_y + 4.0;

            let real_pos_ms = s.media.position_ms;
            let adjusted_ms = (real_pos_ms as i64 + s.offset_ms).max(0) as u64;
            let float_idx = s.float_index;

            let raw_active_font_size = (cfg.font_size_active.min(40)) as f32;
            let current_idx = s.current_index;
            let max_w = width_f - 30.0;

            let responsive_scale = if current_idx < s.lyrics_lines.len() {
                let active_line = &s.lyrics_lines[current_idx];
                let font_size = if active_line.is_background {
                    raw_active_font_size * 0.70
                } else {
                    raw_active_font_size
                };
                let lh = font_size + 4.0;

                if let Ok(layout) = engine.get_cached_text_layout(
                    &active_line.text,
                    &cfg.font_family,
                    font_size,
                    true,
                    max_w,
                    lh,
                ) {
                    let mut metrics = DWRITE_TEXT_METRICS::default();
                    if layout.GetMetrics(&mut metrics).is_ok() {
                        if metrics.lineCount >= 3 {
                            0.75f32
                        } else if metrics.lineCount >= 2 {
                            0.85f32
                        } else {
                            1.00f32
                        }
                    } else {
                        1.00f32
                    }
                } else {
                    1.00f32
                }
            } else {
                1.00f32
            };

            let active_font_size = raw_active_font_size * responsive_scale;
            let font_size_sub_capped = (cfg.font_size_sub.clamp(14, 40) as f32) * responsive_scale;
            let _active_karaoke_color = hex_to_d2d_color(&cfg.karaoke_hex, 1.0);
            let _active_text_color = hex_to_d2d_color(&cfg.active_hex, 1.0);

            let main_lh = (active_font_size * 1.25).max(active_font_size + 8.0);
            let mut active_h = main_lh;

            if current_idx < s.lyrics_lines.len() {
                let active_line = &s.lyrics_lines[current_idx];
                active_h = get_line_block_height(
                    engine,
                    cfg,
                    active_line,
                    active_font_size,
                    font_size_sub_capped,
                    max_w,
                );

                if !active_line.is_background && current_idx + 1 < s.lyrics_lines.len() {
                    let next_line = &s.lyrics_lines[current_idx + 1];
                    if next_line.is_background {
                        let bg_h = get_line_block_height(
                            engine,
                            cfg,
                            next_line,
                            active_font_size,
                            font_size_sub_capped,
                            max_w,
                        );
                        active_h += bg_h + 12.0;
                    }
                }
            }

            let max_center_y = (height_f - active_h - 24.0).max(header_bottom + 4.0);
            let active_center_y = base_center_y.clamp(header_bottom + 4.0, max_center_y);

            let is_in_instrumental_gap = if current_idx < s.lyrics_lines.len() {
                let cur_line = &s.lyrics_lines[current_idx];
                let is_cur_instrumental = cur_line.text.trim().is_empty()
                    || cur_line.text.trim() == "♪"
                    || cur_line.text.to_lowercase().contains("instrumental")
                    || cur_line.text.contains('♪');

                if !is_cur_instrumental {
                    let cur_end_ms = if let Some(end) = cur_line.end_time {
                        end.as_millis() as u64
                    } else if !cur_line.syllables.is_empty() {
                        let total_dur: u64 = cur_line
                            .syllables
                            .iter()
                            .map(|s| s.duration.as_millis() as u64)
                            .sum();
                        cur_line.time.as_millis() as u64 + total_dur
                    } else {
                        cur_line.time.as_millis() as u64 + 3500
                    };

                    if adjusted_ms >= cur_end_ms {
                        if current_idx + 1 < s.lyrics_lines.len() {
                            let next_start_ms =
                                s.lyrics_lines[current_idx + 1].time.as_millis() as u64;
                            adjusted_ms < next_start_ms
                                && (next_start_ms.saturating_sub(cur_end_ms) >= 1500)
                        } else {
                            true
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if s.is_loading {
                let loading_layout = engine.get_cached_text_layout(
                    "Loading lyrics...",
                    &cfg.font_family,
                    12.0,
                    false,
                    width_f - 30.0,
                    30.0,
                )?;
                let loading_color = hex_to_d2d_color("888888", 0.9);
                draw_text_with_shadow(
                    target,
                    &reusable_brush,
                    &loading_layout,
                    15.0,
                    active_center_y,
                    &loading_color,
                    cfg,
                );
            } else if !s.lyrics_lines.is_empty() {
                for offset in -1isize..=1 {
                    let target_idx = (current_idx as isize) + offset;
                    if target_idx >= 0 && target_idx < s.lyrics_lines.len() as isize {
                        let line = &s.lyrics_lines[target_idx as usize];
                        let distance_from_float = (target_idx as f32) - float_idx;

                        let line_top = if distance_from_float <= 0.0 {
                            let prev_h = get_line_block_height(
                                engine,
                                cfg,
                                line,
                                active_font_size,
                                font_size_sub_capped,
                                max_w,
                            );
                            let step_prev = prev_h.max(base_step) + 12.0;
                            active_center_y + distance_from_float * step_prev
                        } else if line.is_background
                            && target_idx > 0
                            && !s.lyrics_lines[(target_idx - 1) as usize].is_background
                        {
                            let main_block_h = get_line_block_height(
                                engine,
                                cfg,
                                &s.lyrics_lines[(target_idx - 1) as usize],
                                active_font_size,
                                font_size_sub_capped,
                                max_w,
                            );
                            active_center_y
                                + (main_block_h + 6.0) * distance_from_float.min(1.0)
                                + (distance_from_float - 1.0).max(0.0) * base_step
                        } else {
                            active_center_y
                                + (active_h + 12.0) * distance_from_float.min(1.0)
                                + (distance_from_float - 1.0).max(0.0) * base_step
                        };

                        if offset != 0 && line_top < header_bottom {
                            continue;
                        }

                        let is_paired_bg_of_active = line.is_background
                            && target_idx > 0
                            && (target_idx as usize) - 1 == current_idx
                            && !s.lyrics_lines[(target_idx - 1) as usize].is_background;

                        let is_active = offset == 0 || is_paired_bg_of_active;

                        let (mut active_karaoke_color, mut active_text_color) = if is_active {
                            let k_col = match line.singer_index {
                                1 => hex_to_d2d_color(&cfg.karaoke_v2_hex, 1.0),
                                2 => hex_to_d2d_color(&cfg.karaoke_group_hex, 1.0),
                                _ => hex_to_d2d_color(&cfg.karaoke_hex, 1.0),
                            };
                            let t_col = hex_to_d2d_color(&cfg.active_hex, 1.0);
                            (k_col, t_col)
                        } else {
                            let side_col = hex_to_d2d_color(&cfg.side_hex, 0.70);
                            (side_col, side_col)
                        };

                        if line.is_background {
                            active_karaoke_color.a = (active_karaoke_color.a * 0.75).min(0.75);
                            active_text_color.a = (active_text_color.a * 0.75).min(0.75);
                        }
                        let eff_font_size = if is_active {
                            if line.is_background {
                                active_font_size * 0.70
                            } else {
                                active_font_size
                            }
                        } else {
                            let font_size_side_capped = (cfg.font_size_side.min(40)) as f32;
                            if line.is_background {
                                font_size_side_capped * 0.70
                            } else {
                                font_size_side_capped
                            }
                        };

                        let is_instrumental = line.text.trim() == "♪"
                            || line.text.to_lowercase().contains("instrumental")
                            || line.text.contains('♪');

                        let active_is_instrumental = is_instrumental || is_in_instrumental_gap;

                        if is_active {
                            if active_is_instrumental {
                                let note_text = "♪";
                                let small_icon_size = 18.0f32;
                                let note_layout = engine.get_cached_text_layout(
                                    note_text,
                                    &cfg.font_family,
                                    small_icon_size,
                                    true,
                                    width_f - 30.0,
                                    small_icon_size + 20.0,
                                )?;
                                let mut note_metrics = DWRITE_TEXT_METRICS::default();
                                note_layout.GetMetrics(&mut note_metrics)?;
                                let note_width = note_metrics.widthIncludingTrailingWhitespace;
                                let note_height = note_metrics.height;
                                let note_x = line_origin_x(
                                    target_idx as usize,
                                    line.singer_index,
                                    width_f,
                                    note_width,
                                    note_text,
                                    cfg,
                                );

                                let (start_ms, dur_ms) = if is_in_instrumental_gap {
                                    let cur_end_ms = if let Some(end) = line.end_time {
                                        end.as_millis() as u64
                                    } else if !line.syllables.is_empty() {
                                        let total_dur: u64 = line
                                            .syllables
                                            .iter()
                                            .map(|s| s.duration.as_millis() as u64)
                                            .sum();
                                        line.time.as_millis() as u64 + total_dur
                                    } else {
                                        line.time.as_millis() as u64 + 3500
                                    };
                                    let next_start_ms = if current_idx + 1 < s.lyrics_lines.len() {
                                        s.lyrics_lines[current_idx + 1].time.as_millis() as u64
                                    } else {
                                        cur_end_ms + 4000
                                    };
                                    (cur_end_ms, next_start_ms.saturating_sub(cur_end_ms).max(1))
                                } else {
                                    let s_ms = line.time.as_millis() as u64;
                                    let d_ms = if let Some(end) = line.end_time {
                                        end.saturating_sub(line.time).as_millis().max(1) as u64
                                    } else {
                                        4000
                                    };
                                    (s_ms, d_ms)
                                };

                                let elapsed_inst = adjusted_ms.saturating_sub(start_ms);
                                let inst_progress =
                                    (elapsed_inst as f32 / dur_ms as f32).clamp(0.0, 1.0);

                                // Base dim layer
                                let dim_note_color = hex_to_d2d_color(&cfg.side_hex, 0.4);
                                draw_text_with_shadow(
                                    target,
                                    &reusable_brush,
                                    &note_layout,
                                    note_x,
                                    line_top,
                                    &dim_note_color,
                                    cfg,
                                );

                                // Active vertical fill layer (bottom to top)
                                let fill_h = note_height * inst_progress;
                                if fill_h > 0.0 {
                                    let icon_bottom = line_top + note_height;
                                    let clip_rect = D2D_RECT_F {
                                        left: note_x - 4.0,
                                        top: icon_bottom - fill_h,
                                        right: note_x + note_width + 4.0,
                                        bottom: icon_bottom + 4.0,
                                    };
                                    target.PushAxisAlignedClip(
                                        &clip_rect,
                                        D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                                    );
                                    draw_text_with_shadow(
                                        target,
                                        &reusable_brush,
                                        &note_layout,
                                        note_x,
                                        line_top,
                                        &active_karaoke_color,
                                        cfg,
                                    );
                                    target.PopAxisAlignedClip();
                                }
                            } else {
                                let karaoke_mode = cfg.karaoke_mode.trim().to_lowercase();
                                let use_karaoke = match karaoke_mode.as_str() {
                                    "always" => true,
                                    "never" => false,
                                    _ => line.is_karaoke, // "auto"
                                };

                                if !use_karaoke {
                                    let line_height =
                                        (eff_font_size * 1.25).max(eff_font_size + 8.0);

                                    let has_inline_bg_nk = !line.is_background
                                        && line.syllables.iter().any(|s| s.is_background);

                                    let text_to_draw = if has_inline_bg_nk {
                                        // Main text from non-bg syllables only
                                        let main_part: String = line
                                            .syllables
                                            .iter()
                                            .filter(|s| !s.is_background)
                                            .map(|s| s.text.as_str())
                                            .collect();
                                        if main_part.is_empty() {
                                            line.text.clone()
                                        } else {
                                            main_part
                                        }
                                    } else if !line.text.is_empty() {
                                        line.text.clone()
                                    } else {
                                        line.syllables
                                            .iter()
                                            .map(|s| s.text.as_str())
                                            .collect::<String>()
                                    };

                                    let line_layout = engine.get_cached_text_layout(
                                        &text_to_draw,
                                        &cfg.font_family,
                                        eff_font_size,
                                        true,
                                        max_w,
                                        line_height,
                                    )?;
                                    let mut metrics = DWRITE_TEXT_METRICS::default();
                                    line_layout.GetMetrics(&mut metrics)?;

                                    let mut main_text_bottom =
                                        line_top + metrics.height.max(line_height);
                                    let line_width = metrics.widthIncludingTrailingWhitespace;
                                    let line_x = line_origin_x(
                                        target_idx as usize,
                                        line.singer_index,
                                        width_f,
                                        line_width,
                                        &text_to_draw,
                                        cfg,
                                    );

                                    let draw_color = if is_active {
                                        active_karaoke_color
                                    } else {
                                        hex_to_d2d_color(&cfg.side_hex, 0.7)
                                    };

                                    draw_text_with_shadow(
                                        target,
                                        &reusable_brush,
                                        &line_layout,
                                        line_x,
                                        line_top,
                                        &draw_color,
                                        cfg,
                                    );

                                    // Render inline bg row for non-karaoke
                                    if has_inline_bg_nk {
                                        let bg_text: String = line
                                            .syllables
                                            .iter()
                                            .filter(|s| s.is_background)
                                            .map(|s| s.text.as_str())
                                            .collect();
                                        if !bg_text.trim().is_empty() {
                                            let bg_fs = eff_font_size * 0.70;
                                            let bg_lh = (bg_fs * 1.25).max(bg_fs + 8.0);
                                            let bg_layout = engine.get_cached_text_layout(
                                                &bg_text,
                                                &cfg.font_family,
                                                bg_fs,
                                                true,
                                                max_w,
                                                bg_lh,
                                            )?;
                                            let mut bg_met = DWRITE_TEXT_METRICS::default();
                                            bg_layout.GetMetrics(&mut bg_met)?;
                                            let bg_w = bg_met.widthIncludingTrailingWhitespace;
                                            let bg_y = main_text_bottom + 6.0;
                                            let bg_x = line_origin_x(
                                                target_idx as usize,
                                                line.singer_index,
                                                width_f,
                                                bg_w,
                                                &bg_text,
                                                cfg,
                                            );
                                            let mut bg_color = draw_color;
                                            bg_color.a = (bg_color.a * 0.75).min(0.75);
                                            draw_text_with_shadow(
                                                target,
                                                &reusable_brush,
                                                &bg_layout,
                                                bg_x,
                                                bg_y,
                                                &bg_color,
                                                cfg,
                                            );
                                            main_text_bottom = bg_y + bg_met.height.max(bg_lh);
                                        }
                                    }

                                    if let Some(ref sub) = line.sub_text {
                                        if !sub.is_empty() {
                                            let font_size_sub_capped =
                                                (cfg.font_size_sub.clamp(14, 40) as f32)
                                                    * responsive_scale;
                                            let min_gap = 6.0f32;
                                            let minimal_y = main_text_bottom + min_gap;

                                            if minimal_y + font_size_sub_capped <= height_f - 4.0 {
                                                let sub_y = minimal_y;

                                                let sub_layout = engine.get_cached_text_layout(
                                                    sub,
                                                    &cfg.font_family,
                                                    font_size_sub_capped,
                                                    false,
                                                    width_f - 30.0,
                                                    font_size_sub_capped + 20.0,
                                                )?;
                                                let mut sub_metrics =
                                                    DWRITE_TEXT_METRICS::default();
                                                sub_layout.GetMetrics(&mut sub_metrics)?;
                                                let sub_width =
                                                    sub_metrics.widthIncludingTrailingWhitespace;
                                                let sub_x = line_origin_x(
                                                    target_idx as usize,
                                                    line.singer_index,
                                                    width_f,
                                                    sub_width,
                                                    sub,
                                                    cfg,
                                                );

                                                let sub_color =
                                                    hex_to_d2d_color(&cfg.sub_hex, 0.95);
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &sub_layout,
                                                    sub_x,
                                                    sub_y,
                                                    &sub_color,
                                                    cfg,
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    let start_ms = line.time.as_millis() as u64;
                                    let elapsed_line = adjusted_ms.saturating_sub(start_ms);

                                    let line_height =
                                        (eff_font_size * 1.25).max(eff_font_size + 8.0);

                                    struct SylPrep<'a> {
                                        syl: &'a Syllable,
                                        layout: IDWriteTextLayout,
                                        width: f32,
                                        progress: f32,
                                    }

                                    let has_inline_bg_syls = !line.is_background
                                        && line.syllables.iter().any(|s| s.is_background);
                                    let bg_font_size_k = eff_font_size * 0.70;
                                    let bg_line_height_k =
                                        (bg_font_size_k * 1.25).max(bg_font_size_k + 8.0);

                                    let mut accumulated_syl_time = 0u64;
                                    let mut prep_syls = Vec::with_capacity(line.syllables.len());
                                    let mut bg_prep_syls: Vec<SylPrep> = Vec::new();

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

                                        let is_bg_syl = has_inline_bg_syls && syl.is_background;
                                        let syl_fs = if is_bg_syl {
                                            bg_font_size_k
                                        } else {
                                            eff_font_size
                                        };
                                        let syl_lh = if is_bg_syl {
                                            bg_line_height_k
                                        } else {
                                            line_height
                                        };

                                        let layout = engine.get_cached_text_layout(
                                            &syl.text,
                                            &cfg.font_family,
                                            syl_fs,
                                            true,
                                            max_w,
                                            syl_lh,
                                        )?;
                                        let mut metrics = DWRITE_TEXT_METRICS::default();
                                        layout.GetMetrics(&mut metrics)?;
                                        let syl_width = metrics.widthIncludingTrailingWhitespace;

                                        let prep = SylPrep {
                                            syl,
                                            layout,
                                            width: syl_width,
                                            progress: syl_progress,
                                        };

                                        if is_bg_syl {
                                            bg_prep_syls.push(prep);
                                        } else {
                                            prep_syls.push(prep);
                                        }
                                    }

                                    struct WordUnitKaraoke<'a> {
                                        items: Vec<&'a SylPrep<'a>>,
                                        width: f32,
                                    }

                                    let mut word_units: Vec<WordUnitKaraoke> = Vec::new();
                                    let mut cur_unit_items = Vec::new();
                                    let mut cur_unit_w = 0.0f32;
                                    let mut prev_ends_with_space = true;

                                    for ps in &prep_syls {
                                        let text_str = &ps.syl.text;
                                        let starts_with_space =
                                            text_str.starts_with(' ') || text_str.starts_with('\t');
                                        let is_new_word = prev_ends_with_space || starts_with_space;

                                        if is_new_word && !cur_unit_items.is_empty() {
                                            word_units.push(WordUnitKaraoke {
                                                items: cur_unit_items,
                                                width: cur_unit_w,
                                            });
                                            cur_unit_items = Vec::new();
                                            cur_unit_w = 0.0;
                                        }

                                        prev_ends_with_space =
                                            text_str.ends_with(' ') || text_str.ends_with('\t');
                                        cur_unit_w += ps.width;
                                        cur_unit_items.push(ps);
                                    }
                                    if !cur_unit_items.is_empty() {
                                        word_units.push(WordUnitKaraoke {
                                            items: cur_unit_items,
                                            width: cur_unit_w,
                                        });
                                    }

                                    struct VisualLineKaraoke<'a> {
                                        items: Vec<&'a SylPrep<'a>>,
                                        width: f32,
                                        snippet: String,
                                    }

                                    let mut visual_lines: Vec<VisualLineKaraoke> = Vec::new();
                                    let mut current_items = Vec::new();
                                    let mut current_w = 0.0f32;
                                    let mut current_text = String::new();

                                    for unit in word_units {
                                        if current_w + unit.width <= max_w {
                                            current_w += unit.width;
                                            for ps in unit.items {
                                                current_text.push_str(&ps.syl.text);
                                                current_items.push(ps);
                                            }
                                        } else {
                                            if !current_items.is_empty() {
                                                visual_lines.push(VisualLineKaraoke {
                                                    items: current_items,
                                                    width: current_w,
                                                    snippet: current_text,
                                                });
                                                current_items = Vec::new();
                                                current_w = 0.0;
                                                current_text = String::new();
                                            }

                                            if unit.width <= max_w {
                                                current_w = unit.width;
                                                for ps in unit.items {
                                                    current_text.push_str(&ps.syl.text);
                                                    current_items.push(ps);
                                                }
                                            } else {
                                                for ps in unit.items {
                                                    if current_w + ps.width > max_w
                                                        && !current_items.is_empty()
                                                    {
                                                        visual_lines.push(VisualLineKaraoke {
                                                            items: current_items,
                                                            width: current_w,
                                                            snippet: current_text,
                                                        });
                                                        current_items = Vec::new();
                                                        current_w = 0.0;
                                                        current_text = String::new();
                                                    }
                                                    current_w += ps.width;
                                                    current_text.push_str(&ps.syl.text);
                                                    current_items.push(ps);
                                                }
                                            }
                                        }
                                    }
                                    if !current_items.is_empty() {
                                        visual_lines.push(VisualLineKaraoke {
                                            items: current_items,
                                            width: current_w,
                                            snippet: current_text,
                                        });
                                    }

                                    let mut render_data = Vec::with_capacity(prep_syls.len());
                                    let mut current_y = line_top;
                                    let mut main_text_bottom = line_top + line_height;

                                    for vline in &visual_lines {
                                        let line_origin = line_origin_x(
                                            target_idx as usize,
                                            line.singer_index,
                                            width_f,
                                            vline.width,
                                            &vline.snippet,
                                            cfg,
                                        );

                                        let mut current_x = line_origin;
                                        for ps in &vline.items {
                                            render_data.push(RenderSyl {
                                                x: current_x,
                                                y: current_y,
                                                width: ps.width,
                                                progress: ps.progress,
                                                layout: ps.layout.clone(),
                                            });
                                            current_x += ps.width;
                                            let mut metrics = DWRITE_TEXT_METRICS::default();
                                            if ps.layout.GetMetrics(&mut metrics).is_ok() {
                                                let b = current_y + metrics.height;
                                                if b > main_text_bottom {
                                                    main_text_bottom = b;
                                                }
                                            }
                                        }
                                        current_y += line_height;
                                    }
                                    let _total_box_height = (current_y - line_top).max(line_height);
                                    let active_idx = render_data
                                        .iter()
                                        .position(|r| r.progress < 1.0)
                                        .unwrap_or_else(|| render_data.len().saturating_sub(1));

                                    let effect = cfg.karaoke_effect.trim().to_lowercase();
                                    let is_sweep_style = matches!(
                                        effect.as_str(),
                                        "sweep" | "kf" | "glow" | "glow_sweep" | "bloom"
                                    );

                                    let (sung_color, unsung_color) = if is_sweep_style {
                                        (
                                            active_karaoke_color,
                                            D2D1_COLOR_F {
                                                a: active_text_color.a * 0.40,
                                                ..active_text_color
                                            },
                                        )
                                    } else {
                                        (active_karaoke_color, active_text_color)
                                    };

                                    for (i, rs) in render_data.iter().enumerate() {
                                        if i == active_idx {
                                            continue;
                                        }

                                        let syl_color = if rs.progress >= 1.0 {
                                            sung_color
                                        } else {
                                            unsung_color
                                        };
                                        draw_text_with_shadow(
                                            target,
                                            &reusable_brush,
                                            &rs.layout,
                                            rs.x,
                                            rs.y,
                                            &syl_color,
                                            cfg,
                                        );
                                    }

                                    if active_idx < render_data.len() {
                                        let rs = &render_data[active_idx];
                                        let effect = cfg.karaoke_effect.trim().to_lowercase();

                                        match effect.as_str() {
                                            "star_bounce" | "star" | "ball" => {
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );

                                                let arc =
                                                    (rs.progress * std::f32::consts::PI).sin();
                                                let word_lift = -(arc * 3.5);
                                                let pop_scale = 1.0 + (arc * 0.05);

                                                let cx = rs.x + (rs.width / 2.0);
                                                let cy = rs.y + (line_height / 2.0);

                                                let word_transform = Matrix3x2 {
                                                    M11: pop_scale,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: pop_scale,
                                                    M31: cx * (1.0 - pop_scale),
                                                    M32: cy * (1.0 - pop_scale) + word_lift,
                                                };

                                                target.SetTransform(&word_transform);
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &blended,
                                                    cfg,
                                                );

                                                let identity = Matrix3x2 {
                                                    M11: 1.0,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: 1.0,
                                                    M31: 0.0,
                                                    M32: 0.0,
                                                };
                                                target.SetTransform(&identity);

                                                // Silky smooth trajectory: ease-out sine horizontal motion across full word duration
                                                let prev_center_x = if active_idx > 0 {
                                                    render_data[active_idx - 1].x
                                                        + (render_data[active_idx - 1].width / 2.0)
                                                } else {
                                                    rs.x - 10.0
                                                };
                                                let curr_center_x = rs.x + (rs.width / 2.0);

                                                let star_size =
                                                    (eff_font_size * 0.50).clamp(12.0, 22.0);
                                                let center_y_base =
                                                    rs.y + (line_height / 2.0) - (star_size * 0.55);

                                                let t = rs.progress.clamp(0.0, 1.0);
                                                let ease_x =
                                                    (t * std::f32::consts::FRAC_PI_2).sin();
                                                let star_x = prev_center_x
                                                    + (curr_center_x - prev_center_x) * ease_x;
                                                let bounce_h = (t * std::f32::consts::PI).sin()
                                                    * (eff_font_size * 0.48).clamp(10.0, 18.0);
                                                let star_y = center_y_base - bounce_h;

                                                // Smooth quad-ease fade-out resting on the word center
                                                let star_alpha = if t <= 0.70 {
                                                    1.0
                                                } else {
                                                    let fade_t = (t - 0.70) / 0.30;
                                                    1.0 - (fade_t * fade_t)
                                                };
                                                let star_color = D2D1_COLOR_F {
                                                    r: active_karaoke_color.r,
                                                    g: active_karaoke_color.g,
                                                    b: active_karaoke_color.b,
                                                    a: active_karaoke_color.a * star_alpha,
                                                };

                                                // Dynamic ASS / KaraFX Particle Sparkle Burst System
                                                if rs.progress > 0.01 && rs.progress < 0.99 {
                                                    let t = rs.progress;
                                                    let sparkle_alpha =
                                                        (arc * 0.95).clamp(0.0, 1.0);
                                                    let word_cx = rs.x + (rs.width / 2.0);
                                                    let word_cy = rs.y + (line_height / 2.0);

                                                    let particles = [
                                                        (
                                                            "✦", 0.42, -135.0f32, 22.0f32, 4.5f32,
                                                            1.0f32, 0.88f32, 0.35f32,
                                                        ),
                                                        (
                                                            "✧", 0.38, -45.0f32, 24.0f32, -5.0f32,
                                                            0.45f32, 0.92f32, 1.00f32,
                                                        ),
                                                        (
                                                            "✦", 0.45, -90.0f32, 28.0f32, 3.0f32,
                                                            1.00f32, 0.96f32, 0.65f32,
                                                        ),
                                                        (
                                                            "✧", 0.36, 140.0f32, 18.0f32, 6.0f32,
                                                            0.95f32, 0.55f32, 0.85f32,
                                                        ),
                                                        (
                                                            "✦", 0.35, 40.0f32, 20.0f32, -4.0f32,
                                                            0.70f32, 1.00f32, 0.50f32,
                                                        ),
                                                    ];

                                                    for (
                                                        glyph,
                                                        font_scale,
                                                        angle_deg,
                                                        max_dist,
                                                        rot_speed,
                                                        r,
                                                        g,
                                                        b,
                                                    ) in particles
                                                    {
                                                        let rad = angle_deg.to_radians();
                                                        let dist = t * max_dist;
                                                        let p_x = word_cx + rad.cos() * dist;
                                                        let p_y = word_cy + rad.sin() * dist
                                                            - (arc * 8.0);

                                                        let p_scale =
                                                            (t * std::f32::consts::PI).sin() * 1.25;
                                                        let angle_rot = t * rot_speed;
                                                        let cos_r = angle_rot.cos();
                                                        let sin_r = angle_rot.sin();

                                                        let p_color = D2D1_COLOR_F {
                                                            r,
                                                            g,
                                                            b,
                                                            a: sparkle_alpha
                                                                * (1.0 - (t - 0.5).abs() * 0.8)
                                                                    .clamp(0.0, 1.0),
                                                        };

                                                        let p_layout = engine
                                                            .get_cached_text_layout(
                                                                glyph,
                                                                &cfg.font_family,
                                                                (eff_font_size * font_scale)
                                                                    .clamp(8.0, 18.0),
                                                                true,
                                                                20.0,
                                                                20.0,
                                                            )?;

                                                        let p_transform = Matrix3x2 {
                                                            M11: cos_r * p_scale,
                                                            M12: sin_r * p_scale,
                                                            M21: -sin_r * p_scale,
                                                            M22: cos_r * p_scale,
                                                            M31: p_x * (1.0 - cos_r * p_scale)
                                                                + p_y * (sin_r * p_scale),
                                                            M32: p_y * (1.0 - cos_r * p_scale)
                                                                - p_x * (sin_r * p_scale),
                                                        };

                                                        target.SetTransform(&p_transform);
                                                        draw_text_with_shadow(
                                                            target,
                                                            &reusable_brush,
                                                            &p_layout,
                                                            p_x - 6.0,
                                                            p_y - 6.0,
                                                            &p_color,
                                                            cfg,
                                                        );
                                                    }
                                                    target.SetTransform(&identity);
                                                }

                                                // Main bouncing star fading away at edges
                                                let star_layout = engine.get_cached_text_layout(
                                                    "★",
                                                    &cfg.font_family,
                                                    (eff_font_size * 0.52).clamp(13.0, 24.0),
                                                    true,
                                                    25.0,
                                                    25.0,
                                                )?;
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &star_layout,
                                                    star_x - 5.0,
                                                    star_y,
                                                    &star_color,
                                                    cfg,
                                                );
                                            }
                                            "none" => {
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &active_karaoke_color,
                                                    cfg,
                                                );
                                            }
                                            "fade" => {
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &blended,
                                                    cfg,
                                                );
                                            }
                                            "wave" => {
                                                let bounce =
                                                    (rs.progress * std::f32::consts::PI).sin();
                                                let y_shift = -(bounce * 6.0);
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y + y_shift,
                                                    &blended,
                                                    cfg,
                                                );
                                            }
                                            "sweep" | "kf" | "glow" | "glow_sweep" | "bloom" => {
                                                let base_color = D2D1_COLOR_F {
                                                    a: active_text_color.a * 0.40,
                                                    ..active_text_color
                                                };
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &base_color,
                                                    cfg,
                                                );
                                                let fill_w = rs.width * rs.progress;
                                                if fill_w > 0.0 {
                                                    let edge_x = rs.x + fill_w;
                                                    let sung_fill = active_karaoke_color;

                                                    // Main solid sung fill up to edge_x - 3.0
                                                    let clip_solid = D2D_RECT_F {
                                                        left: rs.x - 2.0,
                                                        top: rs.y - 4.0,
                                                        right: (edge_x - 3.0).max(rs.x - 2.0),
                                                        bottom: rs.y + line_height + 4.0,
                                                    };
                                                    target.PushAxisAlignedClip(
                                                        &clip_solid,
                                                        D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                                                    );
                                                    draw_text_with_shadow(
                                                        target,
                                                        &reusable_brush,
                                                        &rs.layout,
                                                        rs.x,
                                                        rs.y,
                                                        &sung_fill,
                                                        cfg,
                                                    );
                                                    target.PopAxisAlignedClip();

                                                    // Smooth feathered alpha transition band across [edge_x - 3.0, edge_x + 3.0]
                                                    let clip_feather = D2D_RECT_F {
                                                        left: (edge_x - 3.0).max(rs.x - 2.0),
                                                        top: rs.y - 4.0,
                                                        right: (edge_x + 3.0)
                                                            .min(rs.x + rs.width + 2.0),
                                                        bottom: rs.y + line_height + 4.0,
                                                    };
                                                    target.PushAxisAlignedClip(
                                                        &clip_feather,
                                                        D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                                                    );
                                                    let feather_sung = D2D1_COLOR_F {
                                                        a: sung_fill.a * 0.70,
                                                        ..sung_fill
                                                    };
                                                    draw_text_with_shadow(
                                                        target,
                                                        &reusable_brush,
                                                        &rs.layout,
                                                        rs.x,
                                                        rs.y,
                                                        &feather_sung,
                                                        cfg,
                                                    );
                                                    target.PopAxisAlignedClip();
                                                }
                                            }
                                            "word_glow" => {
                                                let glow_color = D2D1_COLOR_F {
                                                    a: 0.35,
                                                    ..active_karaoke_color
                                                };
                                                reusable_brush.SetColor(&glow_color);
                                                for (dx, dy) in [
                                                    (-1.0, 0.0),
                                                    (1.0, 0.0),
                                                    (0.0, -1.0),
                                                    (0.0, 1.0),
                                                ] {
                                                    target.DrawTextLayout(
                                                        D2D_POINT_2F {
                                                            x: rs.x + dx,
                                                            y: rs.y + dy,
                                                        },
                                                        &rs.layout,
                                                        &reusable_brush,
                                                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                                                    );
                                                }
                                                reusable_brush.SetColor(&active_karaoke_color);
                                                target.DrawTextLayout(
                                                    D2D_POINT_2F { x: rs.x, y: rs.y },
                                                    &rs.layout,
                                                    &reusable_brush,
                                                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                                                );
                                            }
                                            "zoom" | "scale" => {
                                                let zoom_peak =
                                                    (rs.progress * std::f32::consts::PI).sin();
                                                let scale_val = 1.0 + (zoom_peak * 0.22);
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );

                                                let cx = rs.x + (rs.width / 2.0);
                                                let cy = rs.y + (line_height / 2.0);

                                                if cfg.shadow_enabled {
                                                    draw_text_with_shadow(
                                                        target,
                                                        &reusable_brush,
                                                        &rs.layout,
                                                        rs.x,
                                                        rs.y,
                                                        &blended,
                                                        cfg,
                                                    );
                                                }

                                                let transform = Matrix3x2 {
                                                    M11: scale_val,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: scale_val,
                                                    M31: cx * (1.0 - scale_val),
                                                    M32: cy * (1.0 - scale_val),
                                                };

                                                target.SetTransform(&transform);
                                                reusable_brush.SetColor(&blended);
                                                target.DrawTextLayout(
                                                    D2D_POINT_2F { x: rs.x, y: rs.y },
                                                    &rs.layout,
                                                    &reusable_brush,
                                                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                                                );
                                                let identity = Matrix3x2 {
                                                    M11: 1.0,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: 1.0,
                                                    M31: 0.0,
                                                    M32: 0.0,
                                                };
                                                target.SetTransform(&identity);
                                            }
                                            "bounce" | "drop" => {
                                                let bounce =
                                                    (rs.progress * std::f32::consts::PI * 2.0)
                                                        .sin()
                                                        .abs()
                                                        * (1.0 - rs.progress)
                                                        * 9.0;
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y - bounce,
                                                    &blended,
                                                    cfg,
                                                );
                                            }
                                            "slide" | "slide_right" => {
                                                let offset_x = (1.0 - rs.progress).powi(2) * -12.0;
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x + offset_x,
                                                    rs.y,
                                                    &blended,
                                                    cfg,
                                                );
                                            }
                                            "tilt" | "rotate" => {
                                                let angle_rad =
                                                    (rs.progress * std::f32::consts::PI * 2.0)
                                                        .sin()
                                                        * 0.10;
                                                let cos_a = angle_rad.cos();
                                                let sin_a = angle_rad.sin();
                                                let cx = rs.x + (rs.width / 2.0);
                                                let cy = rs.y + (line_height / 2.0);
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );

                                                let transform = Matrix3x2 {
                                                    M11: cos_a,
                                                    M12: sin_a,
                                                    M21: -sin_a,
                                                    M22: cos_a,
                                                    M31: cx * (1.0 - cos_a) + cy * sin_a,
                                                    M32: cy * (1.0 - cos_a) - cx * sin_a,
                                                };

                                                target.SetTransform(&transform);
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &blended,
                                                    cfg,
                                                );
                                                let identity = Matrix3x2 {
                                                    M11: 1.0,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: 1.0,
                                                    M31: 0.0,
                                                    M32: 0.0,
                                                };
                                                target.SetTransform(&identity);
                                            }
                                            "stretch" | "squish" => {
                                                let phase =
                                                    (rs.progress * std::f32::consts::PI).sin();
                                                let scale_x = 1.0 + (phase * 0.25);
                                                let scale_y = 1.0 - (phase * 0.18);
                                                let cx = rs.x + (rs.width / 2.0);
                                                let cy = rs.y + (line_height / 2.0);
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );

                                                let transform = Matrix3x2 {
                                                    M11: scale_x,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: scale_y,
                                                    M31: cx * (1.0 - scale_x),
                                                    M32: cy * (1.0 - scale_y),
                                                };

                                                target.SetTransform(&transform);
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &blended,
                                                    cfg,
                                                );
                                                let identity = Matrix3x2 {
                                                    M11: 1.0,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: 1.0,
                                                    M31: 0.0,
                                                    M32: 0.0,
                                                };
                                                target.SetTransform(&identity);
                                            }
                                            "shimmer" | "flash" => {
                                                let flash_factor =
                                                    (1.0 - rs.progress).powi(3) * 0.6;
                                                let base_color = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );
                                                let flash_color = D2D1_COLOR_F {
                                                    r: (base_color.r + flash_factor).min(1.0),
                                                    g: (base_color.g + flash_factor).min(1.0),
                                                    b: (base_color.b + flash_factor).min(1.0),
                                                    a: active_karaoke_color.a,
                                                };
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &flash_color,
                                                    cfg,
                                                );
                                            }
                                            "neon" | "rainbow" => {
                                                let hue = (rs.progress * 3.0) % 1.0;
                                                let r =
                                                    ((hue * 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0);
                                                let g =
                                                    (2.0 - (hue * 6.0 - 2.0).abs()).clamp(0.0, 1.0);
                                                let b =
                                                    (2.0 - (hue * 6.0 - 4.0).abs()).clamp(0.0, 1.0);
                                                let rainbow_color = D2D1_COLOR_F {
                                                    r,
                                                    g,
                                                    b,
                                                    a: active_karaoke_color.a,
                                                };
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &rainbow_color,
                                                    cfg,
                                                );
                                            }
                                            "float" | "hover" => {
                                                let float_y =
                                                    (rs.progress * std::f32::consts::PI * 2.0)
                                                        .sin()
                                                        * -4.5;
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y + float_y,
                                                    &blended,
                                                    cfg,
                                                );
                                            }
                                            _ => {
                                                let pop_factor = if rs.progress < 0.25 {
                                                    let t = rs.progress / 0.25;
                                                    (t * std::f32::consts::FRAC_PI_2).sin()
                                                } else {
                                                    let t = (rs.progress - 0.25) / 0.75;
                                                    (1.0 - t).powi(3)
                                                };

                                                let scale_val = 1.0 + (pop_factor * 0.08);
                                                let y_shift = -(pop_factor * 2.0);

                                                let cx = rs.x + (rs.width / 2.0);
                                                let cy = rs.y + (line_height / 2.0);

                                                if cfg.shadow_enabled {
                                                    draw_text_with_shadow(
                                                        target,
                                                        &reusable_brush,
                                                        &rs.layout,
                                                        rs.x,
                                                        rs.y,
                                                        &active_karaoke_color,
                                                        cfg,
                                                    );
                                                }

                                                let transform = Matrix3x2 {
                                                    M11: scale_val,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: scale_val,
                                                    M31: cx * (1.0 - scale_val),
                                                    M32: cy * (1.0 - scale_val) + y_shift,
                                                };

                                                target.SetTransform(&transform);
                                                reusable_brush.SetColor(&active_karaoke_color);

                                                target.DrawTextLayout(
                                                    D2D_POINT_2F { x: rs.x, y: rs.y },
                                                    &rs.layout,
                                                    &reusable_brush,
                                                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                                                );

                                                let identity = Matrix3x2 {
                                                    M11: 1.0,
                                                    M12: 0.0,
                                                    M21: 0.0,
                                                    M22: 1.0,
                                                    M31: 0.0,
                                                    M32: 0.0,
                                                };
                                                target.SetTransform(&identity);
                                            }
                                        }
                                    }

                                    // Render inline background syllables as a separate row below
                                    if has_inline_bg_syls && !bg_prep_syls.is_empty() {
                                        let bg_gap = 6.0f32;
                                        let bg_row_y = main_text_bottom + bg_gap;

                                        // Build word units for bg syllables
                                        let mut bg_word_units: Vec<WordUnitKaraoke> = Vec::new();
                                        let mut bg_cur_items = Vec::new();
                                        let mut bg_cur_w = 0.0f32;
                                        let mut bg_prev_sp = true;

                                        for ps in &bg_prep_syls {
                                            let t = &ps.syl.text;
                                            let starts_sp =
                                                t.starts_with(' ') || t.starts_with('\t');
                                            let is_new = bg_prev_sp || starts_sp;
                                            if is_new && !bg_cur_items.is_empty() {
                                                bg_word_units.push(WordUnitKaraoke {
                                                    items: bg_cur_items,
                                                    width: bg_cur_w,
                                                });
                                                bg_cur_items = Vec::new();
                                                bg_cur_w = 0.0;
                                            }
                                            bg_prev_sp = t.ends_with(' ') || t.ends_with('\t');
                                            bg_cur_w += ps.width;
                                            bg_cur_items.push(ps);
                                        }
                                        if !bg_cur_items.is_empty() {
                                            bg_word_units.push(WordUnitKaraoke {
                                                items: bg_cur_items,
                                                width: bg_cur_w,
                                            });
                                        }

                                        // Build visual lines for bg
                                        let mut bg_vlines: Vec<VisualLineKaraoke> = Vec::new();
                                        let mut bg_v_items = Vec::new();
                                        let mut bg_v_w = 0.0f32;
                                        let mut bg_v_text = String::new();
                                        for unit in bg_word_units {
                                            if bg_v_w + unit.width <= max_w {
                                                bg_v_w += unit.width;
                                                for ps in unit.items {
                                                    bg_v_text.push_str(&ps.syl.text);
                                                    bg_v_items.push(ps);
                                                }
                                            } else {
                                                if !bg_v_items.is_empty() {
                                                    bg_vlines.push(VisualLineKaraoke {
                                                        items: bg_v_items,
                                                        width: bg_v_w,
                                                        snippet: bg_v_text,
                                                    });
                                                    bg_v_items = Vec::new();
                                                    bg_v_text = String::new();
                                                }
                                                bg_v_w = unit.width;
                                                for ps in unit.items {
                                                    bg_v_text.push_str(&ps.syl.text);
                                                    bg_v_items.push(ps);
                                                }
                                            }
                                        }
                                        if !bg_v_items.is_empty() {
                                            bg_vlines.push(VisualLineKaraoke {
                                                items: bg_v_items,
                                                width: bg_v_w,
                                                snippet: bg_v_text,
                                            });
                                        }

                                        // Render bg visual lines
                                        let mut bg_y = bg_row_y;
                                        let mut bg_karaoke_color = active_karaoke_color;
                                        bg_karaoke_color.a = (bg_karaoke_color.a * 0.75).min(0.75);
                                        let mut bg_text_color = active_text_color;
                                        bg_text_color.a = (bg_text_color.a * 0.75).min(0.75);

                                        for vline in &bg_vlines {
                                            let bg_origin = line_origin_x(
                                                target_idx as usize,
                                                line.singer_index,
                                                width_f,
                                                vline.width,
                                                &vline.snippet,
                                                cfg,
                                            );
                                            let mut bx = bg_origin;
                                            for ps in &vline.items {
                                                let syl_color = if ps.progress >= 1.0 {
                                                    bg_karaoke_color
                                                } else if ps.progress > 0.0 {
                                                    lerp_d2d_color(
                                                        &bg_text_color,
                                                        &bg_karaoke_color,
                                                        ps.progress,
                                                    )
                                                } else {
                                                    bg_text_color
                                                };
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &ps.layout,
                                                    bx,
                                                    bg_y,
                                                    &syl_color,
                                                    cfg,
                                                );
                                                bx += ps.width;
                                            }
                                            bg_y += bg_line_height_k;
                                        }
                                        main_text_bottom = bg_y;
                                    }

                                    if let Some(ref raw_sub) = line.sub_text {
                                        if !raw_sub.is_empty() {
                                            let formatted_sub = line
                                                .get_formatted_sub_text()
                                                .unwrap_or_else(|| raw_sub.clone());
                                            let font_size_sub_capped =
                                                (cfg.font_size_sub.clamp(14, 40) as f32)
                                                    * responsive_scale;
                                            let min_gap = 6.0f32;
                                            let available_bottom =
                                                height_f - font_size_sub_capped - 24.0;
                                            let minimal_y = main_text_bottom + min_gap;

                                            if minimal_y <= available_bottom {
                                                let sub_y = minimal_y;

                                                let sub_layout = engine.get_cached_text_layout(
                                                    &formatted_sub,
                                                    &cfg.font_family,
                                                    font_size_sub_capped,
                                                    false,
                                                    width_f - 40.0,
                                                    font_size_sub_capped + 20.0,
                                                )?;

                                                let mut sub_metrics =
                                                    DWRITE_TEXT_METRICS::default();
                                                sub_layout.GetMetrics(&mut sub_metrics)?;
                                                let sub_width =
                                                    sub_metrics.widthIncludingTrailingWhitespace;

                                                let sub_x = line_origin_x(
                                                    target_idx as usize,
                                                    line.singer_index,
                                                    width_f,
                                                    sub_width,
                                                    &formatted_sub,
                                                    cfg,
                                                );

                                                // Draw subtle rounded pill container matching Image 1
                                                let pill_px = 10.0f32;
                                                let pill_py = 3.0f32;
                                                let pill_rect = D2D1_ROUNDED_RECT {
                                                    rect: D2D_RECT_F {
                                                        left: sub_x - pill_px,
                                                        top: sub_y - pill_py,
                                                        right: sub_x + sub_width + pill_px,
                                                        bottom: sub_y
                                                            + sub_metrics.height
                                                            + pill_py,
                                                    },
                                                    radiusX: 12.0,
                                                    radiusY: 12.0,
                                                };

                                                let pill_bg = hex_to_d2d_color("0c0c14", 0.45);
                                                reusable_brush.SetColor(&pill_bg);
                                                target.FillRoundedRectangle(
                                                    &pill_rect,
                                                    &reusable_brush,
                                                );
                                                let pill_border = hex_to_d2d_color("44445c", 0.45);
                                                reusable_brush.SetColor(&pill_border);
                                                target.DrawRoundedRectangle(
                                                    &pill_rect,
                                                    &reusable_brush,
                                                    1.0,
                                                    None,
                                                );

                                                // Base dim text layer
                                                let sub_dim_color =
                                                    hex_to_d2d_color(&cfg.sub_hex, 0.45);
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &sub_layout,
                                                    sub_x,
                                                    sub_y,
                                                    &sub_dim_color,
                                                    cfg,
                                                );

                                                let sub_karaoke_on =
                                                    cfg.sub_karaoke_enabled.unwrap_or(true);
                                                if sub_karaoke_on {
                                                    let sub_syllables = line.get_sub_syllables();
                                                    let active_sub_hex_str = cfg
                                                        .sub_active_hex
                                                        .as_deref()
                                                        .unwrap_or("ffffff");
                                                    let sub_active_color =
                                                        hex_to_d2d_color(active_sub_hex_str, 1.0);

                                                    let sub_progress = if !sub_syllables.is_empty()
                                                    {
                                                        let total_sub_time: u64 = sub_syllables
                                                            .iter()
                                                            .map(|s| s.duration.as_millis() as u64)
                                                            .sum();
                                                        let mut accum_t = 0u64;
                                                        let mut prog = 0.0f32;

                                                        for sub_syl in &sub_syllables {
                                                            let dur = sub_syl
                                                                .duration
                                                                .as_millis()
                                                                .max(50)
                                                                as u64;
                                                            let syl_start = accum_t;
                                                            accum_t += dur;

                                                            if elapsed_line >= syl_start {
                                                                let local_p = ((elapsed_line
                                                                    - syl_start)
                                                                    as f32
                                                                    / dur as f32)
                                                                    .clamp(0.0, 1.0);
                                                                let syl_weight = dur as f32
                                                                    / total_sub_time.max(1) as f32;
                                                                prog += local_p * syl_weight;
                                                            }
                                                        }
                                                        prog.clamp(0.0, 1.0)
                                                    } else {
                                                        let line_dur =
                                                            if let Some(end) = line.end_time {
                                                                end.saturating_sub(line.time)
                                                                    .as_millis()
                                                                    .max(1)
                                                                    as u64
                                                            } else {
                                                                4000
                                                            };
                                                        (elapsed_line as f32 / line_dur as f32)
                                                            .clamp(0.0, 1.0)
                                                    };

                                                    let fill_w = sub_width * sub_progress;
                                                    if fill_w > 0.0 {
                                                        let clip_rect = D2D_RECT_F {
                                                            left: sub_x - pill_px,
                                                            top: sub_y - pill_py - 2.0,
                                                            right: sub_x + fill_w,
                                                            bottom: sub_y
                                                                + sub_metrics.height
                                                                + pill_py
                                                                + 2.0,
                                                        };
                                                        target.PushAxisAlignedClip(
                                                            &clip_rect,
                                                            D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                                                        );
                                                        draw_text_with_shadow(
                                                            target,
                                                            &reusable_brush,
                                                            &sub_layout,
                                                            sub_x,
                                                            sub_y,
                                                            &sub_active_color,
                                                            cfg,
                                                        );
                                                        target.PopAxisAlignedClip();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            let font_size_side_capped = (cfg.font_size_side.min(40)) as f32;
                            let eff_side_font_size = if line.is_background {
                                font_size_side_capped * 0.70
                            } else {
                                font_size_side_capped
                            };
                            let side_layout = engine.get_cached_text_layout(
                                &line.text,
                                &cfg.font_family,
                                eff_side_font_size,
                                false,
                                width_f - 30.0,
                                eff_side_font_size + 20.0,
                            )?;
                            let mut side_metrics = DWRITE_TEXT_METRICS::default();
                            side_layout.GetMetrics(&mut side_metrics)?;
                            let side_width = side_metrics.widthIncludingTrailingWhitespace;
                            let side_x = line_origin_x(
                                target_idx as usize,
                                line.singer_index,
                                width_f,
                                side_width,
                                &line.text,
                                cfg,
                            );

                            let distance = (target_idx as f32 - float_idx).abs();
                            let base_side_alpha = if distance > 0.8 { 0.4 } else { 0.75 };
                            let side_alpha = if line.is_background {
                                base_side_alpha * 0.75
                            } else {
                                base_side_alpha
                            };
                            let side_color = hex_to_d2d_color(&cfg.side_hex, side_alpha);
                            draw_text_with_shadow(
                                target,
                                &reusable_brush,
                                &side_layout,
                                side_x,
                                line_top,
                                &side_color,
                                cfg,
                            );
                        }
                    }
                }
            } else {
                let idle_layout = engine.get_cached_text_layout(
                    "Play music to see lyrics...",
                    &cfg.font_family,
                    12.0,
                    false,
                    width_f - 30.0,
                    30.0,
                )?;
                let idle_color = hex_to_d2d_color("777777", 0.9);
                draw_text_with_shadow(
                    target,
                    &reusable_brush,
                    &idle_layout,
                    15.0,
                    base_center_y,
                    &idle_color,
                    cfg,
                );
            }
        }
    }

    target.EndDraw(None, None)
}
