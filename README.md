# rustmaps

Fast offline map tile renderer with a dark theme, designed for weather and radar application backgrounds. Renders [Natural Earth](https://www.naturalearthdata.com/) vector geodata into 256x256 PNG tiles using the Web Mercator (EPSG:3857) projection.

![Dark theme map tiles](https://img.shields.io/badge/theme-dark-0d1117) ![Rust](https://img.shields.io/badge/rust-stable-orange)

## Features

- Dark-theme cartography: coastlines, land, lakes, rivers, country/state borders, city labels
- Multi-resolution: 110m/50m/10m Natural Earth datasets selected by zoom level
- Anti-aliased line rendering with variable-width strokes
- City labels with halo text for readability
- Interactive GUI with pan, zoom, and tile export
- CLI for batch tile generation
- Used as a library by [NexView](https://github.com/FahrenheitResearch/nexview) and hrrr-render

## Getting Started

### 1. Install Rust

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Download Natural Earth Shapefiles

Download the following datasets from [naturalearthdata.com](https://www.naturalearthdata.com/downloads/) and place all `.shp`, `.shx`, `.dbf`, `.prj`, and `.cpg` files into a `data/` folder next to the executable (or in the repo root for development):

| Dataset | Resolution | Download |
|---------|-----------|----------|
| Coastline | 110m, 50m, 10m | [110m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/110m/physical/ne_110m_coastline.zip) / [50m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/50m/physical/ne_50m_coastline.zip) / [10m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/10m/physical/ne_10m_coastline.zip) |
| Land | 50m, 10m | [50m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/50m/physical/ne_50m_land.zip) / [10m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/10m/physical/ne_10m_land.zip) |
| Lakes | 50m, 10m | [50m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/50m/physical/ne_50m_lakes.zip) / [10m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/10m/physical/ne_10m_lakes.zip) |
| Rivers | 10m | [10m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/10m/physical/ne_10m_rivers_lake_centerlines.zip) |
| Country borders | 10m | [10m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/10m/cultural/ne_10m_admin_0_boundary_lines_land.zip) |
| State/province borders | 10m | [10m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/10m/cultural/ne_10m_admin_1_states_provinces_lines.zip) |
| Populated places | 10m | [10m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/10m/cultural/ne_10m_populated_places_simple.zip) |
| Urban areas | 50m | [50m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/50m/cultural/ne_50m_urban_areas.zip) |
| Ocean | 10m | [10m](https://www.naturalearthdata.com/http//www.naturalearthdata.com/download/10m/physical/ne_10m_ocean.zip) |

Required shapefiles (the GUI will show an error listing missing files):

```
ne_110m_coastline.shp
ne_50m_coastline.shp
ne_10m_coastline.shp
ne_50m_land.shp
ne_10m_land.shp
ne_10m_admin_0_boundary_lines_land.shp
ne_10m_admin_1_states_provinces_lines.shp
ne_50m_lakes.shp
ne_10m_lakes.shp
ne_10m_rivers_lake_centerlines.shp
ne_10m_populated_places_simple.shp
```

### 3. Build and Run

**GUI (interactive map viewer):**
```bash
cargo run --release
```

**CLI (batch tile rendering):**
```bash
cargo run --release --bin rustmaps-cli -- --data ./data --output ./output
```

## Data Directory

rustmaps looks for Natural Earth shapefiles in the following locations (in order):

1. `RUSTMAPS_DATA` environment variable (if set and the path exists)
2. `data/` folder next to the executable
3. `data/` folder in the current working directory

Set the environment variable if your data is in a custom location:

```bash
# Linux/macOS
export RUSTMAPS_DATA=/path/to/shapefiles

# Windows
set RUSTMAPS_DATA=C:\path\to\shapefiles
```

## GUI Controls

| Action | Control |
|--------|---------|
| Pan | Click and drag |
| Zoom in/out | Scroll wheel or +/- buttons |
| Export tiles | Click "Export Tiles" in the status bar |

The status bar shows the current zoom level, cursor coordinates (lat/lon), average render time per tile, and number of cached tiles.

## CLI Options

```
rustmaps-cli [OPTIONS]

  --data <PATH>     Path to Natural Earth shapefile directory (default: ./data)
  --output <PATH>   Output directory for rendered tiles (default: ./output)
  --serve           Start a tile server instead of batch rendering
  --port <PORT>     Tile server port (default: 8080, requires --serve)
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
rustmaps = { git = "https://github.com/FahrenheitResearch/rustmaps.git" }
```

```rust
use std::path::Path;

fn main() {
    let renderer = rustmaps::load_renderer(Path::new("data"))
        .expect("Failed to load geodata");

    // Render a single tile (z/x/y)
    let pixels = renderer.render_tile(5, 8, 12);
    let rgba = pixels.to_rgba(); // [u8; 256*256*4]

    // Or render directly to PNG bytes
    let png_bytes = renderer.render_tile_png(5, 8, 12);
    std::fs::write("tile.png", &png_bytes).unwrap();
}
```

## Downloads

Pre-built binaries for Windows, macOS (Intel + Apple Silicon), and Linux are available on the [Releases](https://github.com/FahrenheitResearch/rustmaps/releases) page.

**macOS users:** If Gatekeeper shows a "damaged app" warning, run:
```bash
xattr -cr /Applications/rustmaps.app
```

## License

MIT
