.pragma library

/*
 * Sizing & scale calculator for SunReactor widget.
 * Computes automatic scaling based on screen resolution and physical dimensions,
 * ensuring text and interface elements are comfortably readable on any display:
 * from low-res laptops and standard 1080p desktop monitors to high-DPI QHD, 4K UHD,
 * and ultrawide screens.
 */

function calculateAutoScale(screen) {
    if (!screen) {
        return 1.0;
    }

    const w = screen.width > 0 ? screen.width : 1920;
    const h = screen.height > 0 ? screen.height : 1080;
    const dpr = screen.devicePixelRatio > 0 ? screen.devicePixelRatio : 1.0;
    const availH = screen.desktopAvailableHeight > 0 ? screen.desktopAvailableHeight : h;

    const physW = Math.round(w * dpr);
    const physH = Math.round(h * dpr);

    // Screen.pixelDensity in Qt Quick is in logical pixels per millimeter (DIPs/mm).
    const density = screen.pixelDensity > 0 ? screen.pixelDensity : 0;
    let diagInches = 0;
    let physDpi = 0;
    if (density > 0) {
        // Physical dimensions in mm: w (DIPs) / density (DIPs/mm)
        const widthMm = w / density;
        const heightMm = h / density;
        diagInches = Math.sqrt(widthMm * widthMm + heightMm * heightMm) / 25.4;
        physDpi = density * dpr * 25.4;
    }

    let scale = 1.0;

    // Displays with low vertical resolution (< 900 physical px) have insufficient height for upscale
    if (physH < 900) {
        return 1.0;
    }

    // 1. Screens with physical dimension data (distinguishing laptops vs desktop displays)
    if (diagInches >= 10 && diagInches <= 18) {
        // Laptop screen (closer viewing distance, high physical pixel density)
        if (physDpi >= 200) {
            scale = dpr >= 1.75 ? 1.20 : 1.35;
        } else if (physDpi >= 150) {
            scale = dpr >= 1.5 ? 1.15 : 1.25;
        } else if (physDpi >= 125) {
            scale = 1.20;
        } else {
            scale = 1.15;
        }
    } else if (physDpi >= 140) {
        // Desktop high-DPI monitor (e.g. 24" 4K, 27" 4K/5K)
        scale = dpr >= 1.75 ? 1.20 : (dpr >= 1.4 ? 1.30 : 1.45);
    } else if (physDpi >= 105) {
        // Desktop QHD monitor (e.g. 27" 1440p ~109 DPI)
        scale = dpr >= 1.25 ? 1.15 : 1.25;
    }
    // 2. Resolution-based fallback (when EDID density is missing or <= 0)
    else if (physH >= 2000) {
        // 4K UHD (3840x2160, 4096x2160, etc.)
        scale = dpr >= 1.75 ? 1.20 : (dpr >= 1.4 ? 1.35 : 1.50);
    } else if (physH >= 1350) {
        // 1440p / 1600p QHD
        scale = dpr >= 1.25 ? 1.15 : 1.25;
    } else if (physH >= 950) {
        // 1080p / 1200p FHD (e.g. 1920x1080, 1920x1200)
        scale = dpr >= 1.25 ? 1.10 : 1.15;
    } else {
        // Lower resolution (< 900p, 720p)
        scale = 1.0;
    }

    // Safety guard: ensure typical popup content height (24 grid units) doesn't exceed 85% of available screen height
    if (availH > 0) {
        const referenceGu = 24;
        const estimatedH = referenceGu * 18 * scale;
        if (estimatedH > availH * 0.85) {
            scale = Math.max(1.0, (availH * 0.85) / (referenceGu * 18));
        }
    }

    // Clamp to range [1.0, 2.0] and round to 2 decimal places
    return Math.round(Math.min(2.0, Math.max(1.0, scale)) * 100) / 100;
}

function resolveScale(uiScale, screen) {
    const scaleNum = Number(uiScale);
    if (!isNaN(scaleNum) && scaleNum > 0) {
        return Math.max(0.5, Math.min(3.0, Math.round(scaleNum) / 100.0));
    }
    return calculateAutoScale(screen);
}
