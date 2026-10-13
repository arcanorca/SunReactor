/*
 * The forecast as a details view, Windows XP: column headers in XP's header
 * style, one ruled row per interval. Time, temperature and cloud cover are
 * figures; the sky is a word. Frameless: the Forecast window shows it alone, and
 * the Weather window hosts it when both modules are listed.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property WinStyle ui: WinStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    /*! Upcoming intervals, at most three. */
    readonly property var points: {
        const upcoming = [];
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === 3) {
                break;
            }
        }
        return upcoming;
    }
    readonly property bool available: client.isConnected && client.weatherEnabled && points.length > 0

    WeatherText {
        id: weatherText
    }

    readonly property var columns: [
        { head: i18ndc(domain, "Forecast column: time of the interval", "Time"), weight: 25 },
        { head: i18ndc(domain, "Forecast column: sky condition", "Sky"), weight: 40 },
        { head: i18ndc(domain, "Forecast column: temperature", "Temp"), weight: 17 },
        { head: i18ndc(domain, "Forecast column: cloud cover", "Cloud"), weight: 18 }
    ]

    function cellText(point, column) {
        switch (column) {
        case 0:
            return Format.timeText(point.epochS);
        case 1:
            return weatherText.conditionLabel(point.condition) || "—";
        case 2:
            return i18ndc(domain, "Temperature in degrees, compact", "%1°", Math.round(point.temperatureC));
        default:
            return i18ndc(domain, "Percentage", "%1%", Math.round(point.cloudCoverPercent));
        }
    }

    spacing: 0

    Rectangle {
        id: list

        Layout.fillWidth: true
        implicitHeight: table.implicitHeight + 2 * root.ui.line
        color: root.ui.white
        border.width: root.ui.line
        border.color: root.ui.clientEdge

        Column {
            id: table

            x: root.ui.line
            y: root.ui.line
            width: parent.width - 2 * root.ui.line

            // The column headers: XP header buttons.
            Row {
                id: header

                width: table.width
                height: root.ui.rowH

                Repeater {
                    model: root.columns

                    delegate: Rectangle {
                        id: head

                        required property var modelData

                        width: header.width * modelData.weight / 100
                        height: header.height
                        border.width: root.ui.line
                        border.color: root.ui.etchDark
                        gradient: Gradient {
                            GradientStop { position: 0; color: root.ui.white }
                            GradientStop { position: 1; color: root.ui.panel }
                        }

                        Text {
                            anchors.centerIn: parent
                            width: parent.width - root.ui.gap
                            text: head.modelData.head
                            textFormat: Text.PlainText
                            horizontalAlignment: Text.AlignHCenter
                            font: root.ui.smallBoldFont
                            color: root.ui.ink
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            // One ruled row per interval.
            Repeater {
                model: root.points

                delegate: Row {
                    id: interval

                    required property var modelData

                    width: table.width
                    height: root.ui.rowH

                    Repeater {
                        model: root.columns

                        delegate: Item {
                            id: cell

                            required property var modelData
                            required property int index

                            width: interval.width * cell.modelData.weight / 100
                            height: interval.height

                            Text {
                                x: root.ui.pad
                                anchors.verticalCenter: parent.verticalCenter
                                width: cell.width - root.ui.pad * 1.5
                                text: root.cellText(interval.modelData, cell.index)
                                textFormat: Text.PlainText
                                font: cell.index === 1 ? root.ui.bodyFont : root.ui.figureFont
                                color: root.ui.ink
                                elide: Text.ElideRight
                            }
                            Rectangle {
                                visible: cell.index > 0
                                width: root.ui.hairline
                                height: parent.height
                                color: root.ui.gridline
                            }
                            Rectangle {
                                y: parent.height - root.ui.hairline
                                width: cell.width
                                height: root.ui.hairline
                                color: root.ui.gridline
                            }
                        }
                    }
                }
            }
        }
    }
}
