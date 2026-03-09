//! rustmaps GUI - interactive dark-theme map viewer with pan/zoom.

use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use rustmaps::render::TileRenderer;

const TILE_SIZE: f32 = 256.0;
const MIN_ZOOM: u8 = 2;
const MAX_ZOOM: u8 = 10;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("rustmaps")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "rustmaps",
        native_options,
        Box::new(|cc| Ok(Box::new(MapApp::new(cc)))),
    )
}

struct TileCache {
    textures: HashMap<(u8, u32, u32), egui::TextureHandle>,
    render_times: Vec<f64>, // ms per tile (last N)
}

impl TileCache {
    fn new() -> Self {
        Self {
            textures: HashMap::new(),
            render_times: Vec::new(),
        }
    }

    fn get_or_render(
        &mut self,
        ctx: &egui::Context,
        renderer: &TileRenderer,
        z: u8,
        x: u32,
        y: u32,
    ) -> &egui::TextureHandle {
        self.textures.entry((z, x, y)).or_insert_with(|| {
            let t0 = Instant::now();
            let tile = renderer.render_tile(z, x, y);
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            self.render_times.push(ms);
            // Keep only last 500 timings
            if self.render_times.len() > 500 {
                self.render_times.drain(0..self.render_times.len() - 500);
            }

            let rgba = tile.to_rgba();
            let image = egui::ColorImage::from_rgba_unmultiplied([256, 256], rgba);
            ctx.load_texture(
                format!("tile_{}_{}_{}", z, x, y),
                image,
                egui::TextureOptions {
                    magnification: egui::TextureFilter::Nearest,
                    minification: egui::TextureFilter::Linear,
                    ..Default::default()
                },
            )
        })
    }

    fn cached_count(&self) -> usize {
        self.textures.len()
    }

    fn avg_render_ms(&self) -> f64 {
        if self.render_times.is_empty() {
            return 0.0;
        }
        self.render_times.iter().sum::<f64>() / self.render_times.len() as f64
    }

    fn clear(&mut self) {
        self.textures.clear();
        self.render_times.clear();
    }
}

struct MapApp {
    renderer: Option<Arc<TileRenderer>>,
    load_error: Option<String>,
    cache: TileCache,
    // Map state: center position in world pixel coords at zoom level
    center_lat: f64,
    center_lon: f64,
    zoom: u8,
    // Export state
    show_export: bool,
    export_min_zoom: u8,
    export_max_zoom: u8,
    export_north: String,
    export_south: String,
    export_east: String,
    export_west: String,
    export_status: Option<String>,
    // Data directory
    data_dir: PathBuf,
}

impl MapApp {
    fn new(_cc: &eframe::CreationContext) -> Self {
        // Try to find data directory
        let data_dir = find_data_dir();
        let (renderer, load_error) = match &data_dir {
            Some(dir) => {
                match rustmaps::load_renderer(dir) {
                    Ok(r) => (Some(Arc::new(r)), None),
                    Err(e) => (None, Some(e)),
                }
            }
            None => (None, Some("Natural Earth shapefiles not found.\n\nDownload from https://www.naturalearthdata.com/ and place .shp files in a 'data' folder next to the executable, or set RUSTMAPS_DATA environment variable.".to_string())),
        };

        Self {
            renderer,
            load_error,
            cache: TileCache::new(),
            center_lat: 39.0,
            center_lon: -96.0,
            zoom: 5,
            show_export: false,
            export_min_zoom: 3,
            export_max_zoom: 7,
            export_north: "50.0".to_string(),
            export_south: "24.0".to_string(),
            export_east: "-66.0".to_string(),
            export_west: "-125.0".to_string(),
            export_status: None,
            data_dir: data_dir.unwrap_or_else(|| PathBuf::from("data")),
        }
    }

    fn pixel_to_latlon(&self, screen_pos: egui::Pos2, rect: egui::Rect) -> (f64, f64) {
        let n = (1u64 << self.zoom) as f64;
        let center_world_x = (self.center_lon + 180.0) / 360.0 * n * 256.0;
        let lat_rad = self.center_lat.to_radians();
        let center_world_y =
            (1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n * 256.0;

        let dx = (screen_pos.x - rect.center().x) as f64;
        let dy = (screen_pos.y - rect.center().y) as f64;

        let world_x = center_world_x + dx;
        let world_y = center_world_y + dy;

        let lon = world_x / (n * 256.0) * 360.0 - 180.0;
        let lat_rad =
            (std::f64::consts::PI * (1.0 - 2.0 * world_y / (n * 256.0))).sinh().atan();
        let lat = lat_rad.to_degrees();

        (lat, lon)
    }
}

fn find_data_dir() -> Option<PathBuf> {
    // Check env var first
    if let Ok(dir) = std::env::var("RUSTMAPS_DATA") {
        let p = PathBuf::from(dir);
        if p.exists() {
            return Some(p);
        }
    }

    // Check next to executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let p = parent.join("data");
            if p.exists() {
                return Some(p);
            }
        }
    }

    // Check current directory
    let p = PathBuf::from("data");
    if p.exists() {
        return Some(p);
    }

    // Check common dev location
    let p = PathBuf::from(r"C:\Users\drew\rustmaps\data");
    if p.exists() {
        return Some(p);
    }

    None
}

impl eframe::App for MapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Dark theme
        ctx.set_visuals(egui::Visuals::dark());

        // Handle missing data
        if self.renderer.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("rustmaps");
                    ui.add_space(20.0);
                    if let Some(err) = &self.load_error {
                        ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
                    }
                    ui.add_space(20.0);
                    ui.label("Expected shapefile location:");
                    ui.monospace(self.data_dir.display().to_string());
                    ui.add_space(10.0);
                    ui.label("Required files:");
                    ui.monospace("ne_110m_coastline.shp\nne_50m_coastline.shp\nne_10m_coastline.shp\nne_50m_land.shp\nne_10m_land.shp\nne_10m_admin_0_boundary_lines_land.shp\nne_10m_admin_1_states_provinces_lines.shp\nne_50m_lakes.shp\nne_10m_lakes.shp\nne_10m_rivers_lake_centerlines.shp\nne_10m_populated_places_simple.shp");
                });
            });
            return;
        }

        let renderer = self.renderer.as_ref().unwrap().clone();

        // Main map panel
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(13, 17, 23)))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                // Handle zoom with scroll
                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 && rect.contains(ui.input(|i| {
                    i.pointer.hover_pos().unwrap_or(egui::Pos2::ZERO)
                })) {
                    // Get position under cursor before zoom
                    let cursor_pos = ui.input(|i| i.pointer.hover_pos().unwrap_or(rect.center()));
                    let (lat_before, lon_before) = self.pixel_to_latlon(cursor_pos, rect);

                    if scroll > 0.0 && self.zoom < MAX_ZOOM {
                        self.zoom += 1;
                        self.cache.clear(); // Clear cache on zoom change
                    } else if scroll < 0.0 && self.zoom > MIN_ZOOM {
                        self.zoom -= 1;
                        self.cache.clear();
                    }

                    // Adjust center so the point under cursor stays put
                    let (lat_after, lon_after) = self.pixel_to_latlon(cursor_pos, rect);
                    self.center_lat += lat_before - lat_after;
                    self.center_lon += lon_before - lon_after;
                }

                // Handle panning with drag
                if response.dragged() {
                    let delta = response.drag_delta();
                    let n = (1u64 << self.zoom) as f64;
                    let total_pixels = n * 256.0;

                    // Convert pixel delta to lat/lon delta
                    let dlon = -(delta.x as f64) / total_pixels * 360.0;

                    let lat_rad = self.center_lat.to_radians();
                    let center_y =
                        (1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * total_pixels;
                    let new_y = center_y + delta.y as f64;
                    let new_lat_rad = (std::f64::consts::PI * (1.0 - 2.0 * new_y / total_pixels))
                        .sinh()
                        .atan();
                    let new_lat = new_lat_rad.to_degrees();

                    self.center_lat = new_lat.clamp(-85.0, 85.0);
                    self.center_lon += dlon;
                    // Wrap longitude
                    while self.center_lon > 180.0 {
                        self.center_lon -= 360.0;
                    }
                    while self.center_lon < -180.0 {
                        self.center_lon += 360.0;
                    }
                }

                // Calculate visible tiles
                let n = (1u64 << self.zoom) as f64;
                let center_world_x = (self.center_lon + 180.0) / 360.0 * n * 256.0;
                let lat_rad = self.center_lat.to_radians();
                let center_world_y = (1.0 - lat_rad.tan().asinh() / std::f64::consts::PI)
                    / 2.0
                    * n
                    * 256.0;

                let half_w = rect.width() as f64 / 2.0;
                let half_h = rect.height() as f64 / 2.0;

                let min_world_x = center_world_x - half_w;
                let max_world_x = center_world_x + half_w;
                let min_world_y = center_world_y - half_h;
                let max_world_y = center_world_y + half_h;

                let tile_x_min = (min_world_x / 256.0).floor() as i64;
                let tile_x_max = (max_world_x / 256.0).floor() as i64;
                let tile_y_min = ((min_world_y / 256.0).floor() as i64).max(0);
                let tile_y_max = ((max_world_y / 256.0).floor() as i64).min(n as i64 - 1);

                let painter = ui.painter_at(rect);

                // Draw tiles
                for ty in tile_y_min..=tile_y_max {
                    for tx in tile_x_min..=tile_x_max {
                        // Handle longitude wrapping
                        let actual_tx = ((tx % n as i64) + n as i64) % n as i64;
                        if actual_tx < 0 || actual_tx >= n as i64 || ty < 0 || ty >= n as i64 {
                            continue;
                        }

                        let screen_x = rect.left()
                            + (tx as f64 * 256.0 - min_world_x) as f32;
                        let screen_y = rect.top()
                            + (ty as f64 * 256.0 - min_world_y) as f32;

                        let tile_rect = egui::Rect::from_min_size(
                            egui::pos2(screen_x, screen_y),
                            egui::vec2(TILE_SIZE, TILE_SIZE),
                        );

                        // Only render if visible
                        if !tile_rect.intersects(rect) {
                            continue;
                        }

                        let texture = self.cache.get_or_render(
                            ctx,
                            &renderer,
                            self.zoom,
                            actual_tx as u32,
                            ty as u32,
                        );

                        painter.image(
                            texture.id(),
                            tile_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }

                // Overlay: stats bar at bottom
                let bar_height = 28.0;
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left(), rect.bottom() - bar_height),
                    egui::vec2(rect.width(), bar_height),
                );
                painter.rect_filled(bar_rect, 0.0, egui::Color32::from_rgba_premultiplied(0, 0, 0, 180));

                // Cursor lat/lon
                let cursor_text = if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    if rect.contains(pos) {
                        let (lat, lon) = self.pixel_to_latlon(pos, rect);
                        format!(
                            "  z{}  |  {:.4}, {:.4}  |  {:.1} ms/tile  |  {} cached",
                            self.zoom,
                            lat,
                            lon,
                            self.cache.avg_render_ms(),
                            self.cache.cached_count()
                        )
                    } else {
                        format!(
                            "  z{}  |  {:.1} ms/tile  |  {} cached",
                            self.zoom,
                            self.cache.avg_render_ms(),
                            self.cache.cached_count()
                        )
                    }
                } else {
                    format!(
                        "  z{}  |  {:.1} ms/tile  |  {} cached",
                        self.zoom,
                        self.cache.avg_render_ms(),
                        self.cache.cached_count()
                    )
                };

                painter.text(
                    egui::pos2(bar_rect.left() + 8.0, bar_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &cursor_text,
                    egui::FontId::monospace(13.0),
                    egui::Color32::from_rgb(200, 210, 220),
                );

                // Export button in the bar
                let btn_rect = egui::Rect::from_min_size(
                    egui::pos2(bar_rect.right() - 110.0, bar_rect.top() + 3.0),
                    egui::vec2(100.0, bar_height - 6.0),
                );
                let btn_response = ui.allocate_rect(btn_rect, egui::Sense::click());
                let btn_color = if btn_response.hovered() {
                    egui::Color32::from_rgb(60, 80, 120)
                } else {
                    egui::Color32::from_rgb(40, 55, 85)
                };
                painter.rect_filled(btn_rect, 4.0, btn_color);
                painter.text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Export Tiles",
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgb(200, 210, 220),
                );
                if btn_response.clicked() {
                    self.show_export = !self.show_export;
                }

                // Zoom controls top-right
                let zoom_y = rect.top() + 10.0;
                let zoom_x = rect.right() - 42.0;

                let zoom_in_rect = egui::Rect::from_min_size(
                    egui::pos2(zoom_x, zoom_y),
                    egui::vec2(32.0, 32.0),
                );
                let zoom_out_rect = egui::Rect::from_min_size(
                    egui::pos2(zoom_x, zoom_y + 36.0),
                    egui::vec2(32.0, 32.0),
                );

                let zi_resp = ui.allocate_rect(zoom_in_rect, egui::Sense::click());
                let zo_resp = ui.allocate_rect(zoom_out_rect, egui::Sense::click());

                let zi_color = if zi_resp.hovered() {
                    egui::Color32::from_rgb(60, 70, 80)
                } else {
                    egui::Color32::from_rgba_premultiplied(30, 35, 45, 220)
                };
                let zo_color = if zo_resp.hovered() {
                    egui::Color32::from_rgb(60, 70, 80)
                } else {
                    egui::Color32::from_rgba_premultiplied(30, 35, 45, 220)
                };

                painter.rect_filled(zoom_in_rect, 4.0, zi_color);
                painter.rect_filled(zoom_out_rect, 4.0, zo_color);
                painter.text(zoom_in_rect.center(), egui::Align2::CENTER_CENTER, "+", egui::FontId::monospace(18.0), egui::Color32::WHITE);
                painter.text(zoom_out_rect.center(), egui::Align2::CENTER_CENTER, "-", egui::FontId::monospace(18.0), egui::Color32::WHITE);

                if zi_resp.clicked() && self.zoom < MAX_ZOOM {
                    self.zoom += 1;
                    self.cache.clear();
                }
                if zo_resp.clicked() && self.zoom > MIN_ZOOM {
                    self.zoom -= 1;
                    self.cache.clear();
                }
            });

        // Export window
        if self.show_export {
            let mut show = self.show_export;
            egui::Window::new("Export Tiles")
                .open(&mut show)
                .resizable(false)
                .default_width(300.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Zoom range:");
                        ui.add(egui::DragValue::new(&mut self.export_min_zoom).range(MIN_ZOOM..=MAX_ZOOM).prefix("min: "));
                        ui.add(egui::DragValue::new(&mut self.export_max_zoom).range(MIN_ZOOM..=MAX_ZOOM).prefix("max: "));
                    });

                    ui.separator();
                    ui.label("Bounding box (lat/lon):");
                    egui::Grid::new("bbox_grid").show(ui, |ui| {
                        ui.label("North:");
                        ui.text_edit_singleline(&mut self.export_north);
                        ui.end_row();
                        ui.label("South:");
                        ui.text_edit_singleline(&mut self.export_south);
                        ui.end_row();
                        ui.label("West:");
                        ui.text_edit_singleline(&mut self.export_west);
                        ui.end_row();
                        ui.label("East:");
                        ui.text_edit_singleline(&mut self.export_east);
                        ui.end_row();
                    });

                    ui.separator();

                    // Count tiles
                    if let (Ok(n), Ok(s), Ok(w), Ok(e)) = (
                        self.export_north.parse::<f64>(),
                        self.export_south.parse::<f64>(),
                        self.export_west.parse::<f64>(),
                        self.export_east.parse::<f64>(),
                    ) {
                        let mut total = 0u64;
                        for z in self.export_min_zoom..=self.export_max_zoom {
                            let tiles = count_tiles_in_bbox(n, s, w, e, z);
                            total += tiles as u64;
                        }
                        ui.label(format!("Total tiles: {}", total));
                    }

                    ui.separator();

                    if ui.button("Choose folder and export...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new()
                            .set_title("Choose export folder")
                            .pick_folder()
                        {
                            if let (Ok(n), Ok(s), Ok(w), Ok(e)) = (
                                self.export_north.parse::<f64>(),
                                self.export_south.parse::<f64>(),
                                self.export_west.parse::<f64>(),
                                self.export_east.parse::<f64>(),
                            ) {
                                let renderer = self.renderer.as_ref().unwrap().clone();
                                let min_z = self.export_min_zoom;
                                let max_z = self.export_max_zoom;
                                let t0 = Instant::now();
                                let mut count = 0u32;

                                for z in min_z..=max_z {
                                    let tiles = tiles_in_bbox(n, s, w, e, z);
                                    for (x, y) in &tiles {
                                        let png = renderer.render_tile_png(z, *x, *y);
                                        let dir = folder.join(format!("{}/{}", z, x));
                                        std::fs::create_dir_all(&dir).ok();
                                        let path = dir.join(format!("{}.png", y));
                                        std::fs::write(&path, &png).ok();
                                        count += 1;
                                    }
                                }

                                let elapsed = t0.elapsed();
                                self.export_status = Some(format!(
                                    "Exported {} tiles in {:.1}s ({:.1} ms/tile)",
                                    count,
                                    elapsed.as_secs_f64(),
                                    elapsed.as_secs_f64() * 1000.0 / count.max(1) as f64
                                ));
                            }
                        }
                    }

                    if let Some(status) = &self.export_status {
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 100), status);
                    }
                });
            self.show_export = show;
        }
    }
}

fn latlon_to_tile(lat: f64, lon: f64, z: u8) -> (u32, u32) {
    let n = (1u32 << z) as f64;
    let x = ((lon + 180.0) / 360.0 * n).floor() as u32;
    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor() as u32;
    (x, y)
}

fn tiles_in_bbox(north: f64, south: f64, west: f64, east: f64, z: u8) -> Vec<(u32, u32)> {
    let n = 1u32 << z;
    let (x_min, y_min) = latlon_to_tile(north, west, z);
    let (x_max, y_max) = latlon_to_tile(south, east, z);
    let mut tiles = Vec::new();
    for x in x_min..=x_max.min(n - 1) {
        for y in y_min..=y_max.min(n - 1) {
            tiles.push((x, y));
        }
    }
    tiles
}

fn count_tiles_in_bbox(north: f64, south: f64, west: f64, east: f64, z: u8) -> usize {
    tiles_in_bbox(north, south, west, east, z).len()
}
