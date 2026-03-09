//! rustmaps CLI - render dark-theme map tiles for weather/radar backgrounds.

use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use rayon::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let data_dir = get_str_arg(&args, "--data")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\drew\rustmaps\data"));

    // Tile server mode
    if args.iter().any(|a| a == "--serve") {
        let port: u16 = get_str_arg(&args, "--port")
            .and_then(|s| s.parse().ok())
            .unwrap_or(8080);
        run_server(&data_dir, port);
        return;
    }

    eprintln!("rustmaps - Dark Theme Weather Map Tile Renderer");

    let t0 = Instant::now();
    let renderer = rustmaps::load_renderer(&data_dir)
        .expect("Failed to load geodata");
    eprintln!("Geodata loaded in {:.1}ms", t0.elapsed().as_secs_f64() * 1000.0);

    let output_dir = get_str_arg(&args, "--output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\drew\rustmaps\output"));

    // Generate tiles at zoom levels 3-7 covering CONUS
    let zoom_configs: Vec<(u8, Vec<(u32, u32)>)> = vec![
        (3, conus_tiles(3)),
        (4, conus_tiles(4)),
        (5, conus_tiles(5)),
        (6, conus_tiles(6)),
        (7, conus_tiles(7)),
    ];

    for (z, tiles) in &zoom_configs {
        let t1 = Instant::now();
        let count = tiles.len();

        tiles.par_iter().for_each(|&(x, y)| {
            let tile = renderer.render_tile(*z, x, y);
            let png = tile.to_png_rgb();
            let dir = output_dir.join(format!("{}/{}", z, x));
            fs::create_dir_all(&dir).ok();
            let path = dir.join(format!("{}.png", y));
            fs::write(&path, &png).ok();
        });

        let elapsed = t1.elapsed();
        eprintln!("z{}: {} tiles in {:.1}ms ({:.1}ms/tile)",
            z, count, elapsed.as_secs_f64() * 1000.0,
            elapsed.as_secs_f64() * 1000.0 / count as f64);
    }

    generate_showcase(&renderer, &output_dir);
    eprintln!("\nDone! Tiles written to {:?}", output_dir);
}

/// Get all tile coordinates covering CONUS at a given zoom level
fn conus_tiles(z: u8) -> Vec<(u32, u32)> {
    let n = 1u32 << z;
    let (x_min, y_min) = latlon_to_tile(50.0, -125.0, z);
    let (x_max, y_max) = latlon_to_tile(24.0, -66.0, z);

    let mut tiles = Vec::new();
    for x in x_min..=x_max.min(n - 1) {
        for y in y_min..=y_max.min(n - 1) {
            tiles.push((x, y));
        }
    }
    tiles
}

fn latlon_to_tile(lat: f64, lon: f64, z: u8) -> (u32, u32) {
    let n = (1u32 << z) as f64;
    let x = ((lon + 180.0) / 360.0 * n).floor() as u32;
    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor() as u32;
    (x, y)
}

fn run_server(data_dir: &std::path::Path, port: u16) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    eprintln!("Loading geodata...");
    let renderer = rustmaps::load_renderer(data_dir).expect("Failed to load geodata");
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    eprintln!("Tile server at http://{}/", addr);

    for stream in listener.incoming() {
        let mut stream = match stream { Ok(s) => s, Err(_) => continue };
        let mut buf = [0u8; 4096];
        let n = match stream.read(&mut buf) { Ok(n) => n, Err(_) => continue };
        let request = String::from_utf8_lossy(&buf[..n]);
        let path = request.split_whitespace().nth(1).unwrap_or("/");

        if path == "/" || path == "/index.html" {
            let html = format!(r#"<!DOCTYPE html>
<html><head><title>rustmaps</title>
<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9/dist/leaflet.css"/>
<script src="https://unpkg.com/leaflet@1.9/dist/leaflet.js"></script>
<style>body{{margin:0;background:#0d1117}}#map{{width:100vw;height:100vh}}</style>
</head><body><div id="map"></div><script>
var map = L.map('map',{{zoomControl:true}}).setView([39,-96],5);
L.tileLayer('http://{}/' + '{{z}}/{{x}}/{{y}}.png',{{
  maxZoom:10,minZoom:2,attribution:'rustmaps'
}}).addTo(map);
</script></body></html>"#, addr);
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}", html.len(), html);
            let _ = stream.write_all(resp.as_bytes());
            continue;
        }

        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if parts.len() == 3 {
            if let (Ok(z), Ok(x)) = (parts[0].parse::<u8>(), parts[1].parse::<u32>()) {
                let y_str = parts[2].trim_end_matches(".png");
                if let Ok(y) = y_str.parse::<u32>() {
                    let t = Instant::now();
                    let png_data = renderer.render_tile_png(z, x, y);
                    let ms = t.elapsed().as_secs_f64() * 1000.0;
                    eprintln!("  z{}/{}/{} -> {} bytes ({:.1}ms)", z, x, y, png_data.len(), ms);
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: max-age=86400\r\n\r\n",
                        png_data.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(&png_data);
                    continue;
                }
            }
        }

        let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found";
        let _ = stream.write_all(resp.as_bytes());
    }
}

fn get_str_arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.clone())
}

fn generate_showcase(renderer: &rustmaps::render::TileRenderer, output_dir: &std::path::Path) {
    let showcase_dir = output_dir.join("showcase");
    fs::create_dir_all(&showcase_dir).ok();

    let showcase_tiles: Vec<(u8, u32, u32, &str)> = vec![
        (4, 4, 5, "conus_z4"),
        (5, 8, 11, "texas_z5"),
        (5, 9, 11, "southeast_z5"),
        (5, 9, 10, "northeast_z5"),
        (6, 16, 23, "oklahoma_z6"),
        (6, 17, 24, "gulf_coast_z6"),
        (7, 32, 47, "stl_z7"),
        (7, 35, 49, "atlanta_z7"),
    ];

    for (z, x, y, name) in showcase_tiles {
        let tile = renderer.render_tile(z, x, y);
        let png = tile.to_png_rgb();
        let path = showcase_dir.join(format!("{}.png", name));
        fs::write(&path, &png).ok();
        eprintln!("Showcase: {} (z{}/{}/{}) - {} bytes", name, z, x, y, png.len());
    }
}
