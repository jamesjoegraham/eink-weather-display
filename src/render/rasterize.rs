// src/svg/render.rs
// SVG rendering logic extracted from svg_render.rs

use std::{error::Error, path::Path};

#[cfg(target_os = "linux")]
use embedded_graphics::{Pixel, prelude::*};
use tiny_skia::Pixmap;
use usvg::Transform;

/// The 7-color ACeP palette of the Waveshare 7.3" e-ink display.
/// Defined without platform gates so it can be used for dithered preview on any OS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EinkColor {
    Black,
    White,
    Green,
    Blue,
    Red,
    Yellow,
    Orange,
}

impl EinkColor {
    /// Returns the representative sRGB value for this palette entry.
    pub fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            EinkColor::Black => (0, 0, 0),
            EinkColor::White => (255, 255, 255),
            EinkColor::Green => (0, 255, 0),
            EinkColor::Blue => (0, 0, 255),
            EinkColor::Red => (255, 0, 0),
            EinkColor::Yellow => (255, 255, 0),
            EinkColor::Orange => (255, 128, 0),
        }
    }
}

fn svg_options_with_fonts() -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    options.font_family = "Roboto".to_owned();
    let fontdb = options.fontdb_mut();
    fontdb.load_system_fonts();
    fontdb.load_font_data(include_bytes!("../../fonts/Roboto-Regular.ttf").to_vec());
    fontdb.load_font_data(include_bytes!("../../fonts/RobotoMono-Regular.ttf").to_vec());
    fontdb.set_serif_family("Roboto");
    fontdb.set_sans_serif_family("Roboto");
    fontdb.set_cursive_family("Roboto");
    fontdb.set_fantasy_family("Roboto");
    fontdb.set_monospace_family("Roboto Mono");
    options
}

pub fn rasterize_svg_to_png(
    svg_data: &[u8],
    out_path: &Path,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn Error>> {
    let options = svg_options_with_fonts();
    let tree = usvg::Tree::from_data(svg_data, &options)?;

    let mut pixmap = Pixmap::new(width, height)
        .ok_or_else(|| format!("invalid raster size {}x{}", width, height))?;

    let svg_size = tree.size().to_int_size();
    let sx = width as f32 / svg_size.width() as f32;
    let sy = height as f32 / svg_size.height() as f32;
    let scale = sx.min(sy);

    let rendered_w = svg_size.width() as f32 * scale;
    let rendered_h = svg_size.height() as f32 * scale;
    let tx = (width as f32 - rendered_w) / 2.0;
    let ty = (height as f32 - rendered_h) / 2.0;

    let transform = Transform::from_row(scale, 0.0, 0.0, scale, tx, ty);
    let _ = resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap.save_png(out_path)?;
    Ok(())
}

/// Renders an SVG at 2× resolution, downsamples each 2×2 block to one pixel
/// by averaging, then maps the result to an `EinkColor` using Bayer-ordered
/// dithering. Returns one entry per pixel in row-major order; `None` means the
/// pixel is transparent (averaged alpha < 10) and should not overwrite the
/// display background.
///
/// Using this shared core ensures the preview PNG and the hardware path apply
/// identical supersampling and dithering logic.
fn render_svg_supersampled(
    svg_data: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<Option<EinkColor>>, Box<dyn Error>> {
    let options = svg_options_with_fonts();
    let tree = usvg::Tree::from_data(svg_data, &options)?;

    let render_w = width * 2;
    let render_h = height * 2;

    let mut hires = Pixmap::new(render_w, render_h)
        .ok_or_else(|| format!("invalid raster size {}x{}", render_w, render_h))?;

    let svg_size = tree.size().to_int_size();
    let sx = render_w as f32 / svg_size.width() as f32;
    let sy = render_h as f32 / svg_size.height() as f32;
    let scale = sx.min(sy);

    let rendered_w = svg_size.width() as f32 * scale;
    let rendered_h = svg_size.height() as f32 * scale;
    let tx = (render_w as f32 - rendered_w) / 2.0;
    let ty = (render_h as f32 - rendered_h) / 2.0;

    let transform = Transform::from_row(scale, 0.0, 0.0, scale, tx, ty);
    let _ = resvg::render(&tree, transform, &mut hires.as_mut());

    // Premultiplied RGBA: for opaque pixels (a≈255) premultiplied == linear,
    // so plain integer averaging across the 2×2 block is correct.
    let src = hires.data();
    let mut out = Vec::with_capacity((width * height) as usize);

    for y in 0..height {
        for x in 0..width {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut a = 0u32;
            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    let i = (((y * 2 + dy) * render_w + (x * 2 + dx)) * 4) as usize;
                    r += src[i] as u32;
                    g += src[i + 1] as u32;
                    b += src[i + 2] as u32;
                    a += src[i + 3] as u32;
                }
            }
            out.push(if a / 4 < 10 {
                None
            } else {
                Some(map_rgb_to_eink_color(
                    (r / 4) as u8,
                    (g / 4) as u8,
                    (b / 4) as u8,
                    x,
                    y,
                ))
            });
        }
    }

    Ok(out)
}

/// Renders an SVG to a PNG simulating e-ink output: each pixel is quantized
/// to the 7-color ACeP palette using the same Bayer-ordered dithering that
/// runs on hardware. Useful for previewing the dithered result without a device.
pub fn rasterize_svg_to_dithered_png(
    svg_data: &[u8],
    out_path: &Path,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn Error>> {
    let colors = render_svg_supersampled(svg_data, width, height)?;

    let mut pixmap = Pixmap::new(width, height)
        .ok_or_else(|| format!("invalid output size {}x{}", width, height))?;

    {
        let data = pixmap.data_mut();
        for (i, color) in colors.iter().enumerate() {
            if let Some(c) = color {
                let (r, g, b) = c.to_rgb();
                let off = i * 4;
                data[off] = r;
                data[off + 1] = g;
                data[off + 2] = b;
                data[off + 3] = 255;
            }
        }
    }

    pixmap.save_png(out_path)?;
    Ok(())
}

#[cfg(target_os = "linux")]
use epd_waveshare::{color::OctColor, epd7in3f::Display7in3f};

#[cfg(target_os = "linux")]
fn eink_to_oct(color: EinkColor) -> OctColor {
    match color {
        EinkColor::Black => OctColor::Black,
        EinkColor::White => OctColor::White,
        EinkColor::Green => OctColor::Green,
        EinkColor::Blue => OctColor::Blue,
        EinkColor::Red => OctColor::Red,
        EinkColor::Yellow => OctColor::Yellow,
        EinkColor::Orange => OctColor::Orange,
    }
}

#[cfg(target_os = "linux")]
pub fn draw_svg_icon(
    display: &mut Display7in3f,
    svg_path: &Path,
    origin: Point,
    size: Size,
) -> Result<(), Box<dyn Error>> {
    let svg_data = std::fs::read(svg_path)?;
    draw_svg_bytes(display, &svg_data, origin, size)
}

#[cfg(target_os = "linux")]
pub fn draw_svg_bytes(
    display: &mut Display7in3f,
    svg_data: &[u8],
    origin: Point,
    size: Size,
) -> Result<(), Box<dyn Error>> {
    let colors = render_svg_supersampled(svg_data, size.width, size.height)?;

    for (idx, maybe_color) in colors.iter().enumerate() {
        if let Some(color) = maybe_color {
            let x = (idx as u32 % size.width) as i32;
            let y = (idx as u32 / size.width) as i32;
            let point = Point::new(origin.x + x, origin.y + y);
            if point.x >= 0 && point.y >= 0 && point.x < 800 && point.y < 480 {
                Pixel(point, eink_to_oct(*color)).draw(display)?;
            }
        }
    }

    Ok(())
}

/// Maps an sRGB pixel to the nearest ACeP palette color using Bayer-ordered
/// dithering. Position-dependent: the same RGB value may map to different
/// palette entries at different (x, y) coordinates.
fn map_rgb_to_eink_color(r: u8, g: u8, b: u8, x: u32, y: u32) -> EinkColor {
    const PALETTE: [(EinkColor, (u8, u8, u8)); 7] = [
        (EinkColor::Black, (0, 0, 0)),
        (EinkColor::White, (255, 255, 255)),
        (EinkColor::Green, (0, 255, 0)),
        (EinkColor::Blue, (0, 0, 255)),
        (EinkColor::Red, (255, 0, 0)),
        (EinkColor::Yellow, (255, 255, 0)),
        (EinkColor::Orange, (255, 128, 0)),
    ];

    if b > r.saturating_add(24) && b > g.saturating_add(16) {
        let whiteness = ((u16::from(r) + u16::from(g)) / 2) as u8;
        let threshold = bayer4(x, y).saturating_mul(16);
        return if whiteness > threshold {
            EinkColor::White
        } else {
            EinkColor::Blue
        };
    }

    let max_c = r.max(g).max(b);
    let min_c = r.min(g).min(b);
    let saturation = max_c.saturating_sub(min_c);
    if saturation < 28 {
        let luma = ((54u16 * r as u16) + (183u16 * g as u16) + (19u16 * b as u16)) >> 8;
        let threshold = (bayer4(x, y) as u16) * 16;
        return if luma > threshold {
            EinkColor::White
        } else {
            EinkColor::Black
        };
    }

    let mut best: Option<(EinkColor, i32)> = None;
    let mut second: Option<(EinkColor, i32)> = None;

    for (color, (pr, pg, pb)) in PALETTE {
        let dr = i32::from(r) - i32::from(pr);
        let dg = i32::from(g) - i32::from(pg);
        let db = i32::from(b) - i32::from(pb);
        let dist = dr * dr + dg * dg + db * db;

        match best {
            None => best = Some((color, dist)),
            Some((_, best_dist)) if dist < best_dist => {
                second = best;
                best = Some((color, dist));
            }
            _ => match second {
                None => second = Some((color, dist)),
                Some((_, second_dist)) if dist < second_dist => second = Some((color, dist)),
                _ => {}
            },
        }
    }

    let (best_color, best_dist) = best.unwrap_or((EinkColor::White, 0));
    let (second_color, second_dist) = second.unwrap_or((best_color, best_dist));

    if best_dist == 0 || second_dist == 0 {
        return best_color;
    }

    let best_weight = (second_dist as f32) / ((best_dist + second_dist) as f32);
    let threshold = (bayer4(x, y) as f32) / 15.0;
    if best_weight >= threshold {
        best_color
    } else {
        second_color
    }
}

fn bayer4(x: u32, y: u32) -> u8 {
    const BAYER_4X4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

    BAYER_4X4[(y as usize) & 3][(x as usize) & 3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bayer4_known_values() {
        // Verify against the first row of the 4×4 Bayer matrix: [0, 8, 2, 10]
        assert_eq!(bayer4(0, 0), 0);
        assert_eq!(bayer4(1, 0), 8);
        assert_eq!(bayer4(2, 0), 2);
        assert_eq!(bayer4(3, 0), 10);
        // Confirm the modular wrap
        assert_eq!(bayer4(4, 0), bayer4(0, 0));
        assert_eq!(bayer4(0, 4), bayer4(0, 0));
    }

    // At (x=0, y=0) bayer4 == 0, so the dithering threshold is always 0.
    // best_weight is always >= 0.0, so the nearest-palette color wins without
    // any stochastic mixing — giving fully deterministic results.
    #[test]
    fn pure_palette_colors_map_exactly() {
        assert_eq!(map_rgb_to_eink_color(0, 0, 0, 0, 0), EinkColor::Black);
        assert_eq!(map_rgb_to_eink_color(255, 255, 255, 0, 0), EinkColor::White);
        assert_eq!(map_rgb_to_eink_color(255, 0, 0, 0, 0), EinkColor::Red);
        assert_eq!(map_rgb_to_eink_color(0, 255, 0, 0, 0), EinkColor::Green);
        assert_eq!(map_rgb_to_eink_color(0, 0, 255, 0, 0), EinkColor::Blue);
        assert_eq!(map_rgb_to_eink_color(255, 255, 0, 0, 0), EinkColor::Yellow);
        assert_eq!(map_rgb_to_eink_color(255, 128, 0, 0, 0), EinkColor::Orange);
    }

    #[test]
    fn mid_gray_dithers_by_position() {
        // (128,128,128) is achromatic (saturation < 28), luma ≈ 128.
        // At (0,0): bayer4=0,  threshold=0×16=0   → luma(128) > 0   → White
        assert_eq!(map_rgb_to_eink_color(128, 128, 128, 0, 0), EinkColor::White);
        // At (0,1): bayer4=12, threshold=12×16=192 → luma(128) ≤ 192 → Black
        assert_eq!(map_rgb_to_eink_color(128, 128, 128, 0, 1), EinkColor::Black);
    }

    #[test]
    fn eink_color_to_rgb_roundtrip() {
        // Every exact palette RGB value should map back to its own EinkColor at (0,0).
        for color in [
            EinkColor::Black,
            EinkColor::White,
            EinkColor::Green,
            EinkColor::Blue,
            EinkColor::Red,
            EinkColor::Yellow,
            EinkColor::Orange,
        ] {
            let (r, g, b) = color.to_rgb();
            assert_eq!(
                map_rgb_to_eink_color(r, g, b, 0, 0),
                color,
                "roundtrip failed for {color:?}",
            );
        }
    }
}
