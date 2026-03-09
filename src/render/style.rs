//! Styling configuration: colors and line widths per zoom level.

/// RGB color
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }
}

// Dark theme colors to match NexView
pub const COLOR_OCEAN: Color = Color::from_hex(0x1a1a2e);
pub const COLOR_LAND: Color = Color::from_hex(0x16213e);
pub const COLOR_COASTLINE: Color = Color::from_hex(0x4a90d9);
pub const COLOR_COUNTRY_BORDER: Color = Color::from_hex(0x666680);
pub const COLOR_STATE_BORDER: Color = Color::from_hex(0x444460);
pub const COLOR_LAKE: Color = Color::from_hex(0x1a1a2e);
pub const COLOR_RIVER: Color = Color::from_hex(0x3a6090);
pub const COLOR_CITY_DOT: Color = Color::from_hex(0xffffff);
pub const COLOR_CITY_LABEL: Color = Color::from_hex(0xcccccc);
pub const COLOR_MAJOR_CITY_LABEL: Color = Color::from_hex(0xffffff);

/// Get coastline line width for a zoom level
pub fn coastline_width(_z: u32) -> u32 {
    1
}

/// Get border line width
pub fn border_width(_z: u32) -> u32 {
    1
}

/// Get river line width
pub fn river_width(z: u32) -> u32 {
    if z >= 8 { 2 } else { 1 }
}

/// Get city dot radius based on population tier
pub fn city_dot_radius(tier: u8, z: u32) -> u32 {
    match tier {
        1 => if z >= 8 { 4 } else { 3 },
        2 => if z >= 8 { 3 } else { 2 },
        _ => 2,
    }
}

/// Whether to show a feature at the given zoom level
pub fn show_state_borders(z: u32) -> bool { z >= 4 }
pub fn show_lakes(z: u32) -> bool { z >= 4 }
pub fn show_rivers(z: u32) -> bool { z >= 6 }

pub fn show_city_tier(tier: u8, z: u32) -> bool {
    match tier {
        1 => z >= 4,
        2 => z >= 6,
        3 => z >= 8,
        _ => z >= 10,
    }
}

pub fn show_city_labels(z: u32) -> bool { z >= 5 }
