/*
 * Light cycle, casiodigital: the day's curve as a chronograph bar graph. Thirty
 * columns of LCD segments cover midnight to midnight, each filled to the target
 * brightness of its half hour; the column of now is lit by the backlight. The
 * hero figure is the target now (seven segments, about 2.4 times the body text),
 * with the clock and what the display actually shows.
 *
 * Several displays: one flag each picks the curve shown. Vector only.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../instrument/Curve.js" as Curve
import "Casio.js" as Casio

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property CasioStyle style: CasioStyle {
        theme: root.theme
    }
    readonly property var curve: client.dayCurve
    readonly property var curves: curve && curve.monitors ? curve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0
        && curves[0].samples.length > 1

    /*! Index into `curves`. */
    property int selected: 0
    readonly property var current: curves.length > 0
        ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property var samples: current ? current.samples : []
    readonly property real t0: samples.length > 0 ? samples[0].t : 0
    readonly property real t1: samples.length > 0 ? samples[samples.length - 1].t : 1
    readonly property real now: client.nowEpochS
    readonly property real targetNow: current ? Curve.valueAt(samples, now, "pct") : -1
    readonly property real appliedNow: appliedFor(current ? current.monitorId : "")
    readonly property var clock: Casio.clockParts(Format.timeText(now))

    readonly property int slots: 30
    /*! Target brightness at the middle of each slot, -1 where the day has no sample. */
    readonly property var levels: {
        const out = [];
        const span = Math.max(1, t1 - t0);
        for (let i = 0; i < slots; ++i) {
            const v = Curve.valueAt(samples, t0 + (i + 0.5) * span / slots, "pct");
            out.push(v < 0 ? -1 : v);
        }
        return out;
    }
    /*! The slot of now, or -1 outside the day. */
    readonly property int nowSlot: now >= t0 && now <= t1 && t1 > t0
        ? Math.min(slots - 1, Math.floor((now - t0) / (t1 - t0) * slots)) : -1

    function appliedFor(monitorId) {
        for (const monitor of client.monitors) {
            if (monitor.logicalId === monitorId) {
                return monitor.appliedPercent;
            }
        }
        return -1;
    }

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18nd(root.settings.domain, "Light cycle")
        trailing: i18nd(root.settings.domain, "Today")

        titleItems: Repeater {
            // One flag per display, on the title row, when there is more than one.
            model: root.curves.length > 1 ? root.curves : []

            delegate: Item {
                id: chip

                required property var modelData
                required property int index

                width: flag.width
                height: flag.height

                CasioFlag {
                    id: flag

                    style: root.style
                    text: Curve.shortName(chip.modelData.monitorId)
                    lit: root.selected === chip.index
                }
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.selected = chip.index
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gapWide

            ColumnLayout {
                spacing: 0

                RowLayout {
                    spacing: root.style.gap

                    CasioDigits {
                        id: hero

                        style: root.style
                        size: root.style.digitHero
                        text: Casio.numberText(Math.round(root.targetNow), 3)
                        Accessible.name: i18nd(root.settings.domain, "Brightness the curve targets now")
                    }
                    Text {
                        Layout.alignment: Qt.AlignBottom
                        text: "%"
                        textFormat: Text.PlainText
                        font: root.style.cap(root.style.sizeBody)
                        color: root.style.inkMuted
                    }
                }
                Text {
                    text: i18nd(root.settings.domain, "Target now")
                    textFormat: Text.PlainText
                    font: root.style.cap(root.style.sizeCap)
                    color: root.style.inkMuted
                }
            }

            Item {
                Layout.fillWidth: true
            }

            ColumnLayout {
                spacing: 0

                RowLayout {
                    Layout.alignment: Qt.AlignRight
                    spacing: root.style.gap

                    CasioDigits {
                        style: root.style
                        size: root.style.digitSmall
                        text: root.clock.digits
                    }
                    Text {
                        Layout.alignment: Qt.AlignBottom
                        visible: root.clock.suffix.length > 0
                        text: root.clock.suffix
                        textFormat: Text.PlainText
                        font: root.style.cap(root.style.sizeCap)
                        color: root.style.inkMuted
                    }
                }
                Text {
                    Layout.alignment: Qt.AlignRight
                    text: root.appliedNow >= 0
                        ? i18nd(root.settings.domain, "Applied %1%", Math.round(root.appliedNow))
                        : i18nd(root.settings.domain, "Applied: unknown")
                    textFormat: Text.PlainText
                    font: root.style.cap(root.style.sizeCap)
                    color: root.style.inkMuted
                }
            }
        }

        // The bar graph: thirty columns, ten segments high.
        Item {
            id: plot

            Layout.fillWidth: true
            implicitHeight: root.style.snap(root.style.unit * 2.8)
            Accessible.role: Accessible.Chart
            Accessible.name: i18nd(root.settings.domain, "Today's brightness curve")

            readonly property var paths: Casio.bars(root.levels, 10, width, height, root.style.dpr, root.nowSlot)

            Rectangle {
                visible: root.nowSlot >= 0
                x: root.nowSlot * plot.width / root.slots
                y: 0
                width: plot.width / root.slots
                height: plot.height
                color: root.style.backlightSoft
            }

            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.ghost
                    PathSvg { path: plot.paths.ghost }
                }
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.ink
                    PathSvg { path: plot.paths.lit }
                }
                ShapePath {
                    strokeWidth: -1
                    fillColor: root.style.backlight
                    PathSvg { path: plot.paths.hot }
                }
            }

            Rectangle {
                y: plot.height - root.style.hairline
                width: plot.width
                height: root.style.hairline
                color: root.style.inkMuted
            }
        }

        // Hours from midnight of the curve's day.
        Item {
            Layout.fillWidth: true
            implicitHeight: axisSize.implicitHeight

            Text {
                id: axisSize

                visible: false
                text: "00"
                font: root.style.cap(root.style.sizeCap)
            }

            Repeater {
                id: hours

                model: [0, 6, 12, 18, 24]

                delegate: Text {
                    id: hourLabel

                    required property int modelData

                    readonly property real fraction: root.t1 > root.t0
                        ? Math.min(1, modelData * 3600 / (root.t1 - root.t0)) : 0

                    x: Math.max(0, Math.min(parent.width - width, fraction * parent.width
                        - (modelData === 0 ? 0 : (modelData === 24 ? width : width / 2))))
                    text: String(modelData).padStart(2, "0")
                    textFormat: Text.PlainText
                    font: root.style.cap(root.style.sizeCap)
                    color: root.style.inkMuted
                }
            }
        }
    }
}
