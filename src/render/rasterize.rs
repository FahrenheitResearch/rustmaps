//! Rasterization: anti-aliased lines, polygon fill, circles, text.

use super::TilePixels;
use super::TILE_SIZE;
use crate::tile::{self, TileBounds};

/// Draw an anti-aliased polyline with per-segment bounds clipping
pub fn draw_polyline_aa(
    pixels: &mut TilePixels,
    points: &[(f64, f64)], // (lon, lat)
    z: u8, tx: u32, ty: u32,
    bounds: &TileBounds,
    r: u8, g: u8, b: u8,
    width: f32,
) {
    let margin_lat = (bounds.north - bounds.south) * 0.15;
    let margin_lon = (bounds.east - bounds.west) * 0.15;

    for i in 0..points.len().saturating_sub(1) {
        let (lon0, lat0) = points[i];
        let (lon1, lat1) = points[i + 1];

        let min_lat = lat0.min(lat1);
        let max_lat = lat0.max(lat1);
        let min_lon = lon0.min(lon1);
        let max_lon = lon0.max(lon1);

        if max_lat < bounds.south - margin_lat || min_lat > bounds.north + margin_lat {
            continue;
        }
        if max_lon < bounds.west - margin_lon || min_lon > bounds.east + margin_lon {
            continue;
        }

        let (px0, py0) = tile::latlon_to_pixel(lat0, lon0, z, tx, ty);
        let (px1, py1) = tile::latlon_to_pixel(lat1, lon1, z, tx, ty);

        draw_line_aa(pixels, px0 as f32, py0 as f32, px1 as f32, py1 as f32, r, g, b, width);
    }
}

/// Fill a geographic polygon (lon/lat points) on the tile
pub fn fill_polygon(
    pixels: &mut TilePixels,
    ring: &[(f64, f64)], // (lon, lat)
    z: u8, tx: u32, ty: u32,
    bounds: &TileBounds,
    r: u8, g: u8, b: u8,
) {
    if ring.len() < 3 { return; }

    // Bounding box check
    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    for &(lon, lat) in ring {
        min_lon = min_lon.min(lon);
        max_lon = max_lon.max(lon);
        min_lat = min_lat.min(lat);
        max_lat = max_lat.max(lat);
    }
    if max_lat < bounds.south || min_lat > bounds.north ||
       max_lon < bounds.west || min_lon > bounds.east {
        return;
    }

    // Project to pixel coords
    let pixel_points: Vec<(f32, f32)> = ring.iter()
        .map(|&(lon, lat)| {
            let (px, py) = tile::latlon_to_pixel(lat, lon, z, tx, ty);
            (px as f32, py as f32)
        })
        .collect();

    fill_polygon_pixels(pixels, &pixel_points, r, g, b);
}

/// Scanline fill of a polygon already in pixel coordinates
fn fill_polygon_pixels(pixels: &mut TilePixels, points: &[(f32, f32)], r: u8, g: u8, b: u8) {
    if points.len() < 3 { return; }
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for &(_, y) in points {
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let min_y = (min_y as i32).max(0);
    let max_y = (max_y as i32).min(TILE_SIZE as i32 - 1);
    let n = points.len();
    let mut intersections: Vec<f32> = Vec::with_capacity(32);

    for y in min_y..=max_y {
        let yf = y as f32 + 0.5;
        intersections.clear();
        for i in 0..n {
            let j = (i + 1) % n;
            let y0 = points[i].1;
            let y1 = points[j].1;
            if (y0 <= yf && y1 > yf) || (y1 <= yf && y0 > yf) {
                let x0 = points[i].0;
                let x1 = points[j].0;
                let t = (yf - y0) / (y1 - y0);
                intersections.push(x0 + t * (x1 - x0));
            }
        }
        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut i = 0;
        while i + 1 < intersections.len() {
            let xs = (intersections[i].ceil() as i32).max(0);
            let xe = (intersections[i + 1].floor() as i32).min(TILE_SIZE as i32 - 1);
            for x in xs..=xe {
                pixels.set_pixel(x, y, r, g, b, 255);
            }
            i += 2;
        }
    }
}

/// Anti-aliased line with variable width
fn draw_line_aa(
    pixels: &mut TilePixels,
    x0: f32, y0: f32, x1: f32, y1: f32,
    r: u8, g: u8, b: u8,
    width: f32,
) {
    if width <= 1.2 {
        wu_line(pixels, x0, y0, x1, y1, r, g, b, 1.0);
    } else {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 { return; }
        let nx = -dy / len;
        let ny = dx / len;
        let half = width / 2.0;
        let steps = (width * 1.5).ceil() as i32;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let off = -half + t * width;
            let coverage = if off.abs() > half - 0.5 {
                (half - off.abs() + 0.5).max(0.0).min(1.0)
            } else {
                1.0
            };
            wu_line(
                pixels,
                x0 + nx * off, y0 + ny * off,
                x1 + nx * off, y1 + ny * off,
                r, g, b, coverage,
            );
        }
    }
}

/// Wu's anti-aliased line algorithm
fn wu_line(
    pixels: &mut TilePixels,
    mut x0: f32, mut y0: f32, mut x1: f32, mut y1: f32,
    r: u8, g: u8, b: u8,
    brightness: f32,
) {
    let steep = (y1 - y0).abs() > (x1 - x0).abs();
    if steep {
        std::mem::swap(&mut x0, &mut y0);
        std::mem::swap(&mut x1, &mut y1);
    }
    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
    }

    let dx = x1 - x0;
    let dy = y1 - y0;
    let gradient = if dx.abs() < 0.001 { 1.0 } else { dy / dx };

    // First endpoint
    let xend = x0.round();
    let yend = y0 + gradient * (xend - x0);
    let xgap = rfpart(x0 + 0.5) * brightness;
    let xpxl1 = xend as i32;
    let ypxl1 = yend.floor() as i32;
    if steep {
        pixels.blend_pixel(ypxl1, xpxl1, r, g, b, rfpart(yend) * xgap);
        pixels.blend_pixel(ypxl1 + 1, xpxl1, r, g, b, fpart(yend) * xgap);
    } else {
        pixels.blend_pixel(xpxl1, ypxl1, r, g, b, rfpart(yend) * xgap);
        pixels.blend_pixel(xpxl1, ypxl1 + 1, r, g, b, fpart(yend) * xgap);
    }
    let mut intery = yend + gradient;

    // Second endpoint
    let xend2 = x1.round();
    let yend2 = y1 + gradient * (xend2 - x1);
    let xgap2 = fpart(x1 + 0.5) * brightness;
    let xpxl2 = xend2 as i32;
    let ypxl2 = yend2.floor() as i32;
    if steep {
        pixels.blend_pixel(ypxl2, xpxl2, r, g, b, rfpart(yend2) * xgap2);
        pixels.blend_pixel(ypxl2 + 1, xpxl2, r, g, b, fpart(yend2) * xgap2);
    } else {
        pixels.blend_pixel(xpxl2, ypxl2, r, g, b, rfpart(yend2) * xgap2);
        pixels.blend_pixel(xpxl2, ypxl2 + 1, r, g, b, fpart(yend2) * xgap2);
    }

    // Main loop
    for x in (xpxl1 + 1)..xpxl2 {
        let y = intery.floor() as i32;
        let frac = fpart(intery);
        if steep {
            pixels.blend_pixel(y, x, r, g, b, (1.0 - frac) * brightness);
            pixels.blend_pixel(y + 1, x, r, g, b, frac * brightness);
        } else {
            pixels.blend_pixel(x, y, r, g, b, (1.0 - frac) * brightness);
            pixels.blend_pixel(x, y + 1, r, g, b, frac * brightness);
        }
        intery += gradient;
    }
}

/// Anti-aliased filled circle
pub fn draw_filled_circle(pixels: &mut TilePixels, cx: i32, cy: i32, radius: f32, r: u8, g: u8, b: u8) {
    let ri = radius.ceil() as i32 + 1;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            if dist <= radius + 0.5 {
                let alpha = if dist > radius - 0.5 {
                    (radius + 0.5 - dist).max(0.0).min(1.0)
                } else {
                    1.0
                };
                pixels.blend_pixel(cx + dx, cy + dy, r, g, b, alpha);
            }
        }
    }
}

/// Draw text with 5x7 bitmap font and dark halo for readability
pub fn draw_text(pixels: &mut TilePixels, x: i32, y: i32, text: &str, r: u8, g: u8, b: u8) {
    // Draw dark halo first (1px offset in all directions)
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 { continue; }
            let mut cx = x + dx;
            for ch in text.chars() {
                if let Some(glyph) = get_glyph(ch) {
                    for row in 0..7i32 {
                        let bits = glyph[row as usize];
                        for col in 0..5i32 {
                            if bits & (1 << (4 - col)) != 0 {
                                pixels.blend_pixel(cx + col, y + row + dy, 0, 0, 0, 0.5);
                            }
                        }
                    }
                }
                cx += 6;
            }
        }
    }
    // Draw text on top
    let mut cx = x;
    for ch in text.chars() {
        if let Some(glyph) = get_glyph(ch) {
            for row in 0..7i32 {
                let bits = glyph[row as usize];
                for col in 0..5i32 {
                    if bits & (1 << (4 - col)) != 0 {
                        pixels.blend_pixel(cx + col, y + row, r, g, b, 0.95);
                    }
                }
            }
        }
        cx += 6;
    }
}

fn fpart(x: f32) -> f32 { x - x.floor() }
fn rfpart(x: f32) -> f32 { 1.0 - fpart(x) }

fn get_glyph(ch: char) -> Option<[u8; 7]> {
    let ch = ch.to_ascii_uppercase();
    match ch {
        'A' => Some([0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'B' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
        'C' => Some([0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
        'D' => Some([0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100]),
        'E' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        'F' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
        'G' => Some([0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110]),
        'H' => Some([0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'I' => Some([0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        'J' => Some([0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
        'K' => Some([0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
        'L' => Some([0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        'M' => Some([0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
        'N' => Some([0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
        'O' => Some([0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'P' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
        'Q' => Some([0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
        'R' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        'S' => Some([0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110]),
        'T' => Some([0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        'U' => Some([0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'V' => Some([0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
        'W' => Some([0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
        'X' => Some([0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
        'Y' => Some([0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
        'Z' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
        '0' => Some([0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
        '1' => Some([0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        '2' => Some([0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111]),
        '3' => Some([0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110]),
        '4' => Some([0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
        '5' => Some([0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
        '6' => Some([0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        '7' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
        '8' => Some([0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
        '9' => Some([0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
        ' ' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000]),
        '.' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100]),
        ',' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100, 0b01000]),
        '-' => Some([0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000]),
        '\'' => Some([0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000]),
        _ => None,
    }
}
