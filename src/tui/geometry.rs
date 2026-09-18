use ratatui::layout::Rect;

/// Geographic aspect ratio of an equirectangular world projection:
/// 360° longitude span / 180° latitude span = 2.0.
pub const WORLD_ASPECT: f64 = 2.0;

/// Conservative fallback terminal-cell aspect ratio representing common monospace character cell geometry (e.g. 9px × 20px ≈ 0.45).
pub const DEFAULT_CELL_ASPECT: f64 = 0.45;

/// Origin/confidence of detected terminal cell geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAspectSource {
    /// Derived from active OS terminal pixel metrics via `crossterm::terminal::window_size()`.
    TerminalPixels,
    /// Conservative fallback heuristic (~1:2 cell aspect).
    Fallback,
}

/// Holds the effective terminal cell aspect ratio (cell_pixel_width / cell_pixel_height)
/// and its derivation source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalCellMetrics {
    pub cell_aspect: f64,
    pub source: CellAspectSource,
}

impl Default for TerminalCellMetrics {
    fn default() -> Self {
        Self {
            cell_aspect: DEFAULT_CELL_ASPECT,
            source: CellAspectSource::Fallback,
        }
    }
}

impl TerminalCellMetrics {
    /// Constructs metrics from raw terminal dimensions and pixel counts.
    ///
    /// If any metric is zero, negative, NaN, or results in a non-physical cell aspect
    /// outside the plausible range [0.25, 1.5], falls back to `DEFAULT_CELL_ASPECT`.
    #[must_use]
    pub fn from_raw_dimensions(
        columns: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Self {
        if columns > 0 && rows > 0 && pixel_width > 0 && pixel_height > 0 {
            let cell_w = f64::from(pixel_width) / f64::from(columns);
            let cell_h = f64::from(pixel_height) / f64::from(rows);
            let aspect = cell_w / cell_h;
            // Reject non-physical aspect ratios outside [0.35, 0.65] (e.g. 1.0 when terminal returns char counts)
            if aspect.is_finite() && (0.35..=0.65).contains(&aspect) {
                return Self {
                    cell_aspect: aspect,
                    source: CellAspectSource::TerminalPixels,
                };
            }
        }
        Self::default()
    }
}

/// Probes terminal cell metrics from Crossterm if available, falling back safely.
#[must_use]
pub fn detect_terminal_cell_metrics() -> TerminalCellMetrics {
    #[cfg(feature = "tui")]
    {
        if let Ok(ws) = crossterm::terminal::window_size() {
            return TerminalCellMetrics::from_raw_dimensions(
                ws.columns, ws.rows, ws.width, ws.height,
            );
        }
    }
    TerminalCellMetrics::default()
}

/// Computes an aspect-correct, centered sub-rectangle within `available`
/// that preserves the physical 2:1 equirectangular world aspect ratio
/// given the terminal cell aspect ratio `cell_aspect` (cell_width / cell_height).
///
/// Returns a `Rect` satisfying:
/// 1. `viewport.width <= available.width` and `viewport.height <= available.height`
/// 2. `viewport` is centered within `available` (letterboxed vertically or pillarboxed horizontally)
/// 3. `(viewport.width / viewport.height) * cell_aspect ≈ WORLD_ASPECT` (within integer cell rounding)
#[must_use]
pub fn fit_world_map_viewport(available: Rect, cell_aspect: f64) -> Rect {
    if available.width == 0 || available.height == 0 {
        return Rect::new(available.x, available.y, 0, 0);
    }

    let aspect = if cell_aspect.is_finite() && (0.35..=0.65).contains(&cell_aspect) {
        cell_aspect
    } else {
        DEFAULT_CELL_ASPECT
    };

    // Desired terminal columns per row for an equirectangular world map (360° / 180° = 2.0).
    // desired_cell_ratio = WORLD_ASPECT / cell_aspect
    // e.g. for cell_aspect = 0.5 (8px × 16px), desired_cell_ratio = 4.0 (4 cols per row).
    let desired_cell_ratio = WORLD_ASPECT / aspect;
    let avail_w = f64::from(available.width);
    let avail_h = f64::from(available.height);
    let available_ratio = avail_w / avail_h;

    let (target_w, target_h) = if available_ratio > desired_cell_ratio {
        // Region is wider than desired: height is constraining factor, pillarbox left & right
        let h = available.height;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let w = (f64::from(h) * desired_cell_ratio).round() as u16;
        (w.clamp(1, available.width), h)
    } else if available_ratio < desired_cell_ratio {
        // Region is taller than desired: width is constraining factor, letterbox top & bottom
        let w = available.width;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let h = (f64::from(w) / desired_cell_ratio).round() as u16;
        (w, h.clamp(1, available.height))
    } else {
        // Exactly matched ratio
        (available.width, available.height)
    };

    let offset_x = available.width.saturating_sub(target_w) / 2;
    let offset_y = available.height.saturating_sub(target_h) / 2;

    Rect {
        x: available.x + offset_x,
        y: available.y + offset_y,
        width: target_w,
        height: target_h,
    }
}

/// Discrete semantic zoom level for the location context map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapZoomLevel {
    #[default]
    World,
    Continental,
    Regional,
}

// Enter thresholds are deliberately wider than their matching exit thresholds.
// A one-cell resize near an edge therefore keeps the prior semantic view
// instead of repeatedly switching between two map scales.
const WORLD_ENTER_MIN_WIDTH: u16 = 54;
const WORLD_ENTER_MIN_HEIGHT: u16 = 12;
const WORLD_EXIT_MIN_WIDTH: u16 = 50;
const WORLD_EXIT_MIN_HEIGHT: u16 = 11;
const CONTINENTAL_ENTER_MIN_WIDTH: u16 = 36;
const CONTINENTAL_ENTER_MIN_HEIGHT: u16 = 8;
const CONTINENTAL_EXIT_MIN_WIDTH: u16 = 32;
const CONTINENTAL_EXIT_MIN_HEIGHT: u16 = 7;

impl MapZoomLevel {
    /// Selects the optimal zoom level based on fitted canvas dimensions and optional prior level for hysteresis.
    #[must_use]
    pub fn select(fitted_w: u16, fitted_h: u16, current: Option<Self>) -> Self {
        let world_enter = fitted_w >= WORLD_ENTER_MIN_WIDTH && fitted_h >= WORLD_ENTER_MIN_HEIGHT;
        let world_exit = fitted_w >= WORLD_EXIT_MIN_WIDTH && fitted_h >= WORLD_EXIT_MIN_HEIGHT;
        let continental_enter =
            fitted_w >= CONTINENTAL_ENTER_MIN_WIDTH && fitted_h >= CONTINENTAL_ENTER_MIN_HEIGHT;
        let continental_exit =
            fitted_w >= CONTINENTAL_EXIT_MIN_WIDTH && fitted_h >= CONTINENTAL_EXIT_MIN_HEIGHT;

        match current {
            Some(Self::World) => {
                if world_exit {
                    Self::World
                } else if continental_enter {
                    Self::Continental
                } else {
                    Self::Regional
                }
            }
            Some(Self::Continental) => {
                if world_enter {
                    Self::World
                } else if continental_exit {
                    Self::Continental
                } else {
                    Self::Regional
                }
            }
            Some(Self::Regional) | None => {
                if world_enter {
                    Self::World
                } else if continental_enter {
                    Self::Continental
                } else {
                    Self::Regional
                }
            }
        }
    }

    /// Title label for the map frame.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::World => " Map · World ",
            Self::Continental => " Map · Continental ",
            Self::Regional => " Map · Regional ",
        }
    }

    /// Computes geographic ([min_lon, max_lon], [min_lat, max_lat]) bounds for this zoom level
    /// centered on (center_lon, center_lat) clamped to valid coordinate limits.
    ///
    /// The aspect ratio (span_lon / span_lat) is ALWAYS 2.0 (equirectangular).
    #[must_use]
    pub fn bounds(self, center_lon: f64, center_lat: f64) -> ([f64; 2], [f64; 2]) {
        match self {
            Self::World => ([-180.0, 180.0], [-90.0, 90.0]),
            Self::Continental => {
                // Span: 180° lon, 90° lat -> ratio = 2.0
                let min_lat = (center_lat - 45.0).clamp(-90.0, 0.0);
                let max_lat = min_lat + 90.0;
                let min_lon = (center_lon - 90.0).clamp(-180.0, 0.0);
                let max_lon = min_lon + 180.0;
                ([min_lon, max_lon], [min_lat, max_lat])
            }
            Self::Regional => {
                // Span: 90° lon, 45° lat -> ratio = 2.0
                let min_lat = (center_lat - 22.5).clamp(-90.0, 45.0);
                let max_lat = min_lat + 45.0;
                let min_lon = (center_lon - 45.0).clamp(-180.0, 90.0);
                let max_lon = min_lon + 90.0;
                ([min_lon, max_lon], [min_lat, max_lat])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::*;

    #[test]
    fn test_phase9_2_terminal_cell_metrics_detection_and_fallback() {
        // 1. Valid dimensions (80 cols, 24 rows, 640x384 pixels -> 8x16 px cell -> 0.5 aspect)
        let m1 = TerminalCellMetrics::from_raw_dimensions(80, 24, 640, 384);
        assert_eq!(m1.source, CellAspectSource::TerminalPixels);
        assert!((m1.cell_aspect - 0.5).abs() < 1e-6);

        // 2. Alternative valid font (80 cols, 25 rows, 720x375 pixels -> 9x15 px cell -> 0.6 aspect)
        let m2 = TerminalCellMetrics::from_raw_dimensions(80, 25, 720, 375);
        assert_eq!(m2.source, CellAspectSource::TerminalPixels);
        assert!((m2.cell_aspect - 0.6).abs() < 1e-6);

        // 3. Zero columns -> Fallback
        let m3 = TerminalCellMetrics::from_raw_dimensions(0, 24, 640, 384);
        assert_eq!(m3.source, CellAspectSource::Fallback);
        assert!((m3.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

        // 4. Zero rows -> Fallback
        let m4 = TerminalCellMetrics::from_raw_dimensions(80, 0, 640, 384);
        assert_eq!(m4.source, CellAspectSource::Fallback);
        assert!((m4.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

        // 5. Zero pixel width -> Fallback
        let m5 = TerminalCellMetrics::from_raw_dimensions(80, 24, 0, 384);
        assert_eq!(m5.source, CellAspectSource::Fallback);
        assert!((m5.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

        // 6. Zero pixel height -> Fallback
        let m6 = TerminalCellMetrics::from_raw_dimensions(80, 24, 640, 0);
        assert_eq!(m6.source, CellAspectSource::Fallback);
        assert!((m6.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

        // 7. Non-physical aspect < 0.25 -> Fallback
        let m7 = TerminalCellMetrics::from_raw_dimensions(80, 24, 100, 1000);
        assert_eq!(m7.source, CellAspectSource::Fallback);

        // 8. Non-physical aspect > 1.5 -> Fallback
        let m8 = TerminalCellMetrics::from_raw_dimensions(80, 24, 2000, 100);
        assert_eq!(m8.source, CellAspectSource::Fallback);

        // 9. OS detection returns a sane metric
        let detected = detect_terminal_cell_metrics();
        assert!(detected.cell_aspect.is_finite());
        assert!(detected.cell_aspect > 0.0);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_phase9_2_pure_viewport_geometry_fit_matrix() {
        // 1. Square-ish allocated region (50x50, cell aspect 0.5 -> desired cols/row = 4.0)
        // 50 / 50 = 1.0 < 4.0 -> Taller than desired -> constrain width (50), height = round(50/4) = 13.
        {
            let avail = Rect::new(10, 10, 50, 50);
            let fitted = fit_world_map_viewport(avail, 0.5);
            assert_eq!(fitted.width, 50);
            assert_eq!(fitted.height, 13);
            assert_eq!(fitted.x, 10);
            assert_eq!(fitted.y, 10 + (50 - 13) / 2); // centered vertically: 10 + 18 = 28
            let effective_aspect = (f64::from(fitted.width) / f64::from(fitted.height)) * 0.5;
            assert!((effective_aspect - WORLD_ASPECT).abs() < 0.1);
        }

        // 2. Very wide region (300x10, cell aspect 0.5 -> desired cols/row = 4.0)
        // 300 / 10 = 30.0 > 4.0 -> Wider than desired -> constrain height (10), width = round(10*4) = 40.
        {
            let avail = Rect::new(0, 0, 300, 10);
            let fitted = fit_world_map_viewport(avail, 0.5);
            assert_eq!(fitted.width, 40);
            assert_eq!(fitted.height, 10);
            assert_eq!(fitted.x, (300 - 40) / 2); // 130
            assert_eq!(fitted.y, 0);
            let effective_aspect = (f64::from(fitted.width) / f64::from(fitted.height)) * 0.5;
            assert!((effective_aspect - WORLD_ASPECT).abs() < 1e-6);
        }

        // 3. Very tall region (40x60, cell aspect 0.5 -> desired cols/row = 4.0)
        // 40 / 60 = 0.667 < 4.0 -> Taller than desired -> constrain width (40), height = round(40/4) = 10.
        {
            let avail = Rect::new(5, 5, 40, 60);
            let fitted = fit_world_map_viewport(avail, 0.5);
            assert_eq!(fitted.width, 40);
            assert_eq!(fitted.height, 10);
            assert_eq!(fitted.x, 5);
            assert_eq!(fitted.y, 5 + (60 - 10) / 2); // 5 + 25 = 30
            let effective_aspect = (f64::from(fitted.width) / f64::from(fitted.height)) * 0.5;
            assert!((effective_aspect - WORLD_ASPECT).abs() < 1e-6);
        }

        // 4. Exact target-ratio region (80x20, cell aspect 0.5 -> desired cols/row = 4.0)
        // 80 / 20 = 4.0 == 4.0 -> Exact match.
        {
            let avail = Rect::new(0, 0, 80, 20);
            let fitted = fit_world_map_viewport(avail, 0.5);
            assert_eq!(fitted.width, 80);
            assert_eq!(fitted.height, 20);
            assert_eq!(fitted.x, 0);
            assert_eq!(fitted.y, 0);
        }

        // 5. Odd-numbered dimensions (77x23, cell aspect 0.5 -> desired cols/row = 4.0)
        // 77 / 23 = 3.348 < 4.0 -> Constrain width (77), height = round(77/4) = 19.
        {
            let avail = Rect::new(2, 3, 77, 23);
            let fitted = fit_world_map_viewport(avail, 0.5);
            assert_eq!(fitted.width, 77);
            assert_eq!(fitted.height, 19);
            assert_eq!(fitted.x, 2);
            assert_eq!(fitted.y, 3 + (23 - 19) / 2); // 3 + 2 = 5
            assert!(fitted.x >= avail.x && fitted.x + fitted.width <= avail.x + avail.width);
            assert!(fitted.y >= avail.y && fitted.y + fitted.height <= avail.y + avail.height);
        }

        // 6. Tiny valid region (1x1)
        {
            let avail = Rect::new(0, 0, 1, 1);
            let fitted = fit_world_map_viewport(avail, 0.5);
            assert_eq!(fitted.width, 1);
            assert_eq!(fitted.height, 1);
            assert_eq!(fitted.x, 0);
            assert_eq!(fitted.y, 0);
        }

        // 7. Alternative cell aspect (0.6 -> desired cols/row = 2.0 / 0.6 = 3.3333)
        // 100 x 30: 100 / 30 = 3.3333 -> exact match
        {
            let avail = Rect::new(0, 0, 100, 30);
            let fitted = fit_world_map_viewport(avail, 0.6);
            assert_eq!(fitted.width, 100);
            assert_eq!(fitted.height, 30);
        }

        // 8. Missing / non-finite cell aspect falls back safely to DEFAULT_CELL_ASPECT
        {
            let avail = Rect::new(0, 0, 80, 30);
            let fitted_nan = fit_world_map_viewport(avail, f64::NAN);
            let fitted_fallback = fit_world_map_viewport(avail, DEFAULT_CELL_ASPECT);
            assert_eq!(fitted_nan, fitted_fallback);
        }
    }

    #[test]
    fn test_phase9_2_geographic_projection_normalized_invariance() {
        // Normalization formulas:
        // norm_x = (lon - (-180.0)) / 360.0
        // norm_y = (lat - (-90.0)) / 180.0
        let coords: [(&str, f64, f64, f64, f64); 5] = [
            ("Center", 0.0, 0.0, 0.5, 0.5),
            (
                "New York",
                40.7128,
                -74.0060,
                (-74.0060 + 180.0) / 360.0,
                (40.7128 + 90.0) / 180.0,
            ),
            (
                "Istanbul",
                41.01384,
                28.94966,
                (28.94966 + 180.0) / 360.0,
                (41.01384 + 90.0) / 180.0,
            ),
            (
                "Sydney",
                -33.8688,
                151.2093,
                (151.2093 + 180.0) / 360.0,
                (-33.8688 + 90.0) / 180.0,
            ),
            (
                "Tokyo",
                35.6762,
                139.6503,
                (139.6503 + 180.0) / 360.0,
                (35.6762 + 90.0) / 180.0,
            ),
        ];

        // Verify directional quadrant invariants
        for (name, _lat, _lon, nx, ny) in coords {
            match name {
                "Center" => {
                    assert!((nx - 0.5).abs() < 1e-4);
                    assert!((ny - 0.5).abs() < 1e-4);
                }
                "New York" => {
                    assert!(nx < 0.5, "New York must be west of Prime Meridian");
                    assert!(ny > 0.5, "New York must be north of Equator");
                }
                "Istanbul" | "Tokyo" => {
                    assert!(nx > 0.5, "{name} must be east of Prime Meridian");
                    assert!(ny > 0.5, "{name} must be north of Equator");
                }
                "Sydney" => {
                    assert!(nx > 0.5, "Sydney must be east of Prime Meridian");
                    assert!(ny < 0.5, "Sydney must be south of Equator");
                }
                _ => {}
            }
        }

        // Verify invariance of normalized projection across wide, normal, and tall viewports
        let viewports = [
            Rect::new(0, 0, 120, 20), // Wide
            Rect::new(0, 0, 85, 26),  // Normal
            Rect::new(0, 0, 50, 40),  // Tall
        ];

        for avail in viewports {
            let fitted = fit_world_map_viewport(avail, 0.5);
            assert!(fitted.width <= avail.width);
            assert!(fitted.height <= avail.height);

            // For each city, compute projected sub-cell coordinate inside fitted viewport
            for (_name, _lat, _lon, nx, ny) in coords {
                let px = f64::from(fitted.x) + nx * f64::from(fitted.width);
                let py = f64::from(fitted.y) + (1.0 - ny) * f64::from(fitted.height);
                assert!(px >= f64::from(fitted.x) && px <= f64::from(fitted.x + fitted.width));
                assert!(py >= f64::from(fitted.y) && py <= f64::from(fitted.y + fitted.height));
            }
        }
    }

    #[test]
    fn test_phase9_3_1_cell_aspect_validity_bounds() {
        // 1. Genuine terminal font dimensions (e.g. 9px x 20px -> 0.45)
        let m = TerminalCellMetrics::from_raw_dimensions(120, 30, 1080, 600);
        assert_eq!(m.source, CellAspectSource::TerminalPixels);
        assert!((m.cell_aspect - 0.45).abs() < 1e-4);

        // 2. Reject bogus 1:1 character-count reporting (columns=80, rows=24, width=80, height=24)
        let m_bogus = TerminalCellMetrics::from_raw_dimensions(80, 24, 80, 24);
        assert_eq!(m_bogus.source, CellAspectSource::Fallback);
        assert!((m_bogus.cell_aspect - DEFAULT_CELL_ASPECT).abs() < 1e-6);

        // 3. Reject extreme / non-physical aspects
        let m_zero = TerminalCellMetrics::from_raw_dimensions(80, 24, 0, 0);
        assert_eq!(m_zero.source, CellAspectSource::Fallback);

        let m_flat = TerminalCellMetrics::from_raw_dimensions(80, 24, 1600, 240); // 2.0
        assert_eq!(m_flat.source, CellAspectSource::Fallback);
    }

    #[test]
    fn test_phase9_4_adaptive_map_zoom_aspect_invariance() {
        // 1. Dimension-based selection
        assert_eq!(MapZoomLevel::select(70, 16, None), MapZoomLevel::World);
        assert_eq!(
            MapZoomLevel::select(45, 10, None),
            MapZoomLevel::Continental
        );
        assert_eq!(MapZoomLevel::select(30, 6, None), MapZoomLevel::Regional);

        // 2. Hysteresis holds the current semantic scale through one-cell resize jitter.
        assert_eq!(
            MapZoomLevel::select(51, 11, Some(MapZoomLevel::World)),
            MapZoomLevel::World
        );
        assert_eq!(
            MapZoomLevel::select(49, 11, Some(MapZoomLevel::World)),
            MapZoomLevel::Continental
        );
        assert_eq!(
            MapZoomLevel::select(33, 7, Some(MapZoomLevel::Continental)),
            MapZoomLevel::Continental
        );
        assert_eq!(
            MapZoomLevel::select(31, 7, Some(MapZoomLevel::Continental)),
            MapZoomLevel::Regional
        );

        // 3. Aspect Ratio Invariance (2:1 at ALL levels)
        for zoom in [
            MapZoomLevel::World,
            MapZoomLevel::Continental,
            MapZoomLevel::Regional,
        ] {
            let (x_bounds, y_bounds) = zoom.bounds(29.0, 41.0);
            let span_x = x_bounds[1] - x_bounds[0];
            let span_y = y_bounds[1] - y_bounds[0];
            let ratio = span_x / span_y;
            assert!(
                (ratio - 2.0).abs() < 1e-6,
                "Zoom {zoom:?} violated 2:1 aspect ratio: {ratio}"
            );

            // Clamping to valid world geography
            assert!(x_bounds[0] >= -180.0 && x_bounds[1] <= 180.0);
            assert!(y_bounds[0] >= -90.0 && y_bounds[1] <= 90.0);
        }

        // 4. Locations remain in bounds through dateline and polar clamping.
        let (x_east, y_north) = MapZoomLevel::Regional.bounds(179.0, 85.0);
        assert!((x_east[1] - x_east[0] - 90.0).abs() < 1e-6);
        assert!((y_north[1] - y_north[0] - 45.0).abs() < 1e-6);
        assert!(x_east[1] <= 180.0 && x_east[0] >= -180.0);
        assert!(y_north[1] <= 90.0 && y_north[0] >= -90.0);

        for (lon, lat) in [
            (28.9784, 41.0082),   // Istanbul
            (-74.0060, 40.7128),  // New York
            (151.2093, -33.8688), // Sydney
            (139.6503, 35.6762),  // Tokyo
            (-179.8, 70.0),       // International Date Line
        ] {
            for zoom in [MapZoomLevel::Continental, MapZoomLevel::Regional] {
                let (x, y) = zoom.bounds(lon, lat);
                assert!(
                    (x[0]..=x[1]).contains(&lon),
                    "{zoom:?} lost longitude {lon}"
                );
                assert!((y[0]..=y[1]).contains(&lat), "{zoom:?} lost latitude {lat}");
            }
        }
    }
}
