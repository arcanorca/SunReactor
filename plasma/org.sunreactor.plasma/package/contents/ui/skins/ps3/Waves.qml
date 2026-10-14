/*
 * The XMB wave: the full-screen background of the PlayStation 3 popup, drawn by
 * a fragment shader (shaders/xmb.frag, compiled to shaders/xmb.frag.qsb by
 * tools/build_shaders.sh). It lies over the navy backdrop and under the content
 * (FullRepresentation loads it inside the popup Frame: Modules.OVERLAY).
 *
 * Motion: the picture is a loop of `period` seconds. A 33 ms timer advances it
 * (30 frames a second: the wave is slow, and 60 Hz would double the work for no
 * visible gain). A timer rather than a NumberAnimation is what caps the rate, as
 * a NumberAnimation follows the display's refresh. The timer runs only while the
 * popup is on screen and the desktop's animation speed is not "instant"; a hidden
 * popup costs nothing, and an instant desktop keeps the first frame still. The
 * shader is drawn at half the device resolution: the ribbons are soft, so the
 * upscale is invisible and a frame costs a quarter of the pixels.
 *
 * Calendar: the month sets the ribbons' tint (most months keep the palette's
 * blue; a null entry is the palette accent) and the hour dims the picture at
 * night. With no usable date the picture is the classic blue, by day.
 *
 * Fallback: until the shader has compiled, or where it cannot compile (no RHI, an
 * old driver), the ShaderEffect stays hidden and the backdrop gradient shows.
 *
 * Screenshot harness only (SUNREACTOR_HARNESS_CONFIG): motion_t freezes the loop
 * at a fraction 0 to 1, xmb_month (1 to 12) and xmb_hour (0 to 24) fix the
 * calendar, so frames compare.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasmoid

import "../../engine"

Item {
    id: root

    required property Theme theme

    clip: true

    /*! Seconds for one loop. shaders/xmb.frag has the same PERIOD. */
    readonly property real period: 40
    /*! The desktop's animation speed. Kirigami reports 1 ms, not 0, when it is
        "instant": that keeps the frame still. */
    readonly property bool motion: Kirigami.Units.longDuration > 1
    /*! Harness only: the frozen fraction of the loop from motion_t, or -1 for the live loop. */
    readonly property real frozenPhase: {
        const v = Number(Plasmoid.configuration.motion_t);
        return Number.isFinite(v) && v >= 0 && v < 1 ? v : -1;
    }
    /*! The popup is on screen: shown, and not hidden or minimised. */
    readonly property bool shown: visible && Window.window !== null
        && Window.visibility !== Window.Hidden && Window.visibility !== Window.Minimized
    readonly property bool playing: motion && frozenPhase < 0 && shown

    /*! Seconds into the loop: advanced by the frame timer while playing. */
    property real clockTime: 0
    readonly property real time: frozenPhase >= 0 ? frozenPhase * period : clockTime

    Timer {
        id: frames

        /*! 30 frames a second. */
        interval: 33
        repeat: true
        running: root.playing

        property double last: 0

        onRunningChanged: last = Date.now()
        onTriggered: {
            const now = Date.now();
            // A late tick moves the picture by the time it really took, at most 0.1 s.
            const dt = Math.min(0.1, Math.max(0, (now - last) / 1000));
            last = now;
            root.clockTime = (root.clockTime + dt) % root.period;
        }
    }

    /*! The local clock: read when the popup shows, then once a minute while it shows. */
    property var clock: new Date()
    onShownChanged: {
        if (shown) {
            clock = new Date();
        }
    }
    Timer {
        interval: 60000
        repeat: true
        running: root.shown
        onTriggered: root.clock = new Date()
    }

    /*! The month, 0 for January, or -1 when the date is unknown. Harness: xmb_month. */
    readonly property int month: {
        const m = Number(Plasmoid.configuration.xmb_month);
        if (Number.isInteger(m) && m >= 1 && m <= 12) {
            return m - 1;
        }
        return isNaN(root.clock.getTime()) ? -1 : root.clock.getMonth();
    }
    /*! The hour of the day (0 to 24, fractional), or -1 when the time is unknown. Harness: xmb_hour. */
    readonly property real hour: {
        const h = Number(Plasmoid.configuration.xmb_hour);
        if (Number.isFinite(h) && h >= 0 && h <= 24) {
            return h;
        }
        if (isNaN(root.clock.getTime())) {
            return -1;
        }
        return root.clock.getHours() + root.clock.getMinutes() / 60;
    }

    /*! 1 at night, 0 by day, easing through dusk (19 to 21 h) and dawn (5 to 7 h). */
    readonly property real nightFactor: {
        if (root.hour < 0) {
            return 0;
        }
        return Math.max(root.ease(19, 21, root.hour), 1 - root.ease(5, 7, root.hour));
    }

    function ease(a: real, b: real, x: real): real {
        const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
        return t * t * (3 - 2 * t);
    }

    /*! The ribbons' tint by month, January first: "#rrggbb", or null for the
        palette's accent (the classic blue). Most months keep the blue; the others
        lean a little towards the season, and keep the text's contrast. */
    readonly property var monthTints: [
        "#9dbcff",  // January: silver-blue
        "#7f8fff",  // February: periwinkle
        "#3fc4ea",  // March: aqua
        "#4aa8ff",  // April: sky
        null,       // May
        null,       // June
        "#2f7bff",  // July: royal
        null,       // August
        "#6a9fd6",  // September: steel
        "#7384ff",  // October: violet-blue
        "#3566d6",  // November: cobalt
        "#a6d8ff"   // December: ice
    ]

    /*! WCAG relative luminance (linear light) of a colour: the picture's text-contrast
        budget. The picture's 99th percentile stays at or under 0.12 (white text keeps
        about 6:1 against it). */
    function luminanceOf(c: color): real {
        const lin = v => v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
        return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
    }

    /*! The colour, scaled down (never up) to the luminance `budget`: a month's tint
        may be paler than the classic blue, never brighter over the text. */
    function withinBudget(c: color, budget: real): color {
        const s = Math.min(1, Math.pow(budget / Math.max(1e-6, root.luminanceOf(c)), 1 / 2.4));
        return Qt.rgba(c.r * s, c.g * s, c.b * s, 1);
    }

    /*! The ribbons' tint: the month's, within the budget of the palette's blue. */
    readonly property color ribbonTint: root.month >= 0 && root.monthTints[root.month] !== null
        ? root.withinBudget(Qt.color(root.monthTints[root.month]), root.luminanceOf(root.theme.accent))
        : root.theme.accent
    /*! The white-blue highlight of the ribbon cores: the palette's second accent, mixed in,
        within the budget of the classic blue's highlight. */
    readonly property color sheenTint: root.withinBudget(
        root.theme.mix(root.ribbonTint, root.theme.secondaryAccent, 0.5),
        root.luminanceOf(root.theme.mix(root.theme.accent, root.theme.secondaryAccent, 0.5)))
    /*! The top of the backdrop: the navy, lifted towards the ribbons' tint. */
    readonly property color deepTint: root.theme.mix(root.theme.bg, root.ribbonTint, 0.42)

    ShaderEffect {
        id: wave

        /*! The device pixel ratio of the window this is in (1 when it has none). */
        readonly property real dpr: Window.window ? Window.window.devicePixelRatio : 1

        anchors.fill: parent
        // Drawn only once compiled: until then, or where it cannot compile, the backdrop shows.
        visible: status === ShaderEffect.Compiled
        fragmentShader: Qt.resolvedUrl("shaders/xmb.frag.qsb")

        property real time: root.time
        property vector2d resolution: Qt.vector2d(width, height)
        property color colour: root.ribbonTint
        property color sheen: root.sheenTint
        property color deep: root.deepTint
        property real nightFactor: root.nightFactor

        layer.enabled: true
        layer.smooth: true
        layer.textureSize: Qt.size(Math.max(1, Math.round(width * dpr / 2)),
                                   Math.max(1, Math.round(height * dpr / 2)))
    }
}
