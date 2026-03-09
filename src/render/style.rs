//! Dark theme styling for weather/radar map backgrounds.

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }
}

// Dark theme - optimized for radar/weather overlay visibility
pub const COLOR_OCEAN: Color = Color::from_hex(0x0d1117);
pub const COLOR_LAND: Color = Color::from_hex(0x161b22);
pub const COLOR_COASTLINE: Color = Color::from_hex(0x4a8fd4);
pub const COLOR_COUNTRY_BORDER: Color = Color::from_hex(0x7d8590);
pub const COLOR_STATE_BORDER: Color = Color::from_hex(0x3d444d);
pub const COLOR_LAKE: Color = Color::from_hex(0x0d1117);
pub const COLOR_RIVER: Color = Color::from_hex(0x1a3a5c);
pub const COLOR_CITY_DOT: Color = Color::from_hex(0xc9d1d9);
pub const COLOR_CITY_LABEL: Color = Color::from_hex(0x8b949e);
pub const COLOR_MAJOR_CITY_LABEL: Color = Color::from_hex(0xc9d1d9);

pub fn coastline_width(z: u8) -> f32 {
    match z {
        0..=2 => 0.8,
        3..=4 => 1.0,
        5..=6 => 1.3,
        7..=8 => 1.6,
        _ => 2.0,
    }
}

pub fn border_width(z: u8) -> f32 {
    match z {
        0..=3 => 0.6,
        4..=6 => 0.8,
        7..=8 => 1.0,
        _ => 1.3,
    }
}

pub fn river_width(z: u8) -> f32 {
    match z {
        0..=5 => 0.5,
        6..=7 => 0.7,
        _ => 1.0,
    }
}

pub fn show_state_borders(z: u8) -> bool { z >= 2 }
pub fn show_lakes(z: u8) -> bool { z >= 3 }
pub fn show_rivers(z: u8) -> bool { z >= 6 }

pub fn show_city_tier(tier: u8, z: u8) -> bool {
    match tier {
        0 => z >= 3,
        1 => z >= 4,
        2 => z >= 6,
        3 => z >= 8,
        _ => false,
    }
}

pub fn show_city_labels(z: u8) -> bool { z >= 5 }

pub fn city_dot_radius(tier: u8, z: u8) -> f32 {
    let base = match tier {
        0 => 3.0,
        1 => 2.5,
        2 => 2.0,
        _ => 1.5,
    };
    base + (z as f32 - 5.0).max(0.0) * 0.3
}
