use ratatui::layout::Rect;

use crate::tui::globe::{
    daylight_at, is_land, project_orthographic, unproject_orthographic, Daylight, GlobeCenter,
};

pub(super) struct GlobeLayers {
    pub(super) graticule: Vec<(f64, f64)>,
    pub(super) land_day: Vec<(f64, f64)>,
    pub(super) land_limb: Vec<(f64, f64)>,
    pub(super) land_twilight: Vec<(f64, f64)>,
    pub(super) land_night: Vec<(f64, f64)>,
}

/// Samples every Braille dot of the viewport once: land or ocean, lit or not,
/// and near the limb or facing the viewer.
pub(super) fn shade_globe(
    viewport: Rect,
    center: GlobeCenter,
    subsolar: (f64, f64),
) -> GlobeLayers {
    let dots_x = usize::from(viewport.width) * 2;
    let dots_y = usize::from(viewport.height) * 4;
    let span = 2.04;
    let mut layers = GlobeLayers {
        graticule: graticule(center, dots_x.max(dots_y)),
        land_day: Vec::new(),
        land_limb: Vec::new(),
        land_twilight: Vec::new(),
        land_night: Vec::new(),
    };
    for row in 0..dots_y {
        let y = 1.02 - (row as f64 + 0.5) * span / dots_y as f64;
        for column in 0..dots_x {
            let x = -1.02 + (column as f64 + 0.5) * span / dots_x as f64;
            let Some((lon, lat, z)) = unproject_orthographic(x, y, center) else {
                continue;
            };
            let light = daylight_at(lon, lat, subsolar);
            if is_land(lon, lat) {
                match light {
                    Daylight::Day if z < 0.28 => layers.land_limb.push((x, y)),
                    Daylight::Day => layers.land_day.push((x, y)),
                    Daylight::Twilight => layers.land_twilight.push((x, y)),
                    Daylight::Night => layers.land_night.push((x, y)),
                }
            }
        }
    }
    layers
}

/// Dotted 30° meridians and parallels. Their curvature is what makes the
/// orthographic disk read as a sphere.
fn graticule(center: GlobeCenter, resolution: usize) -> Vec<(f64, f64)> {
    let steps = (resolution / 3).clamp(24, 90);
    let mut points = Vec::new();
    let mut push = |lon: f64, lat: f64| {
        if let Some(point) = project_orthographic(lon, lat, center) {
            if point.z > 0.05 {
                points.push((point.x, point.y));
            }
        }
    };
    for latitude in [-60.0, -30.0, 0.0, 30.0, 60.0] {
        for step in 0..steps * 2 {
            push(-180.0 + 360.0 * step as f64 / (steps * 2) as f64, latitude);
        }
    }
    for meridian in 0..12 {
        let longitude = -180.0 + 30.0 * f64::from(meridian);
        for step in 0..=steps {
            push(longitude, -90.0 + 180.0 * step as f64 / steps as f64);
        }
    }
    points
}
