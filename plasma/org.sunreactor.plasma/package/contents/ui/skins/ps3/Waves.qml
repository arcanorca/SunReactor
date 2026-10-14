/*
 * The XMB wave: translucent silk ribbons flowing slowly across the popup, over
 * the navy backdrop and under the content. FullRepresentation loads it inside
 * the popup Frame (Modules.OVERLAY).
 *
 * Each ribbon is a band along a sine that repeats once per popup width, so the
 * loop is seamless: the drift carries a ribbon one width per loop and the sway
 * returns to its start. The paths are built once per size, each ribbon is drawn
 * once into a texture (ShaderEffectSource, live: false), and the frames only move
 * those textures (x and y). Five nested bands per ribbon (wide and faint, narrow
 * and bright) give the silk its soft edges without a blur.
 *
 * The loop runs only while the popup window is on screen and the desktop's
 * animation speed is not "instant"; otherwise the first frame stays still.
 * Screenshot harness only: the motion_t config (SUNREACTOR_HARNESS_CONFIG)
 * freezes the loop at that fraction, for comparing frames.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasmoid

import "../../engine"

Item {
    id: root

    required property Theme theme

    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }

    clip: true

    /*! Position in the loop, 0 to 1. One loop moves each ribbon one popup width. */
    property real phase: 0
    /*! Seconds for one loop: slow enough to read as drifting silk. */
    readonly property int loopMs: 13000
    /*! The desktop's animation speed. Kirigami reports 1 ms, not 0, when it is
        "instant": that keeps the frame still. */
    readonly property bool motion: Kirigami.Units.longDuration > 1
    /*! Harness only: the frozen phase from motion_t, or -1 for the live loop. */
    readonly property real frozenPhase: {
        const v = Number(Plasmoid.configuration.motion_t);
        return Number.isFinite(v) && v >= 0 && v < 1 ? v : -1;
    }
    /*! The loop plays only while the popup is on screen (never when it is closed). */
    readonly property bool playing: motion && frozenPhase < 0 && visible && Window.window !== null
        && Window.visibility !== Window.Hidden && Window.visibility !== Window.Minimized

    Binding {
        target: root
        property: "phase"
        value: root.frozenPhase
        when: root.frozenPhase >= 0
    }

    NumberAnimation {
        target: root
        property: "phase"
        from: 0
        to: 1
        duration: root.loopMs
        loops: Animation.Infinite
        running: root.playing
    }

    /*! The ribbons, back to front. y is the centre line, amp the height of the
        wave, thick the full thickness and bob the sway (fractions of the popup
        height); cycles the waves per popup width, sway the sway's cycles per loop,
        shift the wave's offset (fraction of a cycle); peak the opacity at the
        centre of the widest band. */
    readonly property var ribbons: [
        { y: 0.25, amp: 0.075, thick: 0.26, bob: 0.025, cycles: 1, sway: 1, shift: 0.00, peak: 0.12 },
        { y: 0.52, amp: 0.095, thick: 0.22, bob: 0.035, cycles: 2, sway: 2, shift: 0.35, peak: 0.11 },
        { y: 0.80, amp: 0.060, thick: 0.22, bob: 0.025, cycles: 1, sway: 1, shift: 0.62, peak: 0.10 }
    ]

    /*! The nested bands of each ribbon, widest and faintest first: share is the
        band's fraction of the ribbon's thickness, k its share of the ribbon's
        peak opacity, color its tint. Stacked, they fall off towards the edges. */
    readonly property var bands: [
        { share: 1.0, k: 0.22, color: root.style.accent },
        { share: 0.8, k: 0.28, color: root.style.accent },
        { share: 0.6, k: 0.36, color: root.style.accentSoft },
        { share: 0.4, k: 0.46, color: root.style.accentSoft },
        { share: 0.2, k: 0.85, color: root.style.ink }
    ]

    /*! The centre line of ribbon r at x (logical px, 0 to two popup widths). The
        wave repeats every popup width, so the path loops with the drift. */
    function centreY(r, x) {
        const u = 2 * Math.PI * (r.cycles * x / root.width + r.shift);
        return root.height * (r.y + r.amp * (Math.sin(u) + 0.4 * Math.sin(2 * u + 1.3)));
    }

    /*! The outline of one band of ribbon r: its centre line shifted by half of
        `share` times its thickness, along the top and back along the bottom. */
    function bandPath(r, share) {
        const n = 96;
        const span = 2 * root.width;
        const half = root.height * r.thick * share / 2;
        const top = [];
        const bottom = [];
        for (let i = 0; i <= n; ++i) {
            const x = span * i / n;
            const c = root.centreY(r, x);
            top.push(Qt.point(x, c - half));
            bottom.push(Qt.point(x, c + half));
        }
        return top.concat(bottom.reverse());
    }

    Repeater {
        model: root.ribbons

        delegate: Item {
            id: ribbon

            required property var modelData

            // Two popup widths: the drift never shows the path's open end.
            width: root.width * 2
            height: root.height
            x: root.phase * root.width - root.width
            y: modelData.bob * root.height * Math.sin(2 * Math.PI * (modelData.sway * root.phase + modelData.shift))

            // The rows the band can reach (its wave, its thickness and its sway):
            // the texture covers only those, so a frame blends less of the popup.
            readonly property real reachTop: Math.max(0, root.height * (modelData.y - 1.4 * modelData.amp
                                                     - modelData.thick / 2 - modelData.bob))
            readonly property real reachBottom: Math.min(root.height, root.height * (modelData.y + 1.4 * modelData.amp
                                                        + modelData.thick / 2 + modelData.bob))

            // The ribbon is drawn into a texture once per size (live: false) and
            // only moved from then on: a frame costs a textured quad, not the curves.
            ShaderEffectSource {
                id: capture

                x: 0
                y: ribbon.reachTop
                width: ribbon.width
                height: Math.max(1, ribbon.reachBottom - ribbon.reachTop)
                sourceItem: art
                sourceRect: Qt.rect(0, ribbon.reachTop, ribbon.width, height)
                hideSource: true
                live: false
                smooth: true

                onWidthChanged: scheduleUpdate()
                onHeightChanged: scheduleUpdate()
                onYChanged: scheduleUpdate()
            }

            Item {
                id: art

                width: ribbon.width
                height: ribbon.height

                Repeater {
                    model: root.bands

                    delegate: Shape {
                        id: band

                        required property var modelData

                        anchors.fill: parent
                        preferredRendererType: Shape.CurveRenderer

                        function tone(f) {
                            return root.style.alpha(band.modelData.color, ribbon.modelData.peak * band.modelData.k * f);
                        }

                        ShapePath {
                            strokeColor: "transparent"
                            fillGradient: LinearGradient {
                                x1: 0
                                y1: 0
                                x2: ribbon.width
                                y2: 0
                                // Brighter every half popup width, so the silk reads as folded light.
                                GradientStop { position: 0; color: band.tone(0.55) }
                                GradientStop { position: 0.25; color: band.tone(1) }
                                GradientStop { position: 0.5; color: band.tone(0.55) }
                                GradientStop { position: 0.75; color: band.tone(1) }
                                GradientStop { position: 1; color: band.tone(0.55) }
                            }
                            PathPolyline { path: root.bandPath(ribbon.modelData, band.modelData.share) }
                        }
                    }
                }
            }
        }
    }
}
