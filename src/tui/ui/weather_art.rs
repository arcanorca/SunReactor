//! Weather scenes as colour pixel art.
//!
//! A scene is a 32×16 pixel canvas rendered with half blocks, so every
//! terminal cell carries two full-colour pixels. Sprites use a small natural
//! palette with a light source from the upper left: highlights on top, shade
//! and a soft shadow underneath. Transparent pixels show the panel
//! background. Each pixel also has a role (sun, cloud, rain, grass…), and its
//! colour leans 30 % toward the theme colour for that role, so the art keeps
//! its natural look while belonging to every theme. With Full effects the sky moves
//! once a second (falling rain and snow, drifting fog, twinkling stars).
//! The moon is drawn from the real lunar age.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::tui::theme::Palette;
use crate::weather::{WeatherCondition, WeatherDayPhase};

/// Terminal cells occupied by one scene.
pub(crate) const ART_WIDTH: u16 = VIEW_W as u16;
pub(crate) const ART_ROWS: u16 = (VIEW_H / 2) as u16;

/// Scenes are composed on a 32×16 canvas; only the window starting at
/// (`VIEW_X`, `VIEW_Y`) is shown, which trims empty sky around the sprites.
const W: usize = 32;
const H: usize = 16;
const VIEW_X: usize = 3;
const VIEW_Y: usize = 2;
const VIEW_W: usize = 26;
const VIEW_H: usize = 14;

/// How strongly a sprite colour leans toward its theme colour.
const THEME_TINT: f64 = 0.3;

/// What a pixel depicts, which decides the theme colour it leans toward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Sun,
    Moon,
    Shade,
    Cloud,
    Rain,
    Snow,
    Fog,
}

impl Role {
    fn theme_color(self, palette: &Palette) -> Color {
        match self {
            Self::Sun => palette.warning,
            Self::Moon | Self::Cloud | Self::Snow => palette.fg,
            Self::Shade => palette.border_inactive,
            Self::Rain => palette.secondary_accent,
            Self::Fog => palette.text_muted,
        }
    }
}

type Rgb = (u8, u8, u8);

/// How the moon should look: its age in the synodic month (0 = new,
/// 0.5 = full) and whether it is seen from the southern hemisphere, where the
/// lit side is mirrored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MoonView {
    pub age: f64,
    pub southern: bool,
}

impl MoonView {
    /// Fraction of the visible disc that is lit.
    #[must_use]
    pub(crate) fn illumination(self) -> f64 {
        (1.0 - (std::f64::consts::TAU * self.age).cos()) / 2.0
    }

    #[must_use]
    pub(crate) fn phase_name(self) -> &'static str {
        match self.age {
            a if !(0.0339..0.9661).contains(&a) => "New moon",
            a if a < 0.2161 => "Waxing crescent",
            a if a < 0.2839 => "First quarter",
            a if a < 0.4661 => "Waxing gibbous",
            a if a < 0.5339 => "Full moon",
            a if a < 0.7161 => "Waning gibbous",
            a if a < 0.7839 => "Last quarter",
            _ => "Waning crescent",
        }
    }
}

pub(crate) struct WeatherArt {
    pub lines: Vec<Line<'static>>,
}

// --- Palette -------------------------------------------------------------

const SUN_HIGHLIGHT: Rgb = (0xff, 0xf6, 0xc4);
const SUN_CORE: Rgb = (0xff, 0xd6, 0x4a);
const SUN_SHADE: Rgb = (0xfb, 0xb1, 0x34);
const SUN_RIM: Rgb = (0xe8, 0x85, 0x1f);
const SUN_RAY: Rgb = (0xff, 0xc8, 0x4d);

const MOON_LIGHT: Rgb = (0xf6, 0xf1, 0xd6);
const MOON_SHADE: Rgb = (0xd9, 0xd1, 0xa8);
const MOON_CRATER: Rgb = (0xbf, 0xb5, 0x8a);
const MOON_DARK: Rgb = (0x3c, 0x44, 0x5c);

const STAR_BRIGHT: Rgb = (0xff, 0xf3, 0xb8);
const STAR_DIM: Rgb = (0x8e, 0x9c, 0xc4);

const RAIN: Rgb = (0x5d, 0xa9, 0xef);
const RAIN_LIGHT: Rgb = (0xa9, 0xda, 0xff);
const SNOW: Rgb = (0xff, 0xff, 0xff);
const SNOW_SHADE: Rgb = (0xcf, 0xe0, 0xf2);
const BOLT: Rgb = (0xff, 0xf2, 0x8f);
const BOLT_EDGE: Rgb = (0xf2, 0xb1, 0x1a);
const FOG: Rgb = (0xd3, 0xd9, 0xe2);
const FOG_SHADE: Rgb = (0xa8, 0xb2, 0xc0);

/// Hand-toned cloud sprites. `H` highlight, `L` light, `M` mid, `S` shadow,
/// `D` deep shadow; `.` is transparent. Light comes from the upper left and
/// the underside settles into a soft shadow instead of a hard outline.
const BIG_CLOUD: [&str; 10] = [
    "........HHHH..........",
    "......HHLLLLHH........",
    ".....HLLLLLLLLH.HHH...",
    "..HHHLLLLLLLLLLHLLLH..",
    ".HLLLLLLLLLLLLLLLLLLH.",
    "HLLLLLLLLLLLLLLLLLLLLM",
    "LLLLLLLLLLLLLLLLLLLLLM",
    "MLLLLLLLLLLLLLLLLLLLMS",
    ".MMMMMMLLLLLLLLLMMMMS.",
    "..SSSSSMMMMMMMMSSSSD..",
];

const SMALL_CLOUD: [&str; 6] = [
    "....HHHH......",
    "..HHLLLLH.HH..",
    ".HLLLLLLLHLLH.",
    "HLLLLLLLLLLLLM",
    "MMLLLLLLLLLLMS",
    ".SSMMMMMMMMSS.",
];

/// Five tones for a cloud, lightest first.
#[derive(Clone, Copy)]
struct CloudTones([Rgb; 5]);

const CLOUD_WHITE: CloudTones = CloudTones([
    (0xff, 0xff, 0xff),
    (0xf0, 0xf4, 0xf9),
    (0xd3, 0xdd, 0xea),
    (0xa8, 0xb6, 0xc9),
    (0x7b, 0x8a, 0xa2),
]);
const CLOUD_GRAY: CloudTones = CloudTones([
    (0xe0, 0xe5, 0xed),
    (0xc2, 0xc9, 0xd5),
    (0xa2, 0xab, 0xba),
    (0x7e, 0x87, 0x98),
    (0x5a, 0x62, 0x72),
]);
const CLOUD_STORM: CloudTones = CloudTones([
    (0x9c, 0xa4, 0xb4),
    (0x7f, 0x88, 0x99),
    (0x66, 0x6e, 0x7f),
    (0x4e, 0x55, 0x66),
    (0x38, 0x3e, 0x4c),
]);

/// Darkens a colour for night skies while keeping a cool tint.
fn night(color: Rgb) -> Rgb {
    let scale = |value: u8, factor: f64| (f64::from(value) * factor).round() as u8;
    (
        scale(color.0, 0.62),
        scale(color.1, 0.66),
        scale(color.2, 0.78),
    )
}

fn night_tones(tones: CloudTones) -> CloudTones {
    CloudTones(tones.0.map(night))
}

// --- Canvas --------------------------------------------------------------

struct Canvas {
    pixels: [[Option<(Rgb, Role)>; W]; H],
    /// Role given to the pixels the current sprite draws.
    role: Role,
}

impl Canvas {
    fn new() -> Self {
        Self {
            pixels: [[None; W]; H],
            role: Role::Cloud,
        }
    }

    fn put(&mut self, x: i32, y: i32, color: Rgb) {
        if (0..W as i32).contains(&x) && (0..H as i32).contains(&y) {
            self.pixels[y as usize][x as usize] = Some((color, self.role));
        }
    }

    fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        if (0..W as i32).contains(&x) && (0..H as i32).contains(&y) {
            self.pixels[y as usize][x as usize].map(|(color, _)| color)
        } else {
            None
        }
    }

    /// Half blocks over the visible window: the upper pixel is the
    /// foreground of `▀`, the lower one its background; a transparent half
    /// leaves the panel colour.
    fn lines(&self, palette: &Palette) -> Vec<Line<'static>> {
        let paint = |pixel: Option<(Rgb, Role)>| {
            pixel.map(|(color, role)| {
                super::light_cycle::mix(rgb(color), role.theme_color(palette), THEME_TINT)
            })
        };
        (0..VIEW_H / 2)
            .map(|row| {
                let y = VIEW_Y + row * 2;
                let spans = (VIEW_X..VIEW_X + VIEW_W)
                    .map(|column| {
                        let top = paint(self.pixels[y][column]);
                        let bottom = paint(self.pixels[y + 1][column]);
                        match (top, bottom) {
                            (None, None) => Span::raw(" "),
                            (Some(top), None) => Span::styled("▀", Style::default().fg(top)),
                            (None, Some(bottom)) => Span::styled("▄", Style::default().fg(bottom)),
                            (Some(top), Some(bottom)) if top == bottom => {
                                Span::styled("█", Style::default().fg(top))
                            }
                            (Some(top), Some(bottom)) => {
                                Span::styled("▀", Style::default().fg(top).bg(bottom))
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                Line::from(spans)
            })
            .collect()
    }

    // --- Sprites ---------------------------------------------------------

    /// A round sun lit from the upper left, with rays that pulse between
    /// long and short on alternate frames.
    fn sun(&mut self, cx: f64, cy: f64, radius: f64, frame: u64, rays: bool) {
        self.role = Role::Sun;
        if rays {
            for ray in 0..8_u64 {
                let angle = std::f64::consts::FRAC_PI_4 * ray as f64;
                let long = (ray + frame).is_multiple_of(2);
                let length = if long { 2 } else { 1 };
                for step in 0..length {
                    let distance = radius + 1.6 + f64::from(step);
                    self.put(
                        (cx + angle.cos() * distance).floor() as i32,
                        (cy + angle.sin() * distance).floor() as i32,
                        SUN_RAY,
                    );
                }
            }
        }
        for y in 0..H as i32 {
            for x in 0..W as i32 {
                let dx = f64::from(x) + 0.5 - cx;
                let dy = f64::from(y) + 0.5 - cy;
                let distance = dx.hypot(dy);
                if distance > radius {
                    continue;
                }
                let light = (-dx * 0.6 - dy * 0.8) / radius;
                let color = if distance > radius - 1.0 {
                    SUN_RIM
                } else if light > 0.4 {
                    SUN_HIGHLIGHT
                } else if light > -0.25 {
                    SUN_CORE
                } else {
                    SUN_SHADE
                };
                self.put(x, y, color);
            }
        }
    }

    /// The moon lit by its age. The terminator is the projected day/night
    /// boundary: `x = cos(2π·age)·√(1 − y²)`.
    fn moon(&mut self, cx: f64, cy: f64, radius: f64, view: MoonView) {
        self.role = Role::Moon;
        let terminator = (std::f64::consts::TAU * view.age).cos();
        let waxing = view.age < 0.5;
        let craters = [(-0.35, -0.3, 0.2), (0.3, 0.25, 0.24), (-0.1, 0.5, 0.14)];
        for y in 0..H as i32 {
            for x in 0..W as i32 {
                let nx = (f64::from(x) + 0.5 - cx) / radius;
                let ny = (f64::from(y) + 0.5 - cy) / radius;
                if nx * nx + ny * ny > 1.0 {
                    continue;
                }
                let mirrored = if view.southern { -nx } else { nx };
                let edge = (1.0 - ny * ny).sqrt();
                let lit = if waxing {
                    mirrored > terminator * edge
                } else {
                    mirrored < -terminator * edge
                };
                self.role = if lit { Role::Moon } else { Role::Shade };
                let color = if !lit {
                    MOON_DARK
                } else if craters
                    .iter()
                    .any(|(x0, y0, r)| (nx - x0).hypot(ny - y0) < *r)
                {
                    MOON_CRATER
                } else if nx + ny > 0.45 {
                    MOON_SHADE
                } else {
                    MOON_LIGHT
                };
                self.put(x, y, color);
            }
        }
    }

    fn stars(&mut self, frame: u64) {
        const STARS: [(i32, i32); 9] = [
            (4, 3),
            (7, 5),
            (5, 10),
            (25, 3),
            (27, 7),
            (21, 2),
            (26, 11),
            (13, 2),
            (3, 6),
        ];
        self.role = Role::Sun;
        for (index, (x, y)) in STARS.iter().enumerate() {
            if self.get(*x, *y).is_some() {
                continue;
            }
            let bright = !(frame + index as u64).is_multiple_of(3);
            self.put(*x, *y, if bright { STAR_BRIGHT } else { STAR_DIM });
        }
    }

    /// A hand-toned cloud sprite at `(left, top)`, painted with `tones`.
    fn cloud(&mut self, left: i32, top: i32, shape: &[&str], tones: CloudTones) {
        let [highlight, light, mid, shadow, deep] = tones.0;
        for (y, row) in shape.iter().enumerate() {
            for (x, code) in row.bytes().enumerate() {
                let (color, role) = match code {
                    b'H' => (highlight, Role::Cloud),
                    b'L' => (light, Role::Cloud),
                    b'M' => (mid, Role::Cloud),
                    b'S' => (shadow, Role::Shade),
                    b'D' => (deep, Role::Shade),
                    _ => continue,
                };
                self.role = role;
                self.put(left + x as i32, top + y as i32, color);
            }
        }
    }

    /// Falling streaks; `density` is the number of columns, `slant` moves the
    /// lower pixel sideways for driving rain.
    fn rain(&mut self, top: i32, bottom: i32, density: usize, slant: bool, frame: u64) {
        const COLUMNS: [i32; 12] = [4, 20, 11, 27, 7, 16, 24, 13, 26, 5, 18, 9];
        self.role = Role::Rain;
        let span = (bottom - top).max(1);
        for (index, x) in COLUMNS.iter().take(density).enumerate() {
            let offset = (index as i32 * 5 + (frame % 64) as i32 * 2) % span;
            let y = top + offset;
            self.put(*x, y, RAIN_LIGHT);
            self.put(if slant { x - 1 } else { *x }, y + 1, RAIN);
        }
    }

    fn snow(&mut self, top: i32, bottom: i32, frame: u64) {
        const FLAKES: [(i32, i32); 10] = [
            (4, 0),
            (12, 3),
            (21, 1),
            (28, 4),
            (8, 5),
            (17, 6),
            (25, 7),
            (4, 6),
            (14, 0),
            (26, 2),
        ];
        self.role = Role::Snow;
        let span = (bottom - top).max(1);
        for (index, (x, phase)) in FLAKES.iter().enumerate() {
            let y = top + (phase + (frame % 64) as i32) % span;
            let drift = i32::from((frame as i32 + index as i32) % 4 < 2);
            let x = x + drift;
            if index % 3 == 0 {
                self.put(x, y, SNOW);
                self.put(x - 1, y, SNOW_SHADE);
                self.put(x + 1, y, SNOW_SHADE);
                self.put(x, y - 1, SNOW_SHADE);
                self.put(x, y + 1, SNOW_SHADE);
            } else {
                self.put(x, y, SNOW);
            }
        }
    }

    fn bolt(&mut self, x: i32, y: i32, frame: u64) {
        const SHAPE: [&str; 8] = [
            "..##", ".##.", "###.", "####", ".##.", ".#..", "##..", "#...",
        ];
        let flash = frame.is_multiple_of(3);
        self.role = Role::Sun;
        for (row, line) in SHAPE.iter().enumerate() {
            for (column, pixel) in line.chars().enumerate() {
                if pixel != '#' {
                    continue;
                }
                let px = x + column as i32;
                let py = y + row as i32;
                let edge = column == 0 || !line[column + 1..].starts_with('#');
                let color = if edge && !flash { BOLT_EDGE } else { BOLT };
                self.put(px, py, color);
            }
        }
    }

    /// Drifting horizontal bands; `bands` is how many rows of fog to draw.
    fn fog(&mut self, top: i32, bands: i32, frame: u64, is_night: bool) {
        self.role = Role::Fog;
        for band in 0..bands {
            let y = top + band * 2;
            let shift = (frame as i32 + band * 3) % 9;
            for x in 0..W as i32 {
                if (x + shift) % 9 < 6 {
                    let color = if band % 2 == 0 { FOG } else { FOG_SHADE };
                    let color = if is_night { night(color) } else { color };
                    self.put(x, y, color);
                }
            }
        }
    }
}

fn rgb(color: Rgb) -> Color {
    Color::Rgb(color.0, color.1, color.2)
}

fn fallback_condition(
    condition: WeatherCondition,
    cloud_cover_percent: Option<u8>,
) -> WeatherCondition {
    if condition != WeatherCondition::Unknown {
        return condition;
    }
    match cloud_cover_percent {
        Some(0..=19) => WeatherCondition::Clear,
        Some(20..=59) => WeatherCondition::PartlyCloudy,
        Some(_) => WeatherCondition::Cloudy,
        None => WeatherCondition::Unknown,
    }
}

fn scene(
    condition: WeatherCondition,
    cloud_cover_percent: Option<u8>,
    day_phase: WeatherDayPhase,
    moon: Option<MoonView>,
    frame: u64,
) -> Canvas {
    let is_night = day_phase == WeatherDayPhase::Night;
    let condition = fallback_condition(condition, cloud_cover_percent);
    let mut canvas = Canvas::new();
    if condition == WeatherCondition::Unknown {
        return canvas;
    }
    let tones = |tones: CloudTones| if is_night { night_tones(tones) } else { tones };
    let bottom = H as i32;

    let sky_light = |canvas: &mut Canvas, cx: f64, cy: f64, radius: f64| {
        if is_night {
            canvas.stars(frame);
            if let Some(view) = moon {
                canvas.moon(cx, cy, radius + 0.5, view);
            }
        } else {
            canvas.sun(cx, cy, radius, frame, true);
        }
    };

    // Positions are in canvas pixels; the visible window starts at (3, 2).
    match condition {
        WeatherCondition::Clear => sky_light(&mut canvas, 16.0, 9.0, 4.6),
        WeatherCondition::PartlyCloudy => {
            sky_light(&mut canvas, 12.0, 6.5, 3.9);
            canvas.cloud(7, 6, &BIG_CLOUD, tones(CLOUD_WHITE));
        }
        WeatherCondition::Cloudy => {
            canvas.cloud(5, 3, &SMALL_CLOUD, tones(CLOUD_GRAY));
            canvas.cloud(6, 5, &BIG_CLOUD, tones(CLOUD_WHITE));
        }
        WeatherCondition::Drizzle => {
            canvas.cloud(5, 2, &BIG_CLOUD, tones(CLOUD_GRAY));
            canvas.rain(12, bottom, 5, false, frame);
        }
        WeatherCondition::Rain => {
            canvas.cloud(5, 2, &BIG_CLOUD, tones(CLOUD_GRAY));
            canvas.rain(12, bottom, 9, false, frame);
        }
        WeatherCondition::HeavyRain => {
            canvas.cloud(3, 2, &SMALL_CLOUD, tones(CLOUD_STORM));
            canvas.cloud(7, 3, &BIG_CLOUD, tones(CLOUD_STORM));
            canvas.rain(13, bottom, 12, true, frame);
        }
        WeatherCondition::Thunderstorm => {
            canvas.cloud(5, 2, &BIG_CLOUD, tones(CLOUD_STORM));
            canvas.rain(12, bottom, 7, true, frame);
            canvas.bolt(14, 10, frame);
        }
        WeatherCondition::Snow => {
            canvas.cloud(5, 2, &BIG_CLOUD, tones(CLOUD_WHITE));
            canvas.snow(12, bottom, frame);
        }
        WeatherCondition::Mist | WeatherCondition::Atmospheric => {
            sky_light(&mut canvas, 16.0, 7.5, 3.6);
            canvas.fog(9, 3, frame, is_night);
        }
        WeatherCondition::Fog => {
            canvas.fog(4, 6, frame, is_night);
        }
        WeatherCondition::Unknown => {}
    }
    canvas
}

/// Draws one scene. `frame` advances the sky animation; pass a constant to
/// keep it still.
pub(crate) fn weather_art(
    condition: WeatherCondition,
    cloud_cover_percent: Option<u8>,
    day_phase: WeatherDayPhase,
    moon: Option<MoonView>,
    frame: u64,
    palette: &Palette,
) -> WeatherArt {
    WeatherArt {
        lines: scene(condition, cloud_cover_percent, day_phase, moon, frame).lines(palette),
    }
}

/// Hand-drawn 7×4 pixel icons for the forecast strip. Letters pick a
/// colour and role from [`mini_color`]; `.` is transparent.
fn mini_sprite(condition: WeatherCondition, night: bool) -> [&'static str; 4] {
    match (fallback_condition(condition, None), night) {
        (WeatherCondition::Clear, false) => ["..oyo..", ".oyhyo.", ".oyyyo.", "..oyo.."],
        (WeatherCondition::Clear, true) => ["..mm...", ".mm....", ".mm....", "..mm..."],
        (WeatherCondition::PartlyCloudy, false) => [".oyo...", "oyywww.", ".wwwwww", "..sssss"],
        (WeatherCondition::PartlyCloudy, true) => [".mm....", "mm.www.", "m.wwwww", "..sssss"],
        (WeatherCondition::Cloudy, _) => ["..www..", ".wwwww.", "wwwwwww", ".sssss."],
        (WeatherCondition::Drizzle, _) => [".www...", "wwwwww.", ".ssssss", "..r..r."],
        (WeatherCondition::Rain, _) => [".www...", "wwwwww.", ".ssssss", ".r.r.r."],
        (WeatherCondition::HeavyRain, _) => [".ddd...", "dddddd.", ".DDDDDD", "r.r.r.r"],
        (WeatherCondition::Thunderstorm, _) => [".ddd...", "dddddd.", ".DDbDDD", ".r.b.r."],
        (WeatherCondition::Snow, _) => [".www...", "wwwwww.", ".ssssss", ".n.n.n."],
        (WeatherCondition::Mist | WeatherCondition::Fog | WeatherCondition::Atmospheric, _) => {
            ["fffff..", "..fffff", "fffff..", "..fffff"]
        }
        (WeatherCondition::Unknown, _) => [".......", ".......", ".......", "......."],
    }
}

fn mini_color(code: u8) -> Option<(Rgb, Role)> {
    Some(match code {
        b'o' => (SUN_RIM, Role::Sun),
        b'y' => (SUN_CORE, Role::Sun),
        b'h' => (SUN_HIGHLIGHT, Role::Sun),
        b'm' => (MOON_LIGHT, Role::Moon),
        b'w' => (CLOUD_WHITE.0[1], Role::Cloud),
        b's' => (CLOUD_WHITE.0[3], Role::Shade),
        b'd' => (CLOUD_STORM.0[1], Role::Cloud),
        b'D' => (CLOUD_STORM.0[3], Role::Shade),
        b'r' => (RAIN, Role::Rain),
        b'b' => (BOLT, Role::Sun),
        b'n' => (SNOW, Role::Snow),
        b'f' => (FOG, Role::Fog),
        _ => return None,
    })
}

/// A small icon for one forecast interval: seven cells wide, two rows tall.
pub(crate) fn mini_icon(
    condition: WeatherCondition,
    cloud_cover_percent: Option<u8>,
    day_phase: Option<WeatherDayPhase>,
    palette: &Palette,
) -> Vec<Line<'static>> {
    let sprite = mini_sprite(
        fallback_condition(condition, cloud_cover_percent),
        day_phase == Some(WeatherDayPhase::Night),
    );
    let paint = |code: u8| {
        mini_color(code).map(|(color, role)| {
            super::light_cycle::mix(rgb(color), role.theme_color(palette), THEME_TINT)
        })
    };
    (0..2)
        .map(|row| {
            let top = sprite[row * 2].as_bytes();
            let bottom = sprite[row * 2 + 1].as_bytes();
            Line::from(
                top.iter()
                    .zip(bottom)
                    .map(|(upper, lower)| half_block(paint(*upper), paint(*lower)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn half_block(top: Option<Color>, bottom: Option<Color>) -> Span<'static> {
    match (top, bottom) {
        (None, None) => Span::raw(" "),
        (Some(top), None) => Span::styled("▀", Style::default().fg(top)),
        (None, Some(bottom)) => Span::styled("▄", Style::default().fg(bottom)),
        (Some(top), Some(bottom)) if top == bottom => Span::styled("█", Style::default().fg(top)),
        (Some(top), Some(bottom)) => Span::styled("▀", Style::default().fg(top).bg(bottom)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [WeatherCondition; 11] = [
        WeatherCondition::Clear,
        WeatherCondition::PartlyCloudy,
        WeatherCondition::Cloudy,
        WeatherCondition::Drizzle,
        WeatherCondition::Rain,
        WeatherCondition::HeavyRain,
        WeatherCondition::Thunderstorm,
        WeatherCondition::Snow,
        WeatherCondition::Mist,
        WeatherCondition::Fog,
        WeatherCondition::Atmospheric,
    ];

    fn pixels(canvas: &Canvas) -> Vec<Option<Rgb>> {
        canvas
            .pixels
            .iter()
            .flatten()
            .map(|pixel| pixel.map(|(color, _)| color))
            .collect()
    }

    fn count(canvas: &Canvas, color: Rgb) -> usize {
        pixels(canvas)
            .into_iter()
            .filter(|pixel| *pixel == Some(color))
            .count()
    }

    fn moon_sky(age: f64, southern: bool) -> Canvas {
        let mut canvas = Canvas::new();
        canvas.moon(16.0, 8.0, 6.0, MoonView { age, southern });
        canvas
    }

    fn lit(canvas: &Canvas) -> usize {
        pixels(canvas)
            .into_iter()
            .filter(|pixel| matches!(pixel, Some(color) if *color != MOON_DARK))
            .count()
    }

    fn lit_sides(canvas: &Canvas) -> (usize, usize) {
        let mut sides = (0, 0);
        for (y, row) in canvas.pixels.iter().enumerate() {
            let _ = y;
            for (x, pixel) in row.iter().enumerate() {
                if matches!(pixel, Some((color, _)) if *color != MOON_DARK) {
                    if x < 16 {
                        sides.0 += 1;
                    } else {
                        sides.1 += 1;
                    }
                }
            }
        }
        sides
    }

    #[test]
    fn the_moon_grows_and_shrinks_with_its_age() {
        let crescent = lit(&moon_sky(0.1, false));
        let quarter = lit(&moon_sky(0.25, false));
        let full = lit(&moon_sky(0.5, false));
        let waning = lit(&moon_sky(0.85, false));
        assert!(
            crescent < quarter && quarter < full,
            "{crescent} {quarter} {full}"
        );
        assert!(waning < full);
        assert_eq!(lit(&moon_sky(0.0, false)), 0);
        assert!(count(&moon_sky(0.0, false), MOON_DARK) > 50);
    }

    #[test]
    fn waxing_light_is_on_the_right_in_the_north_and_mirrored_in_the_south() {
        let north = lit_sides(&moon_sky(0.2, false));
        assert!(north.1 > north.0, "{north:?}");
        let south = lit_sides(&moon_sky(0.2, true));
        assert!(south.0 > south.1, "{south:?}");
        let waning = lit_sides(&moon_sky(0.8, false));
        assert!(waning.0 > waning.1, "{waning:?}");
    }

    #[test]
    fn every_condition_has_its_own_day_and_night_scene() {
        let moon = Some(MoonView {
            age: 0.3,
            southern: false,
        });
        let mut seen = Vec::new();
        for condition in ALL {
            for phase in [WeatherDayPhase::Day, WeatherDayPhase::Night] {
                let canvas = scene(condition, Some(50), phase, moon, 0);
                let art = pixels(&canvas);
                assert!(
                    art.iter().filter(|pixel| pixel.is_some()).count() > 40,
                    "{condition:?} {phase:?} is nearly empty"
                );
                seen.push((condition, phase, art));
            }
        }
        for (index, (condition, phase, art)) in seen.iter().enumerate() {
            for (other_condition, other_phase, other) in &seen[index + 1..] {
                let same_sky = matches!(
                    (condition, other_condition),
                    (WeatherCondition::Mist, WeatherCondition::Atmospheric)
                        | (WeatherCondition::Atmospheric, WeatherCondition::Mist)
                );
                if !same_sky {
                    assert_ne!(
                        art, other,
                        "{condition:?} {phase:?} vs {other_condition:?} {other_phase:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn scene_elements_match_the_condition() {
        let day = |condition| scene(condition, Some(50), WeatherDayPhase::Day, None, 0);
        assert!(count(&day(WeatherCondition::Clear), SUN_CORE) > 10);
        assert!(count(&day(WeatherCondition::Rain), RAIN) >= 5);
        assert!(
            count(&day(WeatherCondition::HeavyRain), RAIN)
                > count(&day(WeatherCondition::Drizzle), RAIN)
        );
        assert!(
            count(&day(WeatherCondition::Thunderstorm), BOLT)
                + count(&day(WeatherCondition::Thunderstorm), BOLT_EDGE)
                > 8
        );
        assert!(count(&day(WeatherCondition::Snow), SNOW) > 10);
        assert!(count(&day(WeatherCondition::Fog), FOG) > 30);
        assert_eq!(count(&day(WeatherCondition::Clear), RAIN), 0);
        let night = scene(
            WeatherCondition::Clear,
            Some(0),
            WeatherDayPhase::Night,
            Some(MoonView {
                age: 0.5,
                southern: false,
            }),
            0,
        );
        assert_eq!(count(&night, SUN_CORE), 0);
        assert!(count(&night, MOON_LIGHT) > 10);
    }

    #[test]
    fn the_sky_moves_between_frames() {
        let rain = |frame| {
            pixels(&scene(
                WeatherCondition::Rain,
                Some(90),
                WeatherDayPhase::Day,
                None,
                frame,
            ))
        };
        assert_ne!(rain(0), rain(1));
        let clear = |frame| {
            pixels(&scene(
                WeatherCondition::Clear,
                Some(0),
                WeatherDayPhase::Day,
                None,
                frame,
            ))
        };
        assert_ne!(clear(0), clear(1), "sun rays pulse");
        assert_eq!(clear(0), clear(2));
    }

    #[test]
    fn rendering_uses_half_blocks_within_the_scene_size() {
        let art = weather_art(
            WeatherCondition::PartlyCloudy,
            Some(40),
            WeatherDayPhase::Day,
            None,
            0,
            &crate::config::Theme::Amber.palette(),
        );
        assert_eq!(art.lines.len(), usize::from(ART_ROWS));
        for line in &art.lines {
            assert_eq!(line.width(), usize::from(ART_WIDTH));
            for span in &line.spans {
                assert!(
                    span.content
                        .chars()
                        .all(|c| matches!(c, ' ' | '▀' | '▄' | '█')),
                    "{:?}",
                    span.content
                );
            }
        }
        assert!(weather_art(
            WeatherCondition::Unknown,
            None,
            WeatherDayPhase::Day,
            None,
            0,
            &crate::config::Theme::Amber.palette(),
        )
        .lines
        .iter()
        .all(|line| line.spans.iter().all(|span| span.content == " ")));
    }

    #[test]
    fn art_leans_toward_each_theme_without_losing_its_own_colours() {
        let sun = |theme: crate::config::Theme| {
            let art = weather_art(
                WeatherCondition::Clear,
                Some(0),
                WeatherDayPhase::Day,
                None,
                0,
                &theme.palette(),
            );
            art.lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .find_map(|span| span.style.fg)
                .expect("the clear sky has a sun")
        };
        let amber = sun(crate::config::Theme::Amber);
        let green = sun(crate::config::Theme::HackerGreen);
        assert_ne!(amber, green, "the tint follows the theme");
        // The original warm sun still dominates the mix.
        if let Color::Rgb(red, _, blue) = green {
            assert!(red > blue, "{green:?}");
        }
    }

    #[test]
    fn every_condition_has_a_distinct_mini_icon() {
        let palette = crate::config::Theme::Amber.palette();
        let mut seen = Vec::new();
        for condition in ALL {
            for phase in [WeatherDayPhase::Day, WeatherDayPhase::Night] {
                let icon = mini_icon(condition, None, Some(phase), &palette);
                assert_eq!(icon.len(), 2);
                assert!(icon.iter().all(|line| line.width() == 7));
                let text: String = icon
                    .iter()
                    .flat_map(|line| line.spans.iter().map(|span| span.content.to_string()))
                    .collect();
                assert!(text.chars().any(|c| c != ' '), "{condition:?} {phase:?}");
                seen.push((condition, phase, format!("{icon:?}")));
            }
        }
        let distinct = |a: WeatherCondition, b: WeatherCondition| {
            let pick = |c| {
                seen.iter()
                    .find(|(cond, phase, _)| *cond == c && *phase == WeatherDayPhase::Day)
                    .map(|(_, _, icon)| icon.clone())
            };
            pick(a) != pick(b)
        };
        assert!(distinct(WeatherCondition::Clear, WeatherCondition::Cloudy));
        assert!(distinct(WeatherCondition::Rain, WeatherCondition::Snow));
        assert!(distinct(
            WeatherCondition::Rain,
            WeatherCondition::Thunderstorm
        ));
    }

    #[test]
    fn moon_view_reports_illumination_and_phase_names() {
        let full = MoonView {
            age: 0.5,
            southern: false,
        };
        assert!((full.illumination() - 1.0).abs() < 1e-9);
        assert_eq!(full.phase_name(), "Full moon");
        assert_eq!(
            MoonView {
                age: 0.1,
                southern: false
            }
            .phase_name(),
            "Waxing crescent"
        );
        assert_eq!(
            MoonView {
                age: 0.75,
                southern: true
            }
            .phase_name(),
            "Last quarter"
        );
    }
}
