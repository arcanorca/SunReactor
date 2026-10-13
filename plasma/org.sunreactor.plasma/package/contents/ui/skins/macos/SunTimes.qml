/*
 * Sun times, macos: the day as an arc from sunrise to sunset, with a warm dot
 * where the sun is now. Rise and set sit under the ends of the arc, and the
 * length of the day (or the solar elevation, when shown) between them.
 *
 *          ╭──────╮
 *      ╭───╯  ●   ╰───╮
 *   ───╯               ╰───
 *   Rise 7:13    11 h 25 min    Set 6:38 PM
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property real dayLength: Math.max(0, client.sunsetEpochS - client.sunriseEpochS)
    /*! Where now is along the day: 0 at sunrise, 1 at sunset; outside 0..1 at night. */
    readonly property real fraction: dayLength > 0 ? (client.nowEpochS - client.sunriseEpochS) / dayLength : -1
    readonly property bool inDay: fraction >= 0 && fraction <= 1
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation

    function hoursText(seconds) {
        const minutes = Math.round(seconds / 60);
        const hours = Math.floor(minutes / 60);
        return i18ndc(domain, "Length of the day, hours and minutes", "%1 h %2 min", hours, minutes % 60);
    }

    /*! The arc's height above the horizon at fraction f, 0..1. */
    function lift(f) {
        return Math.sin(Math.PI * f);
    }

    /*! Points along the arc from sunrise to fraction `upTo`, in the arc's box. */
    function arcPoints(upTo) {
        const w = arcBox.width;
        const h = arcBox.height;
        const steps = 48;
        const out = [];
        for (let i = 0; i <= steps; ++i) {
            const f = upTo * i / steps;
            out.push(Qt.point(f * w, h - lift(f) * h));
        }
        return out;
    }

    spacing: style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Accessible name of the sun times", "Sun")

    Item {
        id: sunBand

        Layout.fillWidth: true
        Layout.preferredHeight: root.style.snap(30)

        // The arc: the whole day faint, the part the sun has covered in its yellow.
        Item {
            id: arcBox

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: root.style.snap(16)

            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeColor: root.style.theme.alpha(root.style.ink, 0.22)
                    strokeWidth: 1.5
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    joinStyle: ShapePath.RoundJoin
                    PathPolyline {
                        path: root.arcPoints(1)
                    }
                }

                ShapePath {
                    strokeColor: root.style.sun
                    strokeWidth: 2
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap
                    joinStyle: ShapePath.RoundJoin
                    PathPolyline {
                        path: root.arcPoints(root.inDay ? root.fraction : 0)
                    }
                }
            }

            // Now, on the arc.
            Rectangle {
                visible: root.inDay
                x: root.style.snap(root.fraction * arcBox.width - 4.5)
                y: root.style.snap(arcBox.height - root.lift(root.fraction) * arcBox.height - 4.5)
                width: 9
                height: 9
                radius: 4.5
                antialiasing: true
                color: root.style.sun
                border.width: 2
                border.color: root.style.ink
            }
        }

        // The horizon.
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: arcBox.bottom
            height: root.style.hairline
            color: root.style.separator
        }

        // Rise, the day's length (or the elevation) and set, on one band under the horizon.
        Text {
            anchors.left: parent.left
            anchors.bottom: parent.bottom
            text: i18ndc(root.domain, "Sunrise time", "Rise %1", Format.timeText(client.sunriseEpochS))
            textFormat: Text.PlainText
            font: root.style.caption
            color: root.style.inkSecondary
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            text: root.showElevation
                ? i18ndc(root.domain, "Solar elevation in degrees", "%1°", client.solarElevation.toFixed(1))
                : root.hoursText(root.dayLength)
            textFormat: Text.PlainText
            font: root.style.captionMedium
            color: root.style.ink
        }

        Text {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            text: i18ndc(root.domain, "Sunset time", "Set %1", Format.timeText(client.sunsetEpochS))
            textFormat: Text.PlainText
            font: root.style.caption
            color: root.style.inkSecondary
        }
    }
}
