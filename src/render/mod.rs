//! Tile renderer - rasterize geographic vectors to 256x256 pixel tiles.

pub mod rasterize;
pub mod style;

use crate::geo::{coastline, borders, lakes, rivers, cities};
use crate::tile::{self, TileBounds};
use rasterize::PixelBuffer;
use style::*;

/// Render a single 256x256 tile and return the RGB pixel buffer.
pub fn render_tile_pixels(z: u32, x: u32, y: u32) -> PixelBuffer {
    let bounds = tile::tile_to_latlon(z, x, y);
    let mut buf = PixelBuffer::new(COLOR_OCEAN);

    // Draw land masses (simplified - fill coastline polygons)
    draw_land(&mut buf, z, x, y, &bounds);

    // Draw coastlines
    draw_polylines(&mut buf, z, x, y, &bounds, coastline::COASTLINE_SEGMENTS, COLOR_COASTLINE, coastline_width(z));

    // Draw country borders
    draw_polylines(&mut buf, z, x, y, &bounds, borders::COUNTRY_BORDERS, COLOR_COUNTRY_BORDER, border_width(z));

    // Draw state borders (zoom 4+)
    if show_state_borders(z) {
        draw_polylines(&mut buf, z, x, y, &bounds, borders::STATE_BORDERS, COLOR_STATE_BORDER, border_width(z));
    }

    // Draw lakes (zoom 4+)
    if show_lakes(z) {
        draw_polygons(&mut buf, z, x, y, &bounds, lakes::LAKE_POLYGONS, COLOR_LAKE);
    }

    // Draw rivers (zoom 6+)
    if show_rivers(z) {
        draw_polylines(&mut buf, z, x, y, &bounds, rivers::RIVER_SEGMENTS, COLOR_RIVER, river_width(z));
    }

    // Draw cities
    for city in cities::CITIES {
        if !show_city_tier(city.tier, z) { continue; }
        if !tile::point_in_tile(city.lat, city.lon, &bounds, 2.0) { continue; }
        let (px, py) = tile::latlon_to_pixel(city.lat, city.lon, z, x, y);
        let ipx = px as i32;
        let ipy = py as i32;
        let radius = city_dot_radius(city.tier, z);
        buf.fill_circle(ipx, ipy, radius, COLOR_CITY_DOT);
        if show_city_labels(z) {
            let label_color = if city.tier == 1 { COLOR_MAJOR_CITY_LABEL } else { COLOR_CITY_LABEL };
            buf.draw_text(ipx + radius as i32 + 2, ipy - 3, city.name, label_color);
        }
    }

    buf
}

fn draw_polylines(
    buf: &mut PixelBuffer, z: u32, tx: u32, ty: u32, bounds: &TileBounds,
    segments: &[&[(f64, f64)]], color: Color, width: u32,
) {
    let margin = margin_for_zoom(z);
    for seg in segments {
        for i in 0..seg.len().saturating_sub(1) {
            let (lat1, lon1) = seg[i];
            let (lat2, lon2) = seg[i + 1];
            if !tile::segment_intersects_tile(lat1, lon1, lat2, lon2, bounds, margin) {
                continue;
            }
            let (x1, y1) = tile::latlon_to_pixel(lat1, lon1, z, tx, ty);
            let (x2, y2) = tile::latlon_to_pixel(lat2, lon2, z, tx, ty);
            buf.draw_line(x1, y1, x2, y2, color, width);
        }
    }
}

fn draw_polygons(
    buf: &mut PixelBuffer, z: u32, tx: u32, ty: u32, bounds: &TileBounds,
    polygons: &[&[(f64, f64)]], color: Color,
) {
    let margin = margin_for_zoom(z);
    for poly in polygons {
        // Check if any point of the polygon is near the tile
        let mut dominated = false;
        for &(lat, lon) in *poly {
            if tile::point_in_tile(lat, lon, bounds, margin) {
                dominated = true;
                break;
            }
        }
        if !dominated { continue; }

        // Convert all points to pixel coordinates
        let pixels: Vec<(f64, f64)> = poly.iter()
            .map(|&(lat, lon)| tile::latlon_to_pixel(lat, lon, z, tx, ty))
            .collect();
        buf.fill_polygon(&pixels, color);

        // Also draw the outline
        for i in 0..pixels.len() {
            let j = (i + 1) % pixels.len();
            buf.draw_line(pixels[i].0, pixels[i].1, pixels[j].0, pixels[j].1, color, 1);
        }
    }
}

fn draw_land(buf: &mut PixelBuffer, z: u32, tx: u32, ty: u32, bounds: &TileBounds) {
    // For each coastline segment that forms a closed polygon, fill with land color
    let margin = margin_for_zoom(z);
    for poly in coastline::LAND_POLYGONS {
        let mut dominated = false;
        for &(lat, lon) in *poly {
            if tile::point_in_tile(lat, lon, bounds, margin) {
                dominated = true;
                break;
            }
        }
        if !dominated { continue; }
        let pixels: Vec<(f64, f64)> = poly.iter()
            .map(|&(lat, lon)| tile::latlon_to_pixel(lat, lon, z, tx, ty))
            .collect();
        buf.fill_polygon(&pixels, COLOR_LAND);
    }
}

fn margin_for_zoom(z: u32) -> f64 {
    // Larger margin at low zoom to catch features that span large areas
    match z {
        0..=3 => 30.0,
        4..=6 => 10.0,
        7..=9 => 5.0,
        _ => 2.0,
    }
}
