//! CLI for rustmaps: render tiles, benchmark, or serve tiles over HTTP.

use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--benchmark") {
        run_benchmark();
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--serve") {
        let port: u16 = args.get(pos + 1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(8080);
        run_server(port);
        return;
    }

    // Single tile mode
    let z = get_arg(&args, "--z").unwrap_or(4);
    let x = get_arg(&args, "--x").unwrap_or(4);
    let y = get_arg(&args, "--y").unwrap_or(6);
    let output = get_str_arg(&args, "--output").unwrap_or_else(|| "tile.png".to_string());

    let start = Instant::now();
    let png_data = rustmaps::render_tile(z, x, y);
    let elapsed = start.elapsed();

    std::fs::write(&output, &png_data).expect("Failed to write output file");
    println!("Rendered tile z={} x={} y={} -> {} ({} bytes, {:.2}ms)",
        z, x, y, output, png_data.len(), elapsed.as_secs_f64() * 1000.0);
}

fn run_benchmark() {
    println!("Benchmarking 100 tile renders...");
    let tiles: Vec<(u32, u32, u32)> = vec![
        // Low zoom
        (2, 1, 1), (2, 2, 1), (2, 1, 2), (2, 2, 2),
        (3, 2, 3), (3, 3, 3), (3, 4, 3), (3, 2, 2),
        // Medium zoom (US region)
        (4, 4, 6), (4, 5, 6), (4, 4, 5), (4, 3, 6),
        (5, 8, 12), (5, 9, 12), (5, 8, 11), (5, 9, 11),
        (6, 16, 24), (6, 17, 24), (6, 16, 25), (6, 17, 25),
        // Higher zoom
        (7, 32, 48), (7, 33, 48), (7, 34, 48), (7, 35, 48),
        (8, 64, 96), (8, 65, 96), (8, 64, 97), (8, 65, 97),
    ];

    // Pad to 100
    let mut all_tiles = Vec::new();
    for i in 0..100 {
        all_tiles.push(tiles[i % tiles.len()]);
    }

    // Warmup
    for &(z, x, y) in all_tiles.iter().take(5) {
        let _ = rustmaps::render_tile(z, x, y);
    }

    let start = Instant::now();
    let mut total_bytes = 0usize;
    for &(z, x, y) in &all_tiles {
        let data = rustmaps::render_tile(z, x, y);
        total_bytes += data.len();
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / 100.0;
    println!("100 tiles rendered in {:.1}ms (avg {:.2}ms/tile, {:.0} KB total)",
        elapsed.as_secs_f64() * 1000.0, avg_ms, total_bytes as f64 / 1024.0);
}

fn run_server(port: u16) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("Failed to bind");
    println!("Serving tiles at http://{}/{{z}}/{{x}}/{{y}}.png", addr);
    println!("Example: http://{}/ for a test page", addr);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut buf = [0u8; 2048];
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let request = String::from_utf8_lossy(&buf[..n]);
        let path = request.split_whitespace().nth(1).unwrap_or("/");

        if path == "/" || path == "/index.html" {
            let html = format!(r#"<!DOCTYPE html>
<html><head><title>rustmaps</title>
<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9/dist/leaflet.css"/>
<script src="https://unpkg.com/leaflet@1.9/dist/leaflet.js"></script>
<style>body{{margin:0}}#map{{width:100vw;height:100vh}}</style>
</head><body><div id="map"></div><script>
var map = L.map('map').setView([39.8, -98.5], 4);
L.tileLayer('http://{}/' + '{{z}}/{{x}}/{{y}}.png', {{
  maxZoom: 18, attribution: 'rustmaps'
}}).addTo(map);
</script></body></html>"#, addr);
            let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}", html.len(), html);
            let _ = stream.write_all(resp.as_bytes());
            continue;
        }

        // Parse /{z}/{x}/{y}.png
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if parts.len() == 3 {
            let z: u32 = parts[0].parse().unwrap_or(0);
            let x: u32 = parts[1].parse().unwrap_or(0);
            let y_str = parts[2].trim_end_matches(".png");
            let y: u32 = y_str.parse().unwrap_or(0);

            let png_data = rustmaps::render_tile(z, x, y);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                png_data.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&png_data);
        } else {
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found";
            let _ = stream.write_all(resp.as_bytes());
        }
    }
}

fn get_arg(args: &[String], name: &str) -> Option<u32> {
    args.iter().position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
}

fn get_str_arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.clone())
}
