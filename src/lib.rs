pub mod tile;
pub mod geo;
pub mod render;

use geo::GeoData;
use render::TileRenderer;
use std::path::Path;

/// Load geodata and create a renderer. Call render_tile() on the result.
pub fn load_renderer(data_dir: &Path) -> Result<TileRenderer, String> {
    let geo = GeoData::load(data_dir)?;
    Ok(TileRenderer::new(geo))
}
