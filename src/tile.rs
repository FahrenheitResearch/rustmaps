//! Web Mercator tile math (EPSG:3857)

use std::f64::consts::PI;

/// Bounding box for a tile in lat/lon coordinates
#[derive(Debug, Clone, Copy)]
pub struct TileBounds {
    pub north: f64,
    pub south: f64,
    pub west: f64,
    pub east: f64,
}

/// Convert tile coordinates to lat/lon bounding box
pub fn tile_to_latlon(z: u32, x: u32, y: u32) -> TileBounds {
    let n = (1u64 << z) as f64;
    let west = (x as f64) / n * 360.0 - 180.0;
    let east = (x as f64 + 1.0) / n * 360.0 - 180.0;
    let north = tile_y_to_lat(y as f64, n);
    let south = tile_y_to_lat(y as f64 + 1.0, n);
    TileBounds { north, south, west, east }
}

fn tile_y_to_lat(y: f64, n: f64) -> f64 {
    let lat_rad = (PI * (1.0 - 2.0 * y / n)).sinh().atan();
    lat_rad * 180.0 / PI
}

/// Convert lat/lon to pixel coordinates within a specific tile
/// Returns (px, py) where 0..256 is within the tile
pub fn latlon_to_pixel(lat: f64, lon: f64, z: u32, tile_x: u32, tile_y: u32) -> (f64, f64) {
    let n = (1u64 << z) as f64;
    // World pixel coordinates
    let world_x = (lon + 180.0) / 360.0 * n;
    let lat_rad = lat * PI / 180.0;
    let world_y = (1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n;
    // Pixel within this tile
    let px = (world_x - tile_x as f64) * 256.0;
    let py = (world_y - tile_y as f64) * 256.0;
    (px, py)
}

/// Check if a lat/lon point is potentially visible in a tile (with margin)
pub fn point_in_tile(lat: f64, lon: f64, bounds: &TileBounds, margin_deg: f64) -> bool {
    lat >= bounds.south - margin_deg
        && lat <= bounds.north + margin_deg
        && lon >= bounds.west - margin_deg
        && lon <= bounds.east + margin_deg
}

/// Check if a line segment between two points might intersect the tile bounds
pub fn segment_intersects_tile(
    lat1: f64, lon1: f64,
    lat2: f64, lon2: f64,
    bounds: &TileBounds,
    margin_deg: f64,
) -> bool {
    let min_lat = lat1.min(lat2);
    let max_lat = lat1.max(lat2);
    let min_lon = lon1.min(lon2);
    let max_lon = lon1.max(lon2);
    !(max_lat < bounds.south - margin_deg
        || min_lat > bounds.north + margin_deg
        || max_lon < bounds.west - margin_deg
        || min_lon > bounds.east + margin_deg)
}
