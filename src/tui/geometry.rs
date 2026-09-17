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
