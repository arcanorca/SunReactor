//! Pure geometry for the Location globe: an orthographic Earth centred on the
//! configured location, filled from a small public-domain land mask, and lit
//! by the same solar model the policy engine uses.

use std::f64::consts::PI;
use std::sync::OnceLock;

use ratatui::layout::Rect;

use super::{
    geometry::DEFAULT_CELL_ASPECT,
    land_mask::{LAND_MASK_COLUMNS, LAND_MASK_ROWS, LAND_MASK_ROW_OFFSETS, LAND_MASK_RUNS},
    motion::UiMotionState,
};

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GlobeCenter {
    pub lon_deg: f64,
    pub lat_deg: f64,
}

impl GlobeCenter {
    #[must_use]
    pub(crate) fn new(lon_deg: f64, lat_deg: f64) -> Self {
        Self {
            lon_deg: normalize_longitude(lon_deg),
            lat_deg: lat_deg.clamp(-90.0, 90.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GlobeProjection {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Wraps a longitude into the conventional `[-180, 180)` interval.
#[must_use]
pub(crate) fn normalize_longitude(longitude_deg: f64) -> f64 {
    let wrapped = (longitude_deg + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped.abs() < EPSILON {
        0.0
    } else {
        wrapped
    }
}

/// Projects a geographic point onto the visible orthographic hemisphere.
///
/// `z` is the front-facing component; points behind the sphere are culled.
#[must_use]
pub(crate) fn project_orthographic(
    point_lon_deg: f64,
    point_lat_deg: f64,
    center: GlobeCenter,
) -> Option<GlobeProjection> {
    let point_lat = point_lat_deg.clamp(-90.0, 90.0).to_radians();
    let center_lat = center.lat_deg.to_radians();
    let delta_lon = normalize_longitude(point_lon_deg - center.lon_deg).to_radians();

    let x = point_lat.cos() * delta_lon.sin();
    let y =
        center_lat.cos() * point_lat.sin() - center_lat.sin() * point_lat.cos() * delta_lon.cos();
    let z =
        center_lat.sin() * point_lat.sin() + center_lat.cos() * point_lat.cos() * delta_lon.cos();

    (z >= -EPSILON).then_some(GlobeProjection { x, y, z })
}

/// Inverse of [`project_orthographic`] for a point on the visible disk.
/// Returns `(lon_deg, lat_deg, z)`.
#[must_use]
pub(crate) fn unproject_orthographic(
    x: f64,
    y: f64,
    center: GlobeCenter,
) -> Option<(f64, f64, f64)> {
    let r2 = x * x + y * y;
    if r2 > 1.0 {
        return None;
    }
    let z = (1.0 - r2).sqrt();
    let center_lat = center.lat_deg.to_radians();
    let lat = (z * center_lat.sin() + y * center_lat.cos())
        .clamp(-1.0, 1.0)
        .asin();
    let lon = center.lon_deg.to_radians() + x.atan2(z * center_lat.cos() - y * center_lat.sin());
    Some((normalize_longitude(lon.to_degrees()), lat.to_degrees(), z))
}

fn land_bits() -> &'static [u64] {
    static BITS: OnceLock<Vec<u64>> = OnceLock::new();
    BITS.get_or_init(|| {
        let mut bits = vec![0u64; (LAND_MASK_COLUMNS * LAND_MASK_ROWS).div_ceil(64)];
        for row in 0..LAND_MASK_ROWS {
            let runs = &LAND_MASK_RUNS[usize::from(LAND_MASK_ROW_OFFSETS[row])
                ..usize::from(LAND_MASK_ROW_OFFSETS[row + 1])];
            let mut column = 0usize;
            for (index, run) in runs.iter().enumerate() {
                let run = usize::from(*run);
                if index % 2 == 1 {
                    for x in column..(column + run).min(LAND_MASK_COLUMNS) {
                        let bit = row * LAND_MASK_COLUMNS + x;
                        bits[bit / 64] |= 1 << (bit % 64);
                    }
                }
                column += run;
            }
        }
        bits
    })
}

/// Whether a geographic point is land in the 0.5° Natural Earth mask.
#[must_use]
pub(crate) fn is_land(lon_deg: f64, lat_deg: f64) -> bool {
    let column =
        (((normalize_longitude(lon_deg) + 180.0) * 2.0) as usize).min(LAND_MASK_COLUMNS - 1);
    let row = (((90.0 - lat_deg.clamp(-90.0, 90.0)) * 2.0) as usize).min(LAND_MASK_ROWS - 1);
    let bit = row * LAND_MASK_COLUMNS + column;
    land_bits()[bit / 64] & (1 << (bit % 64)) != 0
}

/// Illumination of a surface point relative to the sun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Daylight {
    Day,
    /// The sun is within 6° below the horizon (civil twilight).
    Twilight,
    Night,
}

/// Classifies a point from the subsolar point. The sine of the solar
/// elevation equals the dot product of the two surface unit vectors.
#[must_use]
pub(crate) fn daylight_at(lon_deg: f64, lat_deg: f64, subsolar: (f64, f64)) -> Daylight {
    let (sun_lat, sun_lon) = subsolar;
    let vector = |lat: f64, lon: f64| {
        let (lat, lon) = (lat.to_radians(), lon.to_radians());
        (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin())
    };
    let p = vector(lat_deg, lon_deg);
    let s = vector(sun_lat, sun_lon);
    let sin_elevation = p.0 * s.0 + p.1 * s.1 + p.2 * s.2;
    if sin_elevation > 0.0 {
        Daylight::Day
    } else if sin_elevation > (-6.0f64).to_radians().sin() {
        Daylight::Twilight
    } else {
        Daylight::Night
    }
}

/// Fits a centred, physically circular globe into a terminal rectangle.
///
/// Braille canvases map 2×4 dots onto each cell, so equal logical spans are
/// circular when `(columns / rows) × cell_aspect ≈ 1`.
#[must_use]
pub(crate) fn fit_globe_viewport(available: Rect, cell_aspect: f64) -> Rect {
    if available.width == 0 || available.height == 0 {
        return Rect::new(available.x, available.y, 0, 0);
    }

    let aspect = if cell_aspect.is_finite() && (0.25..=1.5).contains(&cell_aspect) {
        cell_aspect
    } else {
        DEFAULT_CELL_ASPECT
    };
    let desired_cell_ratio = 1.0 / aspect;
    let available_ratio = f64::from(available.width) / f64::from(available.height);
    let (width, height) = if available_ratio >= desired_cell_ratio {
        let height = available.height;
        let width = (f64::from(height) * desired_cell_ratio).round() as u16;
        (width.clamp(1, available.width), height)
    } else {
        let width = available.width;
        let height = (f64::from(width) / desired_cell_ratio).round() as u16;
        (width, height.clamp(1, available.height))
    };

    Rect::new(
        available.x + available.width.saturating_sub(width) / 2,
        available.y + available.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

/// Interpolates the shortest spherical path between two globe viewpoints.
#[must_use]
pub(crate) fn interpolate_center(from: GlobeCenter, to: GlobeCenter, phase: f32) -> GlobeCenter {
    let phase = f64::from(phase.clamp(0.0, 1.0));
    let eased = phase * phase * (3.0 - 2.0 * phase);
    let from_vector = center_vector(from);
    let to_vector = center_vector(to);
    let dot =
        (from_vector.0 * to_vector.0 + from_vector.1 * to_vector.1 + from_vector.2 * to_vector.2)
            .clamp(-1.0, 1.0);

    let vector = if dot > 0.9995 {
        normalize_vector(
            from_vector.0 + (to_vector.0 - from_vector.0) * eased,
            from_vector.1 + (to_vector.1 - from_vector.1) * eased,
            from_vector.2 + (to_vector.2 - from_vector.2) * eased,
        )
    } else {
        let angle = dot.acos();
        let sin_angle = angle.sin();
        let from_weight = ((1.0 - eased) * angle).sin() / sin_angle;
        let to_weight = (eased * angle).sin() / sin_angle;
        normalize_vector(
            from_vector.0 * from_weight + to_vector.0 * to_weight,
            from_vector.1 * from_weight + to_vector.1 * to_weight,
            from_vector.2 * from_weight + to_vector.2 * to_weight,
        )
    };

    GlobeCenter::new(
        vector.1.atan2(vector.0).to_degrees(),
        vector.2.asin().to_degrees(),
    )
}

/// The animated viewpoint during a Full-effects rotation, otherwise the
/// configured location.
#[must_use]
pub(crate) fn center_for_motion(
    motion: &UiMotionState,
    target: GlobeCenter,
    now: std::time::Instant,
) -> GlobeCenter {
    motion.globe_rotation_phase(now).map_or(
        target,
        |(phase, from_lon, from_lat, to_lon, to_lat)| {
            interpolate_center(
                GlobeCenter::new(from_lon, from_lat),
                GlobeCenter::new(to_lon, to_lat),
                phase,
            )
        },
    )
}

/// Duration scales with angular distance within a bounded budget.
#[must_use]
pub(crate) fn rotation_duration(from: GlobeCenter, to: GlobeCenter) -> std::time::Duration {
    let (from_x, from_y, from_z) = center_vector(from);
    let (to_x, to_y, to_z) = center_vector(to);
    let dot = (from_x * to_x + from_y * to_y + from_z * to_z).clamp(-1.0, 1.0);
    let millis = 450.0 + (dot.acos() / PI) * 450.0;
    std::time::Duration::from_millis(millis.round() as u64)
}

fn center_vector(center: GlobeCenter) -> (f64, f64, f64) {
    let lat = center.lat_deg.to_radians();
    let lon = center.lon_deg.to_radians();
    (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin())
}

fn normalize_vector(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let length = (x * x + y * y + z * z).sqrt();
    if length <= EPSILON {
        (1.0, 0.0, 0.0)
    } else {
        (x / length, y / length, z / length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_and_antipode_visibility_are_exact() {
        let center = GlobeCenter::new(28.9784, 41.0082);
        let projected = project_orthographic(center.lon_deg, center.lat_deg, center)
            .expect("center is visible");
        assert!(projected.x.abs() < 1e-9);
        assert!(projected.y.abs() < 1e-9);
        assert!((projected.z - 1.0).abs() < 1e-9);

        assert!(project_orthographic(center.lon_deg + 180.0, -center.lat_deg, center).is_none());
    }

    #[test]
    fn horizon_and_longitude_wrapping_are_handled() {
        let center = GlobeCenter::new(179.0, 0.0);
        let horizon = project_orthographic(269.0, 0.0, center).expect("horizon is retained");
        assert!(horizon.z.abs() < 1e-8);
        let wrapped = project_orthographic(-91.0, 0.0, center).expect("wrapped point visible");
        assert!((wrapped.x - horizon.x).abs() < 1e-8);
    }

    #[test]
    fn known_locations_project_to_their_own_centers() {
        for (longitude, latitude) in [
            (28.9784, 41.0082),
            (139.6917, 35.6895),
            (151.2093, -33.8688),
            (-74.0060, 40.7128),
            (-157.8583, 21.3069),
            (179.9, 66.0),
        ] {
            let center = GlobeCenter::new(longitude, latitude);
            let projected = project_orthographic(longitude, latitude, center)
                .expect("viewpoint should be visible");
            assert!(projected.x.abs() < 1e-8);
            assert!(projected.y.abs() < 1e-8);
        }
    }

    #[test]
    fn inverse_projection_round_trips() {
        let center = GlobeCenter::new(-74.0, 40.7);
        for (lon, lat) in [(-74.0, 40.7), (-10.0, 20.0), (-120.0, 60.0), (-60.0, -30.0)] {
            let p = project_orthographic(lon, lat, center).expect("visible");
            let (back_lon, back_lat, z) =
                unproject_orthographic(p.x, p.y, center).expect("on disk");
            assert!((back_lon - lon).abs() < 1e-6, "{back_lon} vs {lon}");
            assert!((back_lat - lat).abs() < 1e-6);
            assert!((z - p.z).abs() < 1e-6);
        }
        assert!(unproject_orthographic(0.9, 0.9, center).is_none());
    }

    #[test]
    fn land_mask_matches_well_known_places() {
        for (lon, lat, land) in [
            (28.97, 41.01, true),  // Istanbul
            (139.69, 35.69, true), // Tokyo
            (-74.0, 40.75, true),  // New York
            (133.0, -25.0, true),  // central Australia
            (-30.0, 30.0, false),  // North Atlantic
            (-150.0, 0.0, false),  // Pacific
            (80.0, -30.0, false),  // Indian Ocean
            (20.0, 0.0, true),     // Congo basin
        ] {
            assert_eq!(is_land(lon, lat), land, "({lon}, {lat})");
        }
    }

    #[test]
    fn daylight_uses_the_subsolar_point() {
        let subsolar = (10.0, 30.0);
        assert_eq!(daylight_at(30.0, 10.0, subsolar), Daylight::Day);
        assert_eq!(daylight_at(-150.0, -10.0, subsolar), Daylight::Night);
        assert_eq!(
            daylight_at(120.0 + 3.0, 0.0, (0.0, 30.0)),
            Daylight::Twilight
        );
    }

    #[test]
    fn globe_fitter_preserves_braille_physical_circularity() {
        for (available, aspect) in [
            (Rect::new(0, 0, 120, 40), 0.5),
            (Rect::new(0, 0, 70, 50), 0.6),
            (Rect::new(0, 0, 48, 30), 1.0),
        ] {
            let fitted = fit_globe_viewport(available, aspect);
            let physical_ratio = (f64::from(fitted.width) / f64::from(fitted.height)) * aspect;
            assert!((physical_ratio - 1.0).abs() < 0.12);
            assert!(fitted.width <= available.width);
            assert!(fitted.height <= available.height);
        }
    }

    #[test]
    fn rotation_duration_is_bounded_and_center_interpolation_settles() {
        let from = GlobeCenter::new(0.0, 0.0);
        let to = GlobeCenter::new(179.0, 10.0);
        let duration = rotation_duration(from, to);
        assert!((450..=900).contains(&duration.as_millis()));
        assert_eq!(interpolate_center(from, to, 0.0), from);
        let settled = interpolate_center(from, to, 1.0);
        assert!((settled.lon_deg - to.lon_deg).abs() < 1e-8);
        assert!((settled.lat_deg - to.lat_deg).abs() < 1e-8);
    }
}
