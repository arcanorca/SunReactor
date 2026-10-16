/*
 * Light cycle module (the hero): today's policy curve from the daemon's
 * day_curve - midnight to midnight, 0..100 % on an honest scale.
 *
 *   line          target brightness (pct) for the selected display
 *   shaded area   between pct_no_weather and pct: what the weather costs
 *   band          the display's min..max limits
 *   ticks         sunrise, solar noon, sunset
 *   now           vertical marker, dot on the target, ring on the applied level
 *   hover         scrubs: time -> target %; it never sets brightness
 *
 * Tabs choose the display when there are two or more, plus "All" (every
 * curve, no shading). Hidden when the daemon has no day_curve (older daemon)
 * or there is no curve for today. Vector rendering; pixel/terminal skins
 * replace this file.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0
        && curves[0].samples.length > 1

    /*! Index into `curves`, or -1 for "All". */
    property int selected: 0
    readonly property bool showAll: selected < 0 && curves.length > 1
    readonly property var current: curves.length > 0 ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null

    readonly property real t0: available ? curves[0].samples[0].t : 0
    readonly property real t1: available ? curves[0].samples[curves[0].samples.length - 1].t : 1
    readonly property real now: client.nowEpochS

    /*! Linear interpolation of a sample field at time t; -1 outside the day. */
    function valueAt(samples, t, field) {
        if (!samples || samples.length === 0 || t < samples[0].t || t > samples[samples.length - 1].t) {
            return -1;
        }
        for (let i = 1; i < samples.length; ++i) {
            const b = samples[i];
            if (b.t >= t) {
                const a = samples[i - 1];
                const f = b.t === a.t ? 0 : (t - a.t) / (b.t - a.t);
                return a[field] + (b[field] - a[field]) * f;
            }
        }
        return samples[samples.length - 1][field];
    }

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    readonly property real targetNow: current ? valueAt(current.samples, now, "pct") : -1
    readonly property real costNow: current
        ? valueAt(current.samples, now, "pctNoWeather") - valueAt(current.samples, now, "pct") : 0

    spacing: root.theme.spacing

    Accessible.role: Accessible.Chart
    Accessible.name: i18nd(domain, "Today's brightness curve")
    Accessible.description: current && targetNow >= 0
        ? i18ndc(domain, "Display name, then the brightness the curve targets now",
                 "%1: %2% now", current.monitorId, Math.round(targetNow))
        : ""

    // Selected display, or "All" with two or more. Plain checkable buttons:
    // the selection is this module's state, never reshuffled by the model.
    RowLayout {
        Layout.fillWidth: true
        visible: root.curves.length > 1
        spacing: 0

        Repeater {
            model: root.curves.length > 1 ? root.curves.length + 1 : 0

            delegate: PlasmaComponents3.ToolButton {
                id: tab

                required property int index
                readonly property bool isAll: index === root.curves.length
                readonly property int value: isAll ? -1 : index

                Layout.fillWidth: true
                Layout.maximumWidth: implicitWidth
                checkable: true
                checked: root.selected === value
                text: isAll ? i18ndc(root.domain, "Show the curves of every display", "All")
                            : root.curves[index].monitorId
                font.family: root.theme.bodyFont
                onClicked: root.selected = value
                Accessible.name: text
            }
        }

        Item {
            Layout.fillWidth: true
        }
    }

    // Readout: what the curve says right now, or under the pointer.
    RowLayout {
        Layout.fillWidth: true
        spacing: root.theme.spacing

        PlasmaComponents3.Label {
            Layout.fillWidth: true
            textFormat: Text.PlainText
            font.family: root.theme.digitsFont
            font.pixelSize: root.theme.fontSizeBody
            font.features: ({ "tnum": 1 })
            color: root.theme.fg
            elide: Text.ElideRight
            text: {
                if (scrub.active && root.current) {
                    const pct = root.valueAt(root.current.samples, scrub.time, "pct");
                    return i18ndc(root.domain, "Time of day, then the brightness the curve targets then",
                                  "%1 · %2%", Format.timeText(scrub.time), Math.round(pct));
                }
                if (root.showAll || root.targetNow < 0) {
                    return i18nd(root.domain, "Today");
                }
                return i18ndc(root.domain, "Brightness the curve targets now", "Now %1%", Math.round(root.targetNow));
            }
        }

        PlasmaComponents3.Label {
            visible: !root.showAll && root.costNow >= 1
            textFormat: Text.PlainText
            font.family: root.theme.digitsFont
            font.pixelSize: root.theme.fontSizeBody
            font.features: ({ "tnum": 1 })
            color: root.theme.textMuted
            text: i18ndc(root.domain, "Brightness the weather is taking away right now", "Weather −%1%",
                         Math.round(root.costNow))
        }
    }

    Item {
        id: plot

        Layout.fillWidth: true
        implicitHeight: Math.round(root.theme.gridUnit * 4.5)

        readonly property real pad: root.theme.spacing
        function xAt(t) {
            return (t - root.t0) / Math.max(1, root.t1 - root.t0) * width;
        }
        function yAt(pct) {
            return pad + (1 - Math.max(0, Math.min(100, pct)) / 100) * (height - 2 * pad);
        }
        function polyline(samples, field) {
            return samples.map(s => Qt.point(xAt(s.t), yAt(s[field])));
        }

        // Plot frame: baseline and the 100 % line.
        Rectangle {
            x: 0; y: plot.yAt(0); width: plot.width; height: root.theme.hairline
            color: root.theme.line
        }
        Rectangle {
            x: 0; y: plot.yAt(100); width: plot.width; height: root.theme.hairline
            color: root.theme.alpha(root.theme.line, 0.5)
        }

        // Limits band of the selected display.
        Rectangle {
            visible: !root.showAll && root.current !== null && root.current.maxPct >= 0
            x: 0
            width: plot.width
            y: root.current ? plot.yAt(root.current.maxPct) : 0
            height: root.current ? plot.yAt(Math.max(0, root.current.minPct)) - y : 0
            color: root.theme.alpha(root.theme.fg, 0.05)
        }

        // Sunrise, solar noon, sunset.
        Repeater {
            model: root.available && root.curve.sun
                ? [root.curve.sun.sunrise, root.curve.sun.solarNoon, root.curve.sun.sunset].filter(t => t > 0)
                : []

            delegate: Rectangle {
                required property real modelData
                x: Math.round(plot.xAt(modelData))
                y: plot.pad
                width: root.theme.hairline
                height: plot.height - 2 * plot.pad
                color: root.theme.alpha(root.theme.secondaryAccent, 0.45)
            }
        }

        // What the weather costs, then the curve itself.
        Shape {
            anchors.fill: parent
            visible: !root.showAll && root.current !== null
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeWidth: -1
                fillColor: root.theme.alpha(root.theme.warning, 0.30)
                PathPolyline {
                    path: root.current
                        ? plot.polyline(root.current.samples, "pctNoWeather")
                            .concat(plot.polyline(root.current.samples, "pct").reverse())
                        : []
                }
            }
        }

        Repeater {
            model: root.showAll ? root.curves : (root.current ? [root.current] : [])

            delegate: Shape {
                id: curveShape

                required property var modelData
                required property int index

                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeColor: !root.showAll || curveShape.index === 0 ? root.theme.accent
                        : (curveShape.index === 1 ? root.theme.secondaryAccent : root.theme.textMuted)
                    strokeWidth: Math.max(1.5, root.theme.lineWidth * 2)
                    fillColor: "transparent"
                    joinStyle: ShapePath.RoundJoin
                    capStyle: ShapePath.RoundCap
                    PathPolyline {
                        path: plot.polyline(curveShape.modelData.samples, "pct")
                    }
                }
            }
        }

        // Now: marker, target dot, applied ring.
        Rectangle {
            visible: root.now >= root.t0 && root.now <= root.t1
            x: Math.round(plot.xAt(root.now))
            y: plot.pad
            width: root.theme.lineWidth
            height: plot.height - 2 * plot.pad
            color: root.theme.alpha(root.theme.fg, 0.6)
        }

        Rectangle {
            readonly property real applied: root.current ? root.appliedFor(root.current.monitorId) : -1
            visible: !root.showAll && applied >= 0 && root.now >= root.t0 && root.now <= root.t1
            width: root.theme.spacing * 3
            height: width
            radius: root.theme.isVector ? width / 2 : 0
            x: plot.xAt(root.now) - width / 2
            y: plot.yAt(applied) - height / 2
            color: "transparent"
            border.width: root.theme.lineWidth * 1.5
            border.color: root.theme.fg
        }

        Rectangle {
            visible: !root.showAll && root.targetNow >= 0
            width: root.theme.spacing * 2
            height: width
            radius: root.theme.isVector ? width / 2 : 0
            x: plot.xAt(root.now) - width / 2
            y: plot.yAt(root.targetNow) - height / 2
            color: root.theme.accent
        }

        // Scrub guide.
        Rectangle {
            visible: scrub.active
            x: Math.round(scrub.mouseX)
            y: plot.pad
            width: root.theme.hairline
            height: plot.height - 2 * plot.pad
            color: root.theme.borderActive
        }

        MouseArea {
            id: scrub

            readonly property bool active: containsMouse && root.available
            readonly property real time: root.t0 + Math.max(0, Math.min(1, mouseX / Math.max(1, width)))
                * (root.t1 - root.t0)

            anchors.fill: parent
            hoverEnabled: true
            // Looking only: a click here must never change brightness.
            acceptedButtons: Qt.NoButton
        }
    }

    // Hour axis.
    Item {
        Layout.fillWidth: true
        implicitHeight: hourMetrics.height

        TextMetrics {
            id: hourMetrics
            font.family: Kirigami.Theme.smallFont.family
            font.pixelSize: root.theme.fontSizeSmall
            text: "00"
        }

        Repeater {
            model: [0, 6, 12, 18, 24]

            delegate: PlasmaComponents3.Label {
                required property int modelData
                x: Math.max(0, Math.min(parent.width - width, plot.xAt(root.t0 + modelData * 3600) - width / 2))
                text: String(modelData).padStart(2, "0")
                font.family: Kirigami.Theme.smallFont.family
                font.pixelSize: root.theme.fontSizeSmall
                color: root.theme.textMuted
            }
        }
    }
}
