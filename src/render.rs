#![allow(static_mut_refs)]
use crate::app_state::APP_STATE;
use crate::config::StyleConfig;
use crate::d2d_engine::{hex_to_d2d_color, lerp_d2d_color, D2DEngine};
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
        let alpha = (cfg.shadow_opacity * 0.35).clamp(0.05, 0.45);
        let shadow_color = hex_to_d2d_color(&cfg.shadow_hex, alpha);
        brush.SetColor(&shadow_color);

        let ox = cfg.shadow_offset_x * 0.5;
        let oy = cfg.shadow_offset_y * 0.5;

        let offsets = [
            (-0.8, -0.8),
            (0.8, -0.8),
            (-0.8, 0.8),
            (0.8, 0.8),
            (0.0, 1.0),
            (1.0, 1.0),
        ];

        for (dx, dy) in offsets {
            target.DrawTextLayout(
                D2D_POINT_2F {
                    x: x + dx + ox,
                    y: y + dy + oy,
                },
                layout,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        }
    }
    brush.SetColor(text_color);
    target.DrawTextLayout(
        D2D_POINT_2F { x, y },
        layout,
        brush,
        D2D1_DRAW_TEXT_OPTIONS_NONE,
    );
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
                    (width_f - 60.0).max(50.0),
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
                    (width_f - 60.0).max(50.0),
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

            let active_font_size = (cfg.font_size_active.min(40)) as f32;
            let font_size_sub_capped = cfg.font_size_sub.clamp(14, 40) as f32;
            let _active_karaoke_color = hex_to_d2d_color(&cfg.karaoke_hex, 1.0);
            let active_text_color = hex_to_d2d_color(&cfg.active_hex, 1.0);

            let max_w = width_f - 30.0;
            let mut active_h = active_font_size + 4.0;
            let current_idx = s.current_index;

            if current_idx < s.lyrics_lines.len() {
                let active_line = &s.lyrics_lines[current_idx];
                let mut cx = 15.0;
                let mut clines = 1.0;
                let lh = active_font_size + 4.0;

                for syl in &active_line.syllables {
                    let layout = engine.get_cached_text_layout(
                        &syl.text,
                        &cfg.font_family,
                        active_font_size,
                        true,
                        max_w,
                        lh,
                    )?;
                    let mut metrics = DWRITE_TEXT_METRICS::default();
                    layout.GetMetrics(&mut metrics)?;
                    let is_word_token = syl
                        .text
                        .chars()
                        .last()
                        .is_some_and(|c| c.is_alphanumeric() || c == '\'' || c == '`');
                    let padding = if syl.text.ends_with(' ') {
                        0.0
                    } else if is_word_token {
                        (active_font_size * 0.25).max(6.0)
                    } else {
                        2.0
                    };
                    let syl_width = metrics.widthIncludingTrailingWhitespace + padding;

                    if cx + syl_width > max_w && cx > 15.0 {
                        cx = 15.0;
                        clines += 1.0;
                    }
                    cx += syl_width;
                }
                active_h = clines * lh;
                if active_line
                    .sub_text
                    .as_ref()
                    .is_some_and(|sub| !sub.is_empty())
                {
                    active_h += font_size_sub_capped + 14.0;
                }
            }

            let max_center_y = (height_f - active_h - 24.0).max(header_bottom + 4.0);
            let active_center_y = base_center_y.clamp(header_bottom + 4.0, max_center_y);

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

                        let line_top = if offset <= 0 {
                            active_center_y + distance_from_float * base_step
                        } else {
                            active_center_y
                                + active_h
                                + 12.0
                                + (distance_from_float - 1.0) * base_step
                        };

                        if offset != 0 && line_top < header_bottom {
                            continue;
                        }

                        let active_karaoke_color = if line.singer_index > 0 {
                            hex_to_d2d_color(&cfg.karaoke_v2_hex, 1.0)
                        } else {
                            hex_to_d2d_color(&cfg.karaoke_hex, 1.0)
                        };

                        let is_active = offset == 0;
                        let is_instrumental = line.text.trim() == "♪"
                            || line.text.to_lowercase().contains("instrumental")
                            || line.text.contains('♪');

                        if is_active {
                            if is_instrumental {
                                let note_text = if line.text.trim().is_empty() {
                                    "♪"
                                } else {
                                    &line.text
                                };
                                let note_layout = engine.get_cached_text_layout(
                                    note_text,
                                    &cfg.font_family,
                                    active_font_size,
                                    true,
                                    width_f - 30.0,
                                    active_font_size + 20.0,
                                )?;
                                draw_text_with_shadow(
                                    target,
                                    &reusable_brush,
                                    &note_layout,
                                    15.0,
                                    line_top,
                                    &active_karaoke_color,
                                    cfg,
                                );
                            } else {
                                let karaoke_mode = cfg.karaoke_mode.trim().to_lowercase();
                                let use_karaoke = match karaoke_mode.as_str() {
                                    "always" => true,
                                    "never" => false,
                                    _ => line.is_karaoke, // "auto"
                                };

                                if !use_karaoke {
                                    let mut current_x = 15.0;
                                    let mut current_y = line_top;
                                    let line_height = active_font_size + 4.0;
                                    let mut render_data = Vec::new();

                                    for syl in &line.syllables {
                                        let layout = engine.get_cached_text_layout(
                                            &syl.text,
                                            &cfg.font_family,
                                            active_font_size,
                                            true,
                                            max_w,
                                            line_height,
                                        )?;
                                        let mut metrics = DWRITE_TEXT_METRICS::default();
                                        layout.GetMetrics(&mut metrics)?;

                                        let is_word_token =
                                            syl.text.chars().last().is_some_and(|c| {
                                                c.is_alphanumeric() || c == '\'' || c == '`'
                                            });
                                        let _padding = if syl.text.ends_with(' ') {
                                            0.0
                                        } else if is_word_token {
                                            (active_font_size * 0.25).max(6.0)
                                        } else {
                                            2.0
                                        };
                                        let syl_width = metrics.widthIncludingTrailingWhitespace;

                                        if current_x + syl_width > max_w && current_x > 15.0 {
                                            current_x = 15.0;
                                            current_y += line_height;
                                        }

                                        render_data.push((current_x, current_y, layout));
                                        current_x += syl_width;
                                    }

                                    let total_box_height = (current_y - line_top) + line_height;

                                    for (x, y, layout) in &render_data {
                                        draw_text_with_shadow(
                                            target,
                                            &reusable_brush,
                                            layout,
                                            *x,
                                            *y,
                                            &active_karaoke_color,
                                            cfg,
                                        );
                                    }

                                    if let Some(ref sub) = line.sub_text {
                                        if !sub.is_empty() {
                                            let font_size_sub_capped =
                                                cfg.font_size_sub.clamp(14, 40) as f32;
                                            let sub_layout = engine.get_cached_text_layout(
                                                sub,
                                                &cfg.font_family,
                                                font_size_sub_capped,
                                                false,
                                                width_f - 30.0,
                                                font_size_sub_capped + 20.0,
                                            )?;
                                            let sub_color = hex_to_d2d_color(&cfg.sub_hex, 0.95);
                                            let sub_y = (line_top + total_box_height + 1.0)
                                                .min(height_f - font_size_sub_capped - 20.0);
                                            draw_text_with_shadow(
                                                target,
                                                &reusable_brush,
                                                &sub_layout,
                                                15.0,
                                                sub_y,
                                                &sub_color,
                                                cfg,
                                            );
                                        }
                                    }
                                } else {
                                    let start_ms = line.time.as_millis() as u64;
                                    let elapsed_line = adjusted_ms.saturating_sub(start_ms);

                                    let mut current_x = 15.0;
                                    let mut current_y = line_top;
                                    let line_height = active_font_size + 4.0;
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

                                        let layout = engine.get_cached_text_layout(
                                            &syl.text,
                                            &cfg.font_family,
                                            active_font_size,
                                            true,
                                            max_w,
                                            line_height,
                                        )?;
                                        let mut metrics = DWRITE_TEXT_METRICS::default();
                                        layout.GetMetrics(&mut metrics)?;

                                        let is_word_token =
                                            syl.text.chars().last().is_some_and(|c| {
                                                c.is_alphanumeric() || c == '\'' || c == '`'
                                            });
                                        let _padding = if syl.text.ends_with(' ') {
                                            0.0
                                        } else if is_word_token {
                                            (active_font_size * 0.25).max(6.0)
                                        } else {
                                            2.0
                                        };
                                        let syl_width = metrics.widthIncludingTrailingWhitespace;

                                        if current_x + syl_width > max_w && current_x > 15.0 {
                                            current_x = 15.0;
                                            current_y += line_height;
                                        }

                                        render_data.push(RenderSyl {
                                            x: current_x,
                                            y: current_y,
                                            width: syl_width,
                                            progress: syl_progress,
                                            layout,
                                        });

                                        current_x += syl_width;
                                    }

                                    let total_box_height = (current_y - line_top) + line_height;

                                    for rs in &render_data {
                                        let is_current_syl = rs.progress > 0.0 && rs.progress < 1.0;
                                        if is_current_syl {
                                            continue;
                                        }

                                        let syl_color = if rs.progress >= 1.0 {
                                            active_karaoke_color
                                        } else {
                                            active_text_color
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

                                    for rs in &render_data {
                                        let is_current_syl = rs.progress > 0.0 && rs.progress < 1.0;
                                        if !is_current_syl {
                                            continue;
                                        }

                                        let effect = cfg.karaoke_effect.trim().to_lowercase();

                                        match effect.as_str() {
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
                                            "sweep" | "kf" => {
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x,
                                                    rs.y,
                                                    &active_text_color,
                                                    cfg,
                                                );
                                                let fill_w = rs.width * rs.progress;
                                                if fill_w > 0.0 {
                                                    let clip_rect = D2D_RECT_F {
                                                        left: rs.x,
                                                        top: rs.y - 2.0,
                                                        right: rs.x + fill_w,
                                                        bottom: rs.y + line_height + 2.0,
                                                    };
                                                    target.PushAxisAlignedClip(
                                                        &clip_rect,
                                                        D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
                                                    );
                                                    reusable_brush.SetColor(&active_karaoke_color);
                                                    target.DrawTextLayout(
                                                        D2D_POINT_2F { x: rs.x, y: rs.y },
                                                        &rs.layout,
                                                        &reusable_brush,
                                                        D2D1_DRAW_TEXT_OPTIONS_NONE,
                                                    );
                                                    target.PopAxisAlignedClip();
                                                }
                                            }
                                            "pulse" => {
                                                // Smooth scale-up-then-down breathing pulse,
                                                // centered on the syllable box, no vertical
                                                // shift (distinct from "pop"'s lift+shrink).
                                                let pulse =
                                                    (rs.progress * std::f32::consts::PI).sin();
                                                let scale_val = 1.0 + (pulse * 0.15);
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
                                            "shake" => {
                                                // Horizontal jitter that decays as the
                                                // syllable's progress advances toward 1.0,
                                                // settling once it's fully highlighted.
                                                let jitter = (rs.progress * 40.0).sin()
                                                    * (1.0 - rs.progress)
                                                    * 2.0;
                                                let blended = lerp_d2d_color(
                                                    &active_text_color,
                                                    &active_karaoke_color,
                                                    rs.progress,
                                                );
                                                draw_text_with_shadow(
                                                    target,
                                                    &reusable_brush,
                                                    &rs.layout,
                                                    rs.x + jitter,
                                                    rs.y,
                                                    &blended,
                                                    cfg,
                                                );
                                            }
                                            "rise" => {
                                                // Syllable slides up into place from a few
                                                // pixels below while crossfading to the
                                                // karaoke color, settling at progress = 1.0.
                                                let rise_amount = (1.0 - rs.progress).powi(2) * 8.0;
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
                                                    rs.y + rise_amount,
                                                    &blended,
                                                    cfg,
                                                );
                                            }
                                            "glow" => {
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

                                    if let Some(ref sub) = line.sub_text {
                                        if !sub.is_empty() {
                                            let font_size_sub_capped =
                                                cfg.font_size_sub.clamp(14, 40) as f32;
                                            let sub_layout = engine.get_cached_text_layout(
                                                sub,
                                                &cfg.font_family,
                                                font_size_sub_capped,
                                                false,
                                                width_f - 30.0,
                                                font_size_sub_capped + 20.0,
                                            )?;
                                            let sub_color = hex_to_d2d_color(&cfg.sub_hex, 0.95);
                                            let sub_y = (line_top + total_box_height + 1.0)
                                                .min(height_f - font_size_sub_capped - 20.0);
                                            draw_text_with_shadow(
                                                target,
                                                &reusable_brush,
                                                &sub_layout,
                                                15.0,
                                                sub_y,
                                                &sub_color,
                                                cfg,
                                            );
                                        }
                                    }
                                }
                            }
                        } else {
                            let font_size_side_capped = (cfg.font_size_side.min(40)) as f32;
                            let side_layout = engine.get_cached_text_layout(
                                &line.text,
                                &cfg.font_family,
                                font_size_side_capped,
                                false,
                                width_f - 30.0,
                                font_size_side_capped + 20.0,
                            )?;

                            let distance = (target_idx as f32 - float_idx).abs();
                            let side_alpha = if distance > 0.8 { 0.4 } else { 0.75 };
                            let side_color = hex_to_d2d_color(&cfg.side_hex, side_alpha);
                            draw_text_with_shadow(
                                target,
                                &reusable_brush,
                                &side_layout,
                                15.0,
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
