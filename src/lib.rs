//! rustmaps - Pure Rust map tile renderer with zero dependencies.
//!
//! Renders 256x256 PNG map tiles at any zoom level (0-18) with
//! coastlines, country borders, US state borders, lakes, rivers, and cities.

pub mod geo;
pub mod render;
pub mod tile;
pub mod png;
pub mod deflate;

/// Render a map tile at the given zoom/x/y coordinates and return PNG bytes.
pub fn render_tile(z: u32, x: u32, y: u32) -> Vec<u8> {
    let buf = render::render_tile_pixels(z, x, y);
    png::encode_png(&buf.data)
}
