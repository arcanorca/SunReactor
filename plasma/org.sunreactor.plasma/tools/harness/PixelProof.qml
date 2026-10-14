/*
 * Pixel-rendering proof: a pixel-themed test surface built only from the
 * engine (PixelSurface, BitmapText, Dither, Frame, discrete cells), fed by the
 * real SunReactorClient talking to tools/fake_daemon.py. tools/pixel_proof.sh
 * runs it under Xvfb at several device pixel ratios, captures the screen and
 * runs tools/conformance.py on every region reported here.
 *
 * Regions whose name starts with "control_" are deliberate negative controls
 * (plain QML Text inside an art-resolution layer) that MUST fail the check;
 * they prove the checker is able to see the failure mode.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import org.sunreactor.plasma

import "../../package/contents/ui/engine"

Window {
    id: win

    x: 0
    y: 0
    width: 520
    height: 360
    visible: true
    color: "#202020"

    SunReactorClient {
        id: daemon
    }

    ThemeEngine {
        id: engine
        client: daemon
    }

    readonly property Theme theme: engine.theme

    function report(name, item) {
        const r = theme.grid.deviceRect(item);
        console.info("SUNREACTOR_PIXEL_REGION " + JSON.stringify({
            name: name, x: r.x, y: r.y, w: r.w, h: r.h,
            k: theme.grid.deviceScale, dpr: theme.grid.devicePixelRatio,
        }));
    }

    Column {
        x: 7.3 // deliberately fractional: the grid must not depend on placement
        y: 5.6
        spacing: 6

        PixelSurface {
            id: surface

            grid: win.theme.grid
            artWidth: 168
            artHeight: win.theme.bitmapPixelSize > 8 ? 116 : 76
            regionName: "surface"

            Rectangle {
                width: surface.artWidth
                height: surface.artHeight
                color: win.theme.bg
            }

            Frame {
                theme: win.theme
                pixel: true
                style: "double"
                width: surface.artWidth
                height: surface.artHeight
                color: win.theme.borderActive
            }

            Rectangle { // telemetry dot
                x: 6; y: 7; width: 4; height: 4
                color: win.theme.success
            }

            BitmapText {
                id: line1
                x: 13; y: 5
                theme: win.theme
                inSurface: true
                text: "LIVE 42% ->34%"
            }

            BitmapText {
                id: line2
                x: 6; y: line1.y + line1.artHeight + 2
                theme: win.theme
                inSurface: true
                color: win.theme.textMuted
                text: "MI 5% · LEN 9% 15°"
            }

            Dither {
                id: ramp
                x: 6; y: line2.y + line2.artHeight + 3
                width: 156; height: 8
                theme: win.theme
                inSurface: true
                level: 0
                levelTo: 1
            }

            Row { // discrete slider: N cells, never width * progress
                id: cells
                x: 6; y: ramp.y + 12
                spacing: 1

                Repeater {
                    model: 22

                    Rectangle {
                        required property int index
                        width: 6
                        height: 6
                        color: index < 13 ? win.theme.accent : win.theme.borderInactive
                    }
                }
            }

            BitmapText {
                x: 6; y: cells.y + 10
                theme: win.theme
                inSurface: true
                magnification: 2
                color: win.theme.secondaryAccent
                text: "18:38"
            }
        }

        Rectangle {
            // Standalone pieces sit on the theme background, like in a popup.
            color: win.theme.bg
            width: standaloneRow.width
            height: standaloneRow.height

        Row {
            id: standaloneRow
            spacing: 5

            BitmapText {
                id: standaloneText
                theme: win.theme
                text: "STANDALONE 7%"
            }

            Dither {
                id: standaloneDither
                theme: win.theme
                width: 30 * win.theme.grid.unit
                height: standaloneText.height
                level: 0.4
            }
        }
        }

        // Negative control 1: QML Text inside an art-resolution layer.
        PixelSurface {
            id: controlSurface

            grid: win.theme.grid
            artWidth: 96
            artHeight: 12

            Rectangle {
                width: controlSurface.artWidth
                height: controlSurface.artHeight
                color: win.theme.bg
            }

            Text {
                x: 2
                y: 2
                text: "TEXT 42%"
                color: win.theme.fg
                font.family: win.theme.bitmapFont
                font.pixelSize: 8
                renderType: Text.NativeRendering
            }
        }
    }

    Timer {
        interval: 1800
        running: true
        onTriggered: {
            win.report("surface", surface.canvas);
            win.report("standalone_text", standaloneText);
            win.report("standalone_dither", standaloneDither);
            win.report("control_text_in_layer", controlSurface.canvas);
            console.info("SUNREACTOR_THEME " + JSON.stringify({
                id: win.theme.themeId, renderer: win.theme.renderer, font: win.theme.bitmapFont,
                k: win.theme.grid.deviceScale, unit: win.theme.grid.unit,
                dpr: win.theme.grid.devicePixelRatio,
                shades: win.theme.shades.map(c => c.toString()),
            }));
            console.info("SUNREACTOR_READY");
        }
    }
}
