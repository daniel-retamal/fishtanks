use ratatui::style::Color;

pub const BLACK: Color = Color::Black;
pub const DARK_GRAY: Color = Color::DarkGray;
pub const GRAY: Color = Color::Gray;
pub const WHITE: Color = Color::White;
pub const RED: Color = Color::Red;
pub const LIGHT_RED: Color = Color::LightRed;
pub const GREEN: Color = Color::Green;
pub const LIGHT_GREEN: Color = Color::LightGreen;
pub const YELLOW: Color = Color::Yellow;
pub const LIGHT_YELLOW: Color = Color::LightYellow;
pub const BLUE: Color = Color::Blue;
pub const LIGHT_BLUE: Color = Color::LightBlue;
pub const MAGENTA: Color = Color::Magenta;
pub const LIGHT_MAGENTA: Color = Color::LightMagenta;
pub const CYAN: Color = Color::Cyan;
pub const LIGHT_CYAN: Color = Color::LightCyan;

pub const GOLD: Color = Color::Rgb(255, 215, 0);
pub const GOLD_PALE: Color = Color::Rgb(255, 240, 150);
pub const GOLD_BRIGHT: Color = Color::Rgb(255, 255, 100);

pub const AMBER_DARK: Color = Color::Rgb(200, 130, 0);
pub const AMBER: Color = Color::Rgb(255, 170, 50);
pub const AMBER_LIGHT: Color = Color::Rgb(255, 210, 110);

pub const ORANGE_DARK: Color = Color::Rgb(210, 60, 0);
pub const ORANGE: Color = Color::Rgb(255, 120, 0);
pub const ORANGE_LIGHT: Color = Color::Rgb(255, 170, 90);

pub const PINK: Color = Color::Rgb(255, 105, 180);

pub const RED_DARK: Color = Color::Rgb(80, 0, 0);
pub const RED_PALE: Color = Color::Rgb(225, 120, 115);

pub const GREEN_DARK: Color = Color::Rgb(0, 100, 0);
pub const FOREST: Color = Color::Rgb(0, 130, 40);
pub const GREEN_LIGHT: Color = Color::Rgb(0, 180, 60);
pub const GREEN_BRIGHT: Color = Color::Rgb(60, 220, 100);

pub const OLIVE: Color = Color::Rgb(120, 150, 50);
pub const OLIVE_LIGHT: Color = Color::Rgb(175, 200, 110);

pub const COBALT_DARK: Color = Color::Rgb(0, 30, 110);
pub const COBALT: Color = Color::Rgb(0, 50, 160);
pub const COBALT_LIGHT: Color = Color::Rgb(0, 70, 190);

pub const NAVY_DARK: Color = Color::Rgb(20, 60, 200);
pub const NAVY: Color = Color::Rgb(60, 110, 240);
pub const NAVY_LIGHT: Color = Color::Rgb(100, 160, 255);

pub const PURPLE_DARK: Color = Color::Rgb(60, 0, 110);
pub const PURPLE: Color = Color::Rgb(90, 0, 160);
pub const PURPLE_LIGHT: Color = Color::Rgb(130, 0, 190);
pub const VIOLET: Color = Color::Rgb(165, 65, 230);
pub const INDIGO: Color = Color::Rgb(100, 60, 180);

pub const TEAL: Color = Color::Rgb(0, 180, 150);

pub const CREAM: Color = Color::Rgb(245, 230, 180);
pub const SILVER: Color = Color::Rgb(220, 220, 220);
pub const STEEL: Color = Color::Rgb(180, 190, 210);

pub const BROWN_DARK: Color = Color::Rgb(120, 80, 40);
pub const BROWN: Color = Color::Rgb(160, 100, 40);
pub const TAN: Color = Color::Rgb(200, 160, 120);
pub const KHAKI: Color = Color::Rgb(220, 180, 50);
pub const TERRACOTTA: Color = Color::Rgb(220, 100, 80);

const CHANNEL_MAX: u8 = u8::MAX;

pub fn rgb_of(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (204, 204, 204),
        Color::DarkGray => (118, 118, 118),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White | Color::Reset | Color::Indexed(_) => (242, 242, 242),
    }
}

pub fn inverted(color: Color) -> Color {
    let (r, g, b) = rgb_of(color);
    Color::Rgb(CHANNEL_MAX - r, CHANNEL_MAX - g, CHANNEL_MAX - b)
}

pub fn blended(from: Color, to: Color, amount: f32) -> Color {
    let (fr, fg, fb) = rgb_of(from);
    let (tr, tg, tb) = rgb_of(to);
    let mix =
        |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount.clamp(0.0, 1.0)).round() as u8;
    Color::Rgb(mix(fr, tr), mix(fg, tg), mix(fb, tb))
}
