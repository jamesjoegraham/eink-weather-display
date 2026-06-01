// src/svg/render.rs
// SVG rendering logic extracted from svg_render.rs

use std::{error::Error, path::Path};

#[cfg(target_os = "linux")]
use embedded_graphics::{prelude::*, Pixel};

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

#[cfg(target_os = "linux")]
use epd_waveshare::{color::OctColor, epd7in3f::Display7in3f};
use tiny_skia::Pixmap;
use usvg::Transform;
#[cfg(target_os = "linux")]
use usvg::Tree;

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
    let options = svg_options_with_fonts();
    let tree = Tree::from_data(svg_data, &options)?;

    let mut pixmap = Pixmap::new(size.width, size.height)
        .ok_or_else(|| format!("invalid icon size {}x{}", size.width, size.height))?;

    let svg_size = tree.size().to_int_size();
    let sx = size.width as f32 / svg_size.width() as f32;
    let sy = size.height as f32 / svg_size.height() as f32;
    let scale = sx.min(sy);

    let rendered_w = svg_size.width() as f32 * scale;
    let rendered_h = svg_size.height() as f32 * scale;
    let tx = (size.width as f32 - rendered_w) / 2.0;
    let ty = (size.height as f32 - rendered_h) / 2.0;

    let transform = Transform::from_row(scale, 0.0, 0.0, scale, tx, ty);
    let _ = resvg::render(&tree, transform, &mut pixmap.as_mut());

    for y in 0..size.height {
        for x in 0..size.width {
            let px = pixmap
                .pixel(x, y)
                .ok_or_else(|| format!("missing raster pixel at {},{}", x, y))?;

            if px.alpha() < 10 {
                continue;
            }

            let color = map_rgb_to_oct_color(px.red(), px.green(), px.blue(), x, y);
            let point = Point::new(origin.x + x as i32, origin.y + y as i32);
            if point.x >= 0 && point.y >= 0 && point.x < 800 && point.y < 480 {
                Pixel(point, color).draw(display)?;
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn map_rgb_to_oct_color(r: u8, g: u8, b: u8, x: u32, y: u32) -> OctColor {
    let palette = [
        (OctColor::Black, (0u8, 0u8, 0u8)),
        (OctColor::White, (255u8, 255u8, 255u8)),
        (OctColor::Green, (0u8, 255u8, 0u8)),
        (OctColor::Blue, (0u8, 0u8, 255u8)),
        (OctColor::Red, (255u8, 0u8, 0u8)),
        (OctColor::Yellow, (255u8, 255u8, 0u8)),
        (OctColor::Orange, (255u8, 128u8, 0u8)),
    ];

    if b > r.saturating_add(24) && b > g.saturating_add(16) {
        let whiteness = ((u16::from(r) + u16::from(g)) / 2) as u8;
        let threshold = bayer4(x, y).saturating_mul(16);
        return if whiteness > threshold {
            OctColor::White
        } else {
            OctColor::Blue
        };
    }

    let max_c = r.max(g).max(b);
    let min_c = r.min(g).min(b);
    let saturation = max_c.saturating_sub(min_c);
    if saturation < 28 {
        let luma = ((54u16 * r as u16) + (183u16 * g as u16) + (19u16 * b as u16)) >> 8;
        let threshold = (bayer4(x, y) as u16) * 16;
        return if luma > threshold {
            OctColor::White
        } else {
            OctColor::Black
        };
    }

    let mut best: Option<(OctColor, i32)> = None;
    let mut second: Option<(OctColor, i32)> = None;

    for (color, (pr, pg, pb)) in palette {
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

    let (best_color, best_dist) = best.unwrap_or((OctColor::White, 0));
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

#[cfg(target_os = "linux")]
fn bayer4(x: u32, y: u32) -> u8 {
    const BAYER_4X4: [[u8; 4]; 4] = [
        [0, 8, 2, 10],
        [12, 4, 14, 6],
        [3, 11, 1, 9],
        [15, 7, 13, 5],
    ];

    BAYER_4X4[(y as usize) & 3][(x as usize) & 3]
}
