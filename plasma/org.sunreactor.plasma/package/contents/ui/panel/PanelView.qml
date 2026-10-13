/*
 * What the panel draws (DESIGN.md section 6), independent of where it lives:
 * CompactRepresentation binds it to the plasmoid, the settings page binds it
 * to unsaved values for the live preview.
 *
 * Modes:
 *   sky        the weather the policy reacts to, as a pixel sprite drawn at a
 *              whole number of device pixels per art pixel;
 *   readout    the brightness of one display (or the mean) in the theme's
 *              digits: bitmap font on a small "screen" for pixel themes,
 *              tabular mono with a level bar otherwise;
 *   sparkline  today's curve with the present moment marked;
 *   ring       a 270 degree gauge of the brightness in the state colour.
 * Plus the temperature (and, opt-in, the sun's height) beside it, unless the
 * item must fit a square (system tray).
 *
 * Colours: text uses the panel's own colour (Kirigami), because the panel is
 * not themed; data marks use the theme's accent / state colours only when
 * they keep a 3:1 contrast against the panel, else they are pulled towards
 * the panel text colour. Pixel themes keep their palette by drawing data on a
 * tile of the theme background, where the palette contrast is by design.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "PanelData.js" as PanelData

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    /*! "sky", "readout", "sparkline" or "ring" (unknown names mean sky). */
    property string mode: "sky"
    /*! Display the number, curve and ring describe; "" = mean of all. */
    property string displayId: ""
    property bool showTemperature: true
    property bool showElevation: false
    property bool vertical: false
    /*! Fit a square of `thickness` (system tray); no labels then. */
    property bool square: false
    /*! The panel's fixed dimension in logical px (height of a horizontal
        panel, width of a vertical one, side of a tray cell). */
    property real thickness: Kirigami.Units.iconSizes.medium
    /*! Sprite url (Icons.js) or a theme icon name (paused, ...). */
    property string skySource: ""
    property bool hovered: false

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    // --- data ------------------------------------------------------------------

    readonly property int percent: client.isConnected
        ? PanelData.displayPercent(client.monitors, displayId, client.globalPercent) : -1
    readonly property var curve: client.isConnected ? PanelData.series(client.dayCurve, displayId) : null
    /*! The mode actually drawn: a sparkline without a curve (older daemon,
        offline) falls back to the number, which shows "—" when unknown. */
    readonly property string effectiveMode: {
        const wanted = PanelData.mode(mode);
        return wanted === "sparkline" && curve === null ? "readout" : wanted;
    }

    // --- colours -------------------------------------------------------------------

    readonly property color panelBackground: Kirigami.Theme.backgroundColor
    readonly property color panelText: Kirigami.Theme.textColor
    /*! Data ink for vector/terminal marks on the panel. */
    readonly property color dataInk: PanelData.ink(theme.available ? theme.accent : Kirigami.Theme.highlightColor,
                                                   panelBackground, panelText, 3)
    readonly property color stateInk: PanelData.ink(theme.available ? theme.stateColor(client.mode)
                                                                    : Kirigami.Theme.highlightColor,
                                                    panelBackground, panelText, 3)
    readonly property color trackInk: Qt.rgba(panelText.r, panelText.g, panelText.b, 0.22)
    /*! Pixel tiles: the palette colour that reads best on the theme bg. */
    readonly property color tileInk: PanelData.bestOn([theme.fg, theme.accent, theme.secondaryAccent], theme.bg)
    readonly property color tileState: {
        const state = theme.stateColor(client.mode);
        return PanelData.contrast(state, theme.bg) >= 2.5 ? state : tileInk;
    }
    readonly property color tileDim: theme.shades.length >= 3 ? theme.shades[1] : theme.borderInactive

    // --- geometry ------------------------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! Side of the square the graphic may use, in logical px. */
    readonly property real graphicSize: Math.max(8, thickness - (thickness >= 32 ? 2 * Math.round(thickness * 0.08) : 2))
    readonly property bool showLabels: !square && labels.visibleCount > 0

    implicitWidth: layout.implicitWidth
    implicitHeight: layout.implicitHeight

    GridLayout {
        id: layout

        anchors.centerIn: parent
        flow: root.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rows: root.vertical ? 2 : 1
        columns: root.vertical ? 1 : 2
        rowSpacing: Kirigami.Units.smallSpacing / 2
        columnSpacing: Kirigami.Units.smallSpacing

        Loader {
            id: graphic

            Layout.alignment: Qt.AlignCenter
            sourceComponent: {
                switch (root.effectiveMode) {
                case "readout":
                    return readoutComponent;
                case "sparkline":
                    return sparklineComponent;
                case "ring":
                    return ringComponent;
                default:
                    return skyComponent;
                }
            }
        }

        PanelLabels {
            id: labels

            Layout.alignment: Qt.AlignCenter
            visible: root.showLabels
            theme: root.theme
            client: root.client
            showTemperature: root.showTemperature
            showElevation: root.showElevation
            vertical: root.vertical
            available: root.vertical ? root.thickness : Number.POSITIVE_INFINITY
            color: root.panelText
        }
    }

    Component {
        id: skyComponent
        Sky {
            view: root
        }
    }

    Component {
        id: readoutComponent
        Readout {
            view: root
        }
    }

    Component {
        id: sparklineComponent
        Sparkline {
            view: root
        }
    }

    Component {
        id: ringComponent
        Ring {
            view: root
        }
    }
}
