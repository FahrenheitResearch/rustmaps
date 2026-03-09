//! Rasterization: line drawing, polygon fill, text rendering with bitmap font.

use crate::render::style::Color;

/// A 256x256 RGB pixel buffer
pub struct PixelBuffer {
    pub data: Vec<u8>,
}

impl PixelBuffer {
    pub fn new(bg: Color) -> Self {
        let mut data = vec![0u8; 256 * 256 * 3];
        for i in 0..(256 * 256) {
            data[i * 3] = bg.r;
            data[i * 3 + 1] = bg.g;
            data[i * 3 + 2] = bg.b;
        }
        Self { data }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && x < 256 && y >= 0 && y < 256 {
            let idx = (y as usize * 256 + x as usize) * 3;
            self.data[idx] = color.r;
            self.data[idx + 1] = color.g;
            self.data[idx + 2] = color.b;
        }
    }

    pub fn draw_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, color: Color, width: u32) {
        if width <= 1 {
            self.draw_line_thin(x0 as i32, y0 as i32, x1 as i32, y1 as i32, color);
        } else {
            let dx = x1 - x0;
            let dy = y1 - y0;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.5 { return; }
            let nx = -dy / len;
            let ny = dx / len;
            let half = width as f64 / 2.0;
            let steps = width;
            for i in 0..steps {
                let t = (i as f64 / (steps - 1).max(1) as f64) * 2.0 * half - half;
                let ox = nx * t;
                let oy = ny * t;
                self.draw_line_thin(
                    (x0 + ox) as i32, (y0 + oy) as i32,
                    (x1 + ox) as i32, (y1 + oy) as i32,
                    color,
                );
            }
        }
    }

    fn draw_line_thin(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: Color) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx: i32 = if x0 < x1 { 1 } else { -1 };
        let sy: i32 = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let max_steps = (dx.abs() + dy.abs() + 2) * 2;
        let mut steps = 0;

        loop {
            self.set_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 { break; }
            steps += 1;
            if steps > max_steps { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                if x0 == x1 { break; }
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                if y0 == y1 { break; }
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: u32, color: Color) {
        let r = radius as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    self.set_pixel(cx + dx, cy + dy, color);
                }
            }
        }
    }

    pub fn fill_polygon(&mut self, points: &[(f64, f64)], color: Color) {
        if points.len() < 3 { return; }
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        for &(_, y) in points {
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
        }
        let min_y = (min_y as i32).max(0);
        let max_y = (max_y as i32).min(255);
        let mut intersections: Vec<f64> = Vec::with_capacity(16);

        for y in min_y..=max_y {
            let yf = y as f64 + 0.5;
            intersections.clear();
            let n = points.len();
            for i in 0..n {
                let j = (i + 1) % n;
                let (_, y0) = points[i];
                let (_, y1) = points[j];
                if (y0 <= yf && y1 > yf) || (y1 <= yf && y0 > yf) {
                    let (x0, _) = points[i];
                    let (x1, _) = points[j];
                    let t = (yf - y0) / (y1 - y0);
                    let x = x0 + t * (x1 - x0);
                    intersections.push(x);
                }
            }
            intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut i = 0;
            while i + 1 < intersections.len() {
                let x_start = (intersections[i] as i32).max(0);
                let x_end = (intersections[i + 1] as i32).min(255);
                for x in x_start..=x_end {
                    self.set_pixel(x, y, color);
                }
                i += 2;
            }
        }
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, color: Color) {
        let mut cx = x;
        for ch in text.chars() {
            if let Some(glyph) = get_glyph(ch) {
                for row in 0..7 {
                    let bits = glyph[row];
                    for col in 0..5 {
                        if bits & (1 << (4 - col)) != 0 {
                            self.set_pixel(cx + col, y + row as i32, color);
                        }
                    }
                }
            }
            cx += 6;
        }
    }
}

fn get_glyph(ch: char) -> Option<[u8; 7]> {
    let ch = ch.to_ascii_uppercase();
    match ch {
        'A' => Some([0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'B' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
        'C' => Some([0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
        'D' => Some([0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100]),
        'E' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        'F' => Some([0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
        'G' => Some([0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110]),
        'H' => Some([0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'I' => Some([0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        'J' => Some([0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
        'K' => Some([0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
        'L' => Some([0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        'M' => Some([0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
        'N' => Some([0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
        'O' => Some([0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'P' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
        'Q' => Some([0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
        'R' => Some([0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        'S' => Some([0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110]),
        'T' => Some([0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        'U' => Some([0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        'V' => Some([0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
        'W' => Some([0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
        'X' => Some([0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
        'Y' => Some([0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
        'Z' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
        '0' => Some([0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
        '1' => Some([0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        '2' => Some([0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111]),
        '3' => Some([0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110]),
        '4' => Some([0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
        '5' => Some([0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
        '6' => Some([0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        '7' => Some([0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
        '8' => Some([0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
        '9' => Some([0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
        ' ' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000]),
        '.' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100]),
        ',' => Some([0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100, 0b01000]),
        '-' => Some([0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000]),
        _ => None,
    }
}
