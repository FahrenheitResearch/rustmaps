//! Tile renderer - rasterize Natural Earth geodata to 256x256 tiles.

pub mod rasterize;
pub mod style;

use crate::geo::GeoData;
use crate::tile;
use rasterize::*;
use style::*;

pub const TILE_SIZE: usize = 256;

pub struct TileRenderer {
    pub geo: GeoData,
}

/// RGBA pixel buffer for a tile
pub struct TilePixels {
    pub data: Vec<u8>,
}

impl TilePixels {
    pub fn new() -> Self {
        TilePixels {
            data: vec![0; TILE_SIZE * TILE_SIZE * 4],
        }
    }

    pub fn fill(&mut self, r: u8, g: u8, b: u8) {
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            self.data[i * 4] = r;
            self.data[i * 4 + 1] = g;
            self.data[i * 4 + 2] = b;
            self.data[i * 4 + 3] = 255;
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if x < 0 || y < 0 || x >= TILE_SIZE as i32 || y >= TILE_SIZE as i32 { return; }
        let idx = (y as usize * TILE_SIZE + x as usize) * 4;
        if a == 255 {
            self.data[idx] = r;
            self.data[idx + 1] = g;
            self.data[idx + 2] = b;
            self.data[idx + 3] = 255;
        } else if a > 0 {
            let alpha = a as f32 / 255.0;
            let inv = 1.0 - alpha;
            self.data[idx] = (r as f32 * alpha + self.data[idx] as f32 * inv) as u8;
            self.data[idx + 1] = (g as f32 * alpha + self.data[idx + 1] as f32 * inv) as u8;
            self.data[idx + 2] = (b as f32 * alpha + self.data[idx + 2] as f32 * inv) as u8;
            self.data[idx + 3] = 255;
        }
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, coverage: f32) {
        if coverage <= 0.0 { return; }
        let a = (coverage.min(1.0) * 255.0) as u8;
        self.set_pixel(x, y, r, g, b, a);
    }

    /// Get raw RGBA pixel data (for GPU texture upload)
    pub fn to_rgba(&self) -> &[u8] {
        &self.data
    }

    pub fn to_png_rgb(&self) -> Vec<u8> {
        let mut rgb = vec![0u8; TILE_SIZE * TILE_SIZE * 3];
        for i in 0..(TILE_SIZE * TILE_SIZE) {
            rgb[i * 3] = self.data[i * 4];
            rgb[i * 3 + 1] = self.data[i * 4 + 1];
            rgb[i * 3 + 2] = self.data[i * 4 + 2];
        }
        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, TILE_SIZE as u32, TILE_SIZE as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&rgb).unwrap();
        }
        buf
    }
}

impl TileRenderer {
    pub fn new(geo: GeoData) -> Self { TileRenderer { geo } }

    pub fn render_tile_png(&self, z: u8, x: u32, y: u32) -> Vec<u8> {
        self.render_tile(z, x, y).to_png_rgb()
    }

    pub fn render_tile(&self, z: u8, x: u32, y: u32) -> TilePixels {
        let mut px = TilePixels::new();
        let bounds = tile::tile_to_latlon(z, x, y);

        let oc = COLOR_OCEAN;
        px.fill(oc.r, oc.g, oc.b);

        let lc = COLOR_LAND;
        for poly in self.geo.land_for_zoom(z) {
            fill_polygon(&mut px, poly, z, x, y, &bounds, lc.r, lc.g, lc.b);
        }

        if show_lakes(z) {
            let c = COLOR_LAKE;
            for poly in self.geo.lakes_for_zoom(z) {
                fill_polygon(&mut px, poly, z, x, y, &bounds, c.r, c.g, c.b);
            }
        }

        if show_rivers(z) {
            let c = COLOR_RIVER;
            let w = river_width(z);
            for line in &self.geo.rivers {
                draw_polyline_aa(&mut px, line, z, x, y, &bounds, c.r, c.g, c.b, w);
            }
        }

        if show_state_borders(z) {
            let c = COLOR_STATE_BORDER;
            let w = border_width(z) * 0.7;
            for line in &self.geo.state_borders {
                draw_polyline_aa(&mut px, line, z, x, y, &bounds, c.r, c.g, c.b, w);
            }
        }

        {
            let c = COLOR_COUNTRY_BORDER;
            let w = border_width(z);
            for line in &self.geo.country_borders {
                draw_polyline_aa(&mut px, line, z, x, y, &bounds, c.r, c.g, c.b, w);
            }
        }

        {
            let c = COLOR_COASTLINE;
            let w = coastline_width(z);
            for line in self.geo.coastlines_for_zoom(z) {
                draw_polyline_aa(&mut px, line, z, x, y, &bounds, c.r, c.g, c.b, w);
            }
        }

        for city in &self.geo.cities {
            if !show_city_tier(city.tier, z) { continue; }
            if !point_in_bounds(city.lon, city.lat, &bounds, 0.5) { continue; }
            let (cpx, cpy) = tile::latlon_to_pixel(city.lat, city.lon, z, x, y);
            let radius = city_dot_radius(city.tier, z);
            let dc = COLOR_CITY_DOT;
            draw_filled_circle(&mut px, cpx as i32, cpy as i32, radius, dc.r, dc.g, dc.b);
            if show_city_labels(z) {
                let lc = if city.tier <= 1 { COLOR_MAJOR_CITY_LABEL } else { COLOR_CITY_LABEL };
                draw_text(&mut px, cpx as i32 + radius as i32 + 3, cpy as i32 - 3, &city.name, lc.r, lc.g, lc.b);
            }
        }

        px
    }
}

fn point_in_bounds(lon: f64, lat: f64, bounds: &tile::TileBounds, margin: f64) -> bool {
    lat >= bounds.south - margin && lat <= bounds.north + margin &&
    lon >= bounds.west - margin && lon <= bounds.east + margin
}
