use shapefile::{self, Shape};
use std::path::Path;

/// A polyline (coastline, border, river) as a list of (lon, lat) segments
pub type Polyline = Vec<(f64, f64)>;

/// A polygon (land, lake) as exterior ring of (lon, lat) points
pub type Polygon = Vec<(f64, f64)>;

pub struct City {
    pub name: String,
    pub lon: f64,
    pub lat: f64,
    pub population: u64,
    pub tier: u8, // 0 = mega city, 1 = major, 2 = medium, 3 = small
}

pub struct GeoData {
    pub coastlines_10m: Vec<Polyline>,
    pub coastlines_50m: Vec<Polyline>,
    pub coastlines_110m: Vec<Polyline>,
    pub country_borders: Vec<Polyline>,
    pub state_borders: Vec<Polyline>,
    pub land_10m: Vec<Polygon>,
    pub land_50m: Vec<Polygon>,
    pub lakes_10m: Vec<Polygon>,
    pub lakes_50m: Vec<Polygon>,
    pub rivers: Vec<Polyline>,
    pub cities: Vec<City>,
}

impl GeoData {
    pub fn load(data_dir: &Path) -> Result<Self, String> {
        eprintln!("Loading geodata from {:?}...", data_dir);

        let coastlines_110m = load_polylines(&data_dir.join("ne_110m_coastline.shp"));
        let coastlines_50m = load_polylines(&data_dir.join("ne_50m_coastline.shp"));
        let coastlines_10m = load_polylines(&data_dir.join("ne_10m_coastline.shp"));
        let country_borders = load_polylines(&data_dir.join("ne_10m_admin_0_boundary_lines_land.shp"));
        let state_borders = load_polylines_generic(&data_dir.join("ne_10m_admin_1_states_provinces_lines.shp"));
        let land_50m = load_polygons(&data_dir.join("ne_50m_land.shp"));
        let land_10m = load_polygons(&data_dir.join("ne_10m_land.shp"));
        let lakes_50m = load_polygons(&data_dir.join("ne_50m_lakes.shp"));
        let lakes_10m = load_polygons(&data_dir.join("ne_10m_lakes.shp"));
        let rivers = load_polylines(&data_dir.join("ne_10m_rivers_lake_centerlines.shp"));
        let cities = load_cities(&data_dir.join("ne_10m_populated_places_simple.shp"));

        eprintln!("  Coastlines: {} (110m), {} (50m), {} (10m)",
            coastlines_110m.len(), coastlines_50m.len(), coastlines_10m.len());
        eprintln!("  Land polygons: {} (50m), {} (10m)", land_50m.len(), land_10m.len());
        eprintln!("  Country borders: {}", country_borders.len());
        eprintln!("  State borders: {}", state_borders.len());
        eprintln!("  Lakes: {} (50m), {} (10m)", lakes_50m.len(), lakes_10m.len());
        eprintln!("  Rivers: {}", rivers.len());
        eprintln!("  Cities: {}", cities.len());

        Ok(GeoData {
            coastlines_10m,
            coastlines_50m,
            coastlines_110m,
            country_borders,
            state_borders,
            land_10m,
            land_50m,
            lakes_10m,
            lakes_50m,
            rivers,
            cities,
        })
    }

    /// Get coastlines appropriate for zoom level
    pub fn coastlines_for_zoom(&self, z: u8) -> &[Polyline] {
        match z {
            0..=3 => &self.coastlines_110m,
            4..=6 => &self.coastlines_50m,
            _ => &self.coastlines_10m,
        }
    }

    /// Get land polygons appropriate for zoom level
    pub fn land_for_zoom(&self, z: u8) -> &[Polygon] {
        match z {
            0..=6 => &self.land_50m,
            _ => &self.land_10m,
        }
    }

    /// Get lakes appropriate for zoom level
    pub fn lakes_for_zoom(&self, z: u8) -> &[Polygon] {
        match z {
            0..=6 => &self.lakes_50m,
            _ => &self.lakes_10m,
        }
    }
}

fn load_polylines(path: &Path) -> Vec<Polyline> {
    if !path.exists() {
        eprintln!("  Warning: {:?} not found", path.file_name().unwrap_or_default());
        return vec![];
    }

    let shapes = match shapefile::read_shapes_as::<_, shapefile::Polyline>(path) {
        Ok(shapes) => shapes,
        Err(e) => {
            eprintln!("  Error reading {:?}: {}", path.file_name().unwrap_or_default(), e);
            return vec![];
        }
    };

    let mut result = Vec::new();
    for shape in shapes {
        for part in shape.parts() {
            let line: Polyline = part.iter()
                .map(|p| (p.x, p.y)) // (lon, lat)
                .collect();
            if line.len() >= 2 {
                result.push(line);
            }
        }
    }
    result
}

/// Generic loader that handles mixed shape types (NullShape + Polyline)
fn load_polylines_generic(path: &Path) -> Vec<Polyline> {
    if !path.exists() {
        eprintln!("  Warning: {:?} not found", path.file_name().unwrap_or_default());
        return vec![];
    }

    let shapes = match shapefile::read_shapes(path) {
        Ok(shapes) => shapes,
        Err(e) => {
            eprintln!("  Error reading {:?}: {}", path.file_name().unwrap_or_default(), e);
            return vec![];
        }
    };

    let mut result = Vec::new();
    for shape in shapes {
        match shape {
            Shape::Polyline(pl) => {
                for part in pl.parts() {
                    let line: Polyline = part.iter().map(|p| (p.x, p.y)).collect();
                    if line.len() >= 2 {
                        result.push(line);
                    }
                }
            }
            Shape::PolylineZ(pl) => {
                for part in pl.parts() {
                    let line: Polyline = part.iter().map(|p| (p.x, p.y)).collect();
                    if line.len() >= 2 {
                        result.push(line);
                    }
                }
            }
            _ => {} // Skip NullShape and others
        }
    }
    result
}

fn load_polygons(path: &Path) -> Vec<Polygon> {
    if !path.exists() {
        eprintln!("  Warning: {:?} not found", path.file_name().unwrap_or_default());
        return vec![];
    }

    let shapes = match shapefile::read_shapes_as::<_, shapefile::Polygon>(path) {
        Ok(shapes) => shapes,
        Err(e) => {
            eprintln!("  Error reading {:?}: {}", path.file_name().unwrap_or_default(), e);
            return vec![];
        }
    };

    let mut result = Vec::new();
    for shape in shapes {
        for ring in shape.rings() {
            let points: Vec<(f64, f64)> = match ring {
                shapefile::PolygonRing::Outer(pts) => pts.iter().map(|p| (p.x, p.y)).collect(),
                shapefile::PolygonRing::Inner(pts) => pts.iter().map(|p| (p.x, p.y)).collect(),
            };
            if points.len() >= 3 {
                result.push(points);
            }
        }
    }
    result
}

fn load_cities(path: &Path) -> Vec<City> {
    if !path.exists() {
        eprintln!("  Warning: cities shapefile not found");
        return vec![];
    }

    let records = match shapefile::read(path) {
        Ok(records) => records,
        Err(e) => {
            eprintln!("  Error reading cities: {}", e);
            return vec![];
        }
    };

    let mut cities = Vec::new();
    for (shape, record) in records {
        if let Shape::Point(point) = shape {
            let name = record.get("name")
                .and_then(|v| match v {
                    shapefile::dbase::FieldValue::Character(Some(s)) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            let pop = record.get("pop_max")
                .and_then(|v| match v {
                    shapefile::dbase::FieldValue::Numeric(Some(n)) => Some(*n as u64),
                    _ => None,
                })
                .unwrap_or(0);

            let tier = if pop >= 5_000_000 { 0 }
                      else if pop >= 1_000_000 { 1 }
                      else if pop >= 300_000 { 2 }
                      else { 3 };

            cities.push(City {
                name,
                lon: point.x,
                lat: point.y,
                population: pop,
                tier,
            });
        }
    }

    cities.sort_by(|a, b| b.population.cmp(&a.population));
    cities
}
