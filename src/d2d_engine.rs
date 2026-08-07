use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
    D2D1_RENDER_TARGET_USAGE_NONE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, IDWriteTextLayout,
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT_BOLD, DWRITE_FONT_WEIGHT_NORMAL,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::HDC;

static D2D_ENGINE_INSTANCE: OnceLock<D2DEngine> = OnceLock::new();

pub fn get_d2d_engine() -> &'static D2DEngine {
    D2D_ENGINE_INSTANCE.get_or_init(|| {
        D2DEngine::new().expect("Failed to initialize Direct2D / DirectWrite engine")
    })
}

pub fn hex_to_d2d_color(hex: &str, alpha: f32) -> D2D1_COLOR_F {
    let clean = hex.trim_start_matches('#');
    let val = u32::from_str_radix(clean, 16).unwrap_or(0xFFFFFF);
    let r = ((val >> 16) & 0xFF) as f32 / 255.0;
    let g = ((val >> 8) & 0xFF) as f32 / 255.0;
    let b = (val & 0xFF) as f32 / 255.0;
    D2D1_COLOR_F {
        r,
        g,
        b,
        a: alpha.clamp(0.0, 1.0),
    }
}

/// Linear-interpolate between two colors by t in [0, 1]. Used by the
/// "fade" and "wave" karaoke effects to crossfade base -> karaoke color
/// as a syllable's progress advances, instead of a hard cut.
pub fn lerp_d2d_color(from: &D2D1_COLOR_F, to: &D2D1_COLOR_F, t: f32) -> D2D1_COLOR_F {
    let t = t.clamp(0.0, 1.0);
    D2D1_COLOR_F {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

type TextFormatKey = (String, u32, bool);
// text + format key + max_w/max_h (rounded to int) -> layout
type TextLayoutKey = (String, TextFormatKey, u32, u32);

pub struct D2DEngine {
    pub d2d_factory: ID2D1Factory,
    pub dwrite_factory: IDWriteFactory,
    text_format_cache: Mutex<HashMap<TextFormatKey, IDWriteTextFormat>>,
    text_layout_cache: Mutex<HashMap<TextLayoutKey, IDWriteTextLayout>>,
}

unsafe impl Send for D2DEngine {}
unsafe impl Sync for D2DEngine {}

impl D2DEngine {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            let d2d_factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            Ok(Self {
                d2d_factory,
                dwrite_factory,
                text_format_cache: Mutex::new(HashMap::new()),
                text_layout_cache: Mutex::new(HashMap::new()),
            })
        }
    }

    pub unsafe fn create_dc_render_target(
        &self,
        hdc: HDC,
        rect: &RECT,
    ) -> windows::core::Result<ID2D1DCRenderTarget> {
        let props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 0.0,
            dpiY: 0.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };

        let target = self.d2d_factory.CreateDCRenderTarget(&props)?;
        target.BindDC(hdc, rect)?;
        Ok(target)
    }

    pub unsafe fn get_text_format(
        &self,
        font_family: &str,
        size: f32,
        bold: bool,
    ) -> windows::core::Result<IDWriteTextFormat> {
        let size_key = (size * 10.0) as u32;
        let key = (font_family.to_string(), size_key, bold);

        if let Ok(cache) = self.text_format_cache.lock() {
            if let Some(format) = cache.get(&key) {
                return Ok(format.clone());
            }
        }

        let family_hstring = HSTRING::from(font_family);
        let weight = if bold {
            DWRITE_FONT_WEIGHT_BOLD
        } else {
            DWRITE_FONT_WEIGHT_NORMAL
        };
        let format = self.dwrite_factory.CreateTextFormat(
            PCWSTR(family_hstring.as_ptr()),
            None,
            weight,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            PCWSTR(HSTRING::from("en-us").as_ptr()),
        )?;

        if let Ok(mut cache) = self.text_format_cache.lock() {
            cache.insert(key, format.clone());
        }

        Ok(format)
    }

    pub unsafe fn create_text_layout(
        &self,
        text: &str,
        format: &IDWriteTextFormat,
        max_w: f32,
        max_h: f32,
    ) -> windows::core::Result<IDWriteTextLayout> {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        self.dwrite_factory
            .CreateTextLayout(&utf16, format, max_w, max_h)
    }

    /// Same as get_text_format + create_text_layout, but reuses a previously
    /// built layout when the text/font/size/box haven't changed. This is the
    /// hot path called every repaint (up to 60x/sec while a song plays), so
    /// avoiding CreateTextLayout on every frame is the single biggest win
    /// for CPU/GPU usage in this app.
    pub unsafe fn get_cached_text_layout(
        &self,
        text: &str,
        font_family: &str,
        size: f32,
        bold: bool,
        max_w: f32,
        max_h: f32,
    ) -> windows::core::Result<IDWriteTextLayout> {
        let size_key = (size * 10.0) as u32;
        let format_key: TextFormatKey = (font_family.to_string(), size_key, bold);
        let key: TextLayoutKey = (
            text.to_string(),
            format_key,
            max_w.round() as u32,
            max_h.round() as u32,
        );

        if let Ok(cache) = self.text_layout_cache.lock() {
            if let Some(layout) = cache.get(&key) {
                return Ok(layout.clone());
            }
        }

        let format = self.get_text_format(font_family, size, bold)?;
        let layout = self.create_text_layout(text, &format, max_w, max_h)?;

        if let Ok(mut cache) = self.text_layout_cache.lock() {
            // Cheap safety valve: if a song has pathologically many distinct
            // strings (shouldn't happen in practice), drop the cache instead
            // of growing forever.
            if cache.len() > 512 {
                cache.clear();
            }
            cache.insert(key, layout.clone());
        }

        Ok(layout)
    }

    /// Call this whenever the track/lyrics change so stale strings don't
    /// linger in memory forever.
    pub fn clear_layout_cache(&self) {
        if let Ok(mut cache) = self.text_layout_cache.lock() {
            cache.clear();
        }
    }
}
