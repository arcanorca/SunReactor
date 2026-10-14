/*
 * Light cycle, synthwave84: the hero, a retro-futurist poster.
 *
 *   DAYLIGHT ───────────────────────────────────────────
 *   NOW                                    Weather −3%
 *   47%  (pink to cyan, neon bloom)        Next 4:20 PM · Set 50% → 23%
 *   ┌───────── sky ─────────┐
 *   │        ◓ striped sun  │   the cyan day curve; the sun's height is the
 *   │  ~~~~~~~~~~~~~~~~~~~~ │   target now (it rises with brightness); the dot
 *   ════════════════════════   on the curve is the same point, at this time.
 *   ╱ ╲ ╱ ╲  perspective grid, lines converging on the horizon
 *   00      06      12      18      24
 *
 * Each display's curve is shown; chips choose the display when there are
 * several. Hovering scrubs the day: the hero reads that time's target. No
 * timers and no animation: the picture changes only when the data does.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Effects
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod
import "../../Format.js" as Format
import "../instrument/Curve.js" as Curve
import "../instrument" as Instrument

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }

    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0
        && curves[0].samples.length > 1

    /*! Index into `curves` of the display on the chart. */
    property int selected: 0
    readonly property var current: curves.length > 0
        ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property var samples: current ? current.samples : []
    readonly property real t0: available ? samples[0].t : 0
    readonly property real t1: available ? samples[samples.length - 1].t : 1
    readonly property real now: client.nowEpochS

    /*! Linear interpolation of a sample field at time t; -1 outside the day. */
    function valueAt(points, t, field) {
        if (!points || points.length === 0 || t < points[0].t || t > points[points.length - 1].t) {
            return -1;
        }
        for (let i = 1; i < points.length; ++i) {
            const b = points[i];
            if (b.t >= t) {
                const a = points[i - 1];
                const f = b.t === a.t ? 0 : (t - a.t) / (b.t - a.t);
                return a[field] + (b[field] - a[field]) * f;
            }
        }
        return points[points.length - 1][field];
    }

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    readonly property real targetNow: available ? valueAt(samples, now, "pct") : -1
    readonly property real costNow: targetNow >= 0
        ? valueAt(samples, now, "pctNoWeather") - targetNow : 0

    readonly property var milestones: current && current.milestones ? current.milestones : []
    readonly property var nextMilestone: {
        for (const milestone of milestones) {
            if (milestone.t > now) {
                return milestone;
            }
        }
        return null;
    }
    /*! "Next 4:20 PM · Set 50% → 23%": the daemon's milestone and its target. */
    readonly property string nextText: {
        const m = nextMilestone;
        if (!m) {
            return "";
        }
        let step = m.label;
        if (m.pct >= 0) {
            step = i18ndc(domain, "Milestone name, then the brightness it targets", "%1 → %2%", m.label, m.pct);
        }
        return i18ndc(domain, "Time of the next step, then what happens then", "Next %1 · %2",
                      Format.timeText(m.t), step);
    }

    spacing: style.rowGap

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && targetNow >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now",
                 "%1: %2% now", current.monitorId, Math.round(targetNow))
        : ""

    // --- the sun, the grid and the curve: geometry and paths ------------------------

    /*! The sun's cut-outs: [start, thickness] as fractions of its diameter, from
        the top. They grow thicker towards the bottom, like the outrun sun. */
    readonly property var bands: [[0.52, 0.010], [0.62, 0.020], [0.72, 0.032], [0.82, 0.044], [0.92, 0.054]]

    /*! One slice of the disc between two heights (local px, disc centred on (r, r)),
        as a closed polygon sampled along the circle. */
    function discSlice(r, yA, yB) {
        const a = Math.asin(Math.max(-1, Math.min(1, (yA - r) / r)));
        const b = Math.asin(Math.max(-1, Math.min(1, (yB - r) / r)));
        const n = 28;
        let d = "";
        for (let i = 0; i <= n; ++i) {
            const f = a + (b - a) * i / n;
            d += (i === 0 ? "M " : "L ") + (r + r * Math.cos(f)).toFixed(2) + " " + (r + r * Math.sin(f)).toFixed(2) + " ";
        }
        for (let i = 0; i <= n; ++i) {
            const f = b - (b - a) * i / n;
            d += "L " + (r - r * Math.cos(f)).toFixed(2) + " " + (r + r * Math.sin(f)).toFixed(2) + " ";
        }
        return d + "Z ";
    }

    /*! The striped sun: the disc minus the bands, as one path of slices. */
    function sunPath(r) {
        const d = 2 * r;
        const edges = [0];
        for (const band of bands) {
            edges.push(band[0] * d, (band[0] + band[1]) * d);
        }
        edges.push(d);
        let path = "";
        for (let i = 0; i + 1 < edges.length; i += 2) {
            path += discSlice(r, edges[i], edges[i + 1]);
        }
        return path;
    }

    /*! Horizontal lines of the floor, closer together towards the horizon. */
    function floorLines(w, h) {
        const n = 9;
        let d = "";
        for (let k = 1; k <= n; ++k) {
            const y = (h * Math.pow(k / n, 2)).toFixed(2);
            d += "M 0 " + y + " L " + w.toFixed(2) + " " + y + " ";
        }
        return d;
    }

    /*! Lines from the vanishing point on the horizon down to the floor. */
    function rayLines(w, h) {
        const vx = w / 2;
        const n = 9;
        let d = "";
        for (let j = -n; j <= n; ++j) {
            d += "M " + vx.toFixed(2) + " 0 L " + (vx + j * w / 6).toFixed(2) + " " + h.toFixed(2) + " ";
        }
        return d;
    }

    // --- heading and the readout ----------------------------------------------------

    // The heading (as Heading.qml draws it), with the display chips at its right
    // end when there are several: they choose the display on the chart.
    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap2

        Text {
            Layout.alignment: Qt.AlignVCenter
            text: i18ndc(root.domain, "Synthwave heading of the brightness curve", "Daylight")
            textFormat: Text.PlainText
            font: root.style.headingFont
            color: root.style.pink
            // Shear about the centre, as the other headings.
            transformOrigin: Item.Center
            transform: Matrix4x4 {
                matrix: Qt.matrix4x4(1, -root.style.shear, 0, 0,
                                     0, 1, 0, 0,
                                     0, 0, 1, 0,
                                     0, 0, 0, 1)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignVCenter
            implicitHeight: root.style.hairline
            color: root.style.alpha(root.style.cyan, 0.45)
        }

    }

    /*! The big figure: pink to cyan, through the glyphs' alpha, with a neon bloom.
        Static: it is redrawn only when its text changes. */
    component HeroNumber: Item {
        id: hero

        required property string text

        implicitWidth: glyphs.implicitWidth
        implicitHeight: glyphs.implicitHeight

        // The bloom of the filled figure, underneath it.
        Instrument.Glow {
            anchors.fill: fill
            source: fill
            strength: root.style.glow
            tint: root.style.pink
        }

        MultiEffect {
            id: fill

            anchors.fill: parent
            source: paint
            maskEnabled: true
            maskSource: glyphs
        }

        // The gradient, and the glyphs as its mask. Both are hidden items with a
        // layer: the effect draws them, nothing draws them twice.
        Rectangle {
            id: paint

            visible: false
            layer.enabled: true
            width: hero.width
            height: hero.height
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: root.style.pink }
                GradientStop { position: 1; color: root.style.cyan }
            }
        }
        Text {
            id: glyphs

            visible: false
            layer.enabled: true
            width: hero.width
            height: hero.height
            text: hero.text
            textFormat: Text.PlainText
            font: root.style.heroFont
            color: "#ffffff"
            verticalAlignment: Text.AlignVCenter
        }
    }

    // --- the chart --------------------------------------------------------------------

    Item {
        id: plot

        Layout.fillWidth: true
        implicitHeight: root.style.snap(root.style.unit * 5.9)
        visible: root.available

        /*! Where the horizon is, and the floor below it. */
        readonly property real horizonY: Math.round(height * 0.6)
        readonly property real groundH: height - horizonY
        /*! The sun's radius: it fills most of the sky. */
        readonly property real sunR: Math.max(6, Math.min(width * 0.2, horizonY * 0.42))
        /*! Target 100 % sits here; 0 % sits just above the horizon. Sun and curve share it. */
        readonly property real topY: sunR + horizonY * 0.04
        readonly property real lowY: horizonY - sunR * 0.35

        function xAt(t) {
            return (t - root.t0) / Math.max(1, root.t1 - root.t0) * width;
        }
        function yAt(pct) {
            const f = Math.max(0, Math.min(100, pct)) / 100;
            return lowY + (topY - lowY) * f;
        }
        function polyline(points, field) {
            return points.map(s => Qt.point(xAt(s.t), yAt(s[field])));
        }

        // The sky: the striped sun and the day curve, clipped at the horizon.
        Item {
            id: sky

            x: 0
            y: 0
            width: plot.width
            height: plot.horizonY
            clip: true

            // A haze of pink rising from the horizon.
            Rectangle {
                anchors.fill: parent
                gradient: Gradient {
                    GradientStop { position: 0; color: "transparent" }
                    GradientStop { position: 1; color: root.style.alpha(root.style.pink, 0.2) }
                }
            }

            // The sun: its centre follows the target brightness.
            Instrument.Glow {
                anchors.fill: sunShape
                source: sunShape
                strength: root.style.glow
                tint: root.style.pink
            }
            Shape {
                id: sunShape

                readonly property real centreY: root.targetNow >= 0 ? plot.yAt(root.targetNow) : plot.horizonY

                x: plot.width / 2 - plot.sunR
                y: centreY - plot.sunR
                width: 2 * plot.sunR
                // The sun sets behind the horizon by design: the sky clips the lower
                // part, so the box stops at the horizon rather than leaving the sky.
                height: Math.max(1, Math.min(2 * plot.sunR, plot.horizonY - y))
                visible: root.targetNow >= 0
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeWidth: -1
                    fillGradient: LinearGradient {
                        x1: 0
                        y1: 0
                        x2: 0
                        y2: 2 * plot.sunR
                        GradientStop { position: 0; color: root.style.yellow }
                        GradientStop { position: 1; color: root.style.pink }
                    }
                    PathSvg { path: root.sunPath(plot.sunR) }
                }
            }

            // The day curve, glowing cyan.
            Instrument.Glow {
                anchors.fill: curveShape
                source: curveShape
                strength: root.style.glow
                tint: root.style.cyan
            }
            Shape {
                id: curveShape

                width: sky.width
                height: sky.height
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeColor: root.style.cyan
                    strokeWidth: root.style.line2
                    fillColor: "transparent"
                    joinStyle: ShapePath.RoundJoin
                    capStyle: ShapePath.RoundCap
                    PathPolyline { path: root.samples.length > 1 ? plot.polyline(root.samples, "pct") : [] }
                }
            }

            // Now: a vertical line and a dot on the curve.
            Rectangle {
                visible: root.available && root.now >= root.t0 && root.now <= root.t1
                x: Math.round(plot.xAt(root.now) - root.style.hairline / 2)
                y: 0
                width: root.style.line2
                height: sky.height
                color: root.style.alpha(root.style.pink, 0.45)
            }
            Rectangle {
                id: nowDot

                visible: root.targetNow >= 0 && root.now >= root.t0 && root.now <= root.t1
                width: root.style.snap(root.style.unit * 0.42)
                height: width
                radius: width / 2
                x: plot.xAt(root.now) - width / 2
                y: plot.yAt(root.targetNow) - height / 2
                color: root.style.pink
                border.width: root.style.line2
                border.color: root.style.ink
            }

            // Scrub guide.
            Rectangle {
                visible: scrub.active
                x: Math.round(scrub.mouseX - root.style.hairline / 2)
                y: 0
                width: root.style.line2
                height: sky.height + ground.height
                color: root.style.alpha(root.style.ink, 0.5)
            }
        }

        // The floor: lines converging on the vanishing point at the horizon.
        Item {
            id: ground

            x: 0
            y: plot.horizonY
            width: plot.width
            height: plot.groundH
            clip: true

            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeColor: root.style.alpha(root.style.cyan, 0.42)
                    strokeWidth: root.style.hairline
                    fillColor: "transparent"
                    PathSvg { path: root.floorLines(plot.width, ground.height) }
                }
                ShapePath {
                    strokeColor: root.style.alpha(root.style.pink, 0.5)
                    strokeWidth: root.style.hairline
                    fillColor: "transparent"
                    PathSvg { path: root.rayLines(plot.width, ground.height) }
                }
            }
        }

        // The horizon: a neon line with a bloom.
        Rectangle {
            id: horizon

            x: 0
            y: plot.horizonY - height / 2
            width: plot.width
            height: root.style.line2
            color: root.style.pink
        }
        Instrument.Glow {
            anchors.fill: horizon
            source: horizon
            strength: root.style.glow
            tint: root.style.pink
        }

        MouseArea {
            id: scrub

            readonly property bool active: containsMouse && root.available
            readonly property real time: root.t0 + Math.max(0, Math.min(1, mouseX / Math.max(1, width)))
                * (root.t1 - root.t0)
            readonly property real pct: root.valueAt(root.samples, time, "pct")

            anchors.fill: parent
            hoverEnabled: true
            // Looking only: a click here must never change brightness.
            acceptedButtons: Qt.NoButton
        }

        // The overlay on the sky's top edge: the hero figure with its label on the left, the
        // display chips, the weather's cost and the next milestone on the right, clear of the
        // sun. It is the last child of the stage, so the chips take their clicks; the scrub
        // stops at its edge.
        RowLayout {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: root.style.gap
            spacing: root.style.gap2
            visible: root.available

            ColumnLayout {
                Layout.alignment: Qt.AlignTop
                spacing: 0

                RowLayout {
                    spacing: root.style.gap

                    Text {
                        text: scrub.active
                            ? i18ndc(root.domain, "Time of day, then the brightness the curve targets then",
                                     "%1 · %2%", Format.timeText(scrub.time), Math.round(scrub.pct))
                            : i18ndc(root.domain, "Label above the hero percentage: the brightness right now", "Now")
                        textFormat: Text.PlainText
                        font: root.style.labelFont
                        color: root.style.cyan
                    }
                }
                HeroNumber {
                    text: root.targetNow >= 0
                        ? (scrub.active ? Math.round(scrub.pct) : Math.round(root.targetNow)) + "%"
                        : "—"
                }
            }

            Item {
                Layout.fillWidth: true
            }

            // The chips and the next milestone, to the right of the sun's column; they wrap inside it.
            // A dark plate keeps the text legible over the curve; it stays right of the sun.
            Rectangle {
                Layout.alignment: Qt.AlignTop
                implicitWidth: rightColumn.implicitWidth + root.style.gap * 2
                implicitHeight: rightColumn.implicitHeight + root.style.gap * 2
                radius: root.style.radius * 0.5
                color: root.style.alpha(root.style.night, 0.72)

                ColumnLayout {
                    id: rightColumn

                    readonly property real maxWidth: root.style.unit * 6.5 - root.style.gap * 2

                    anchors.centerIn: parent
                    spacing: root.style.gap * 0.5

                    Flow {
                        Layout.alignment: Qt.AlignRight
                        Layout.maximumWidth: rightColumn.maxWidth
                        visible: root.curves.length > 1
                        spacing: root.style.gap * 0.5

                        Repeater {
                            model: root.curves

                            delegate: Pill {
                                id: chip

                                required property var modelData
                                required property int index

                                style: root.style
                                text: Curve.shortName(chip.modelData.monitorId)
                                tone: root.style.cyan
                                filled: chip.index === root.selected
                                onClicked: root.selected = chip.index
                            }
                        }
                    }
                    Text {
                        Layout.alignment: Qt.AlignRight
                        Layout.maximumWidth: rightColumn.maxWidth
                        visible: root.costNow >= 1
                        text: i18ndc(root.domain, "Brightness the weather is taking away right now", "Weather −%1%",
                                     Math.round(root.costNow))
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.yellow
                        horizontalAlignment: Text.AlignRight
                    }
                    Text {
                        Layout.alignment: Qt.AlignRight
                        Layout.maximumWidth: rightColumn.maxWidth
                        visible: text.length > 0
                        text: root.nextText
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.muted
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        horizontalAlignment: Text.AlignRight
                    }
                }
            }
        }
    }

    // Hour axis.
    Item {
        Layout.fillWidth: true
        visible: root.available
        implicitHeight: hourMetrics.height

        TextMetrics {
            id: hourMetrics

            font: root.style.dataSmallFont
            text: "00"
        }

        Repeater {
            model: [0, 6, 12, 18, 24]

            delegate: Text {
                required property int modelData

                x: Math.max(0, Math.min(parent.width - width, plot.xAt(root.t0 + modelData * 3600) - width / 2))
                text: String(modelData).padStart(2, "0")
                textFormat: Text.PlainText
                font: root.style.dataSmallFont
                color: root.style.muted
            }
        }
    }
}
