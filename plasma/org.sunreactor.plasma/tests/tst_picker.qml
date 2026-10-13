/*
 * The theme picker under the theme button (qmltestrunner, offscreen): one click
 * on the button opens the list, a click on a row or on Follow applies the choice
 * and keeps the list open, a press outside, Escape or a second click on the
 * button closes it, and the keyboard moves and applies. The client is an
 * offline SunReactorClient; the list is the stub's themes and the stub records
 * the choices the way FullRepresentation stores them in the widget's settings.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../package/contents/ui/components"
import "../package/contents/ui/engine"

Item {
    id: stage

    width: 320
    height: 480

    // list_themes entries, the shape the client gives (families in pixel, instrument, terminal).
    readonly property var stubThemes: [
        { id: "handheld", name: "Handheld", family: "pixel", bg: "#141c12", fg: "#9bbc0f", accent: "#8b1d42", secondaryAccent: "#8bac0f" },
        { id: "commodore64", name: "Commodore 64", family: "pixel", bg: "#101225", fg: "#a7b7ff", accent: "#ffffff", secondaryAccent: "#7fe5e5" },
        { id: "braun", name: "Braun", family: "instrument", bg: "#161616", fg: "#f2f2f0", accent: "#ff5500", secondaryAccent: "#f0b01e" },
        { id: "nord", name: "Nord", family: "instrument", bg: "#2e3440", fg: "#d8dee9", accent: "#88c0d0", secondaryAccent: "#81a1c1" },
        { id: "synthwave84", name: "Synthwave '84", family: "instrument", bg: "#262335", fg: "#ffffff", accent: "#ff7edb", secondaryAccent: "#36f9f6" },
        { id: "amber", name: "Amber", family: "instrument", bg: "#1a1100", fg: "#ffb000", accent: "#ff8c00", secondaryAccent: "#ffd27f" },
        { id: "nixie", name: "Nixie", family: "instrument", bg: "#0d0d0d", fg: "#ff9a3c", accent: "#ff4d00", secondaryAccent: "#ffc78a" },
        { id: "oscilloscope", name: "Oscilloscope", family: "instrument", bg: "#050a05", fg: "#39ff14", accent: "#00ff88", secondaryAccent: "#0f3d0f" },
        { id: "cyberpunk", name: "Cyberpunk", family: "instrument", bg: "#0a0a12", fg: "#fcee0a", accent: "#00f0ff", secondaryAccent: "#ff003c" },
        { id: "dracula", name: "Dracula", family: "terminal", bg: "#282a36", fg: "#f8f8f2", accent: "#bd93f9", secondaryAccent: "#ff79c6" },
        { id: "vfdhifi", name: "VFD Hi-Fi", family: "instrument", bg: "#06100c", fg: "#7fffd4", accent: "#00d19a", secondaryAccent: "#1e4d40" },
    ]

    // What the widget keeps in its settings; the stub's handlers write here.
    property string themeMode: "follow"
    property string pinnedTheme: ""

    readonly property var stubClientSpec: ({
        id: "braun", name: "Braun", family: "instrument", glyphStyle: "rounded", legacy: false,
        palette: { bg: "#161616", fg: "#f2f2f0", accent: "#ff5500", secondaryAccent: "#f0b01e",
                   borderActive: "#ff5500", borderInactive: "#404040", textMuted: "#8a8781",
                   success: "#248a3d", warning: "#d97706", error: "#dc2626" },
        widget: { renderer: "vector", font: "sans", pixelScale: 0, density: "comfortable", border: "line",
                  radius: 4, dither: "none", glow: 0, shades: [],
                  modules: ["telemetry", "curve", "displays", "automation", "weather", "forecast", "suntimes"] },
    })

    Theme {
        id: stubTheme

        spec: stage.stubClientSpec
    }

    SunReactorClient {
        id: offline

        socketPath: "/nonexistent/sunreactor-picker-test.sock"
    }

    Rectangle {
        anchors.fill: parent
        color: "#333333"
    }

    ThemeButton {
        id: button

        x: stage.width - width - 8
        y: 8
        theme: stubTheme
        client: offline
        themeList: stage.stubThemes
        themeMode: stage.themeMode
        pinnedId: stage.pinnedTheme

        onFollowRequested: stage.themeMode = "follow"
        onPinRequested: id => {
            stage.pinnedTheme = id;
            stage.themeMode = "pinned";
        }
    }

    TestCase {
        id: suite

        name: "ThemePicker"
        when: windowShown

        function picker() {
            return findChild(button, "themePicker");
        }

        function list() {
            return findChild(button, "themeList");
        }

        function followRow() {
            return findChild(button, "followRow");
        }

        /*! The delegate of one theme, scrolled into view. */
        function rowFor(id) {
            const view = list();
            for (let i = 0; i < view.count; ++i) {
                if (view.model.get(i).themeId === id) {
                    view.positionViewAtIndex(i, ListView.Contain);
                    return view.itemAtIndex(i);
                }
            }
            return null;
        }

        function init() {
            if (button.pickerOpen) {
                picker().close();
            }
            stage.themeMode = "follow";
            stage.pinnedTheme = "";
        }

        function test_oneClickOpensTheList() {
            verify(!button.pickerOpen, "closed at the start");
            mouseClick(button);
            verify(button.pickerOpen, "one click opens it");
            tryVerify(() => list().itemAtIndex(0) !== null, 2000, "rows are laid out");
            compare(list().count, stage.stubThemes.length);
            verify(followRow().visible, "Follow SunReactor sits at the top");
        }

        function test_clickingARowAppliesItAndKeepsTheListOpen() {
            mouseClick(button);
            verify(button.pickerOpen);
            tryVerify(() => rowFor("braun") !== null, 2000);
            mouseClick(rowFor("braun"));
            compare(stage.pinnedTheme, "braun", "the stub records the pin");
            compare(stage.themeMode, "pinned");
            verify(button.pinned, "the button shows the pin at once");
            verify(button.pickerOpen, "the list stays open");
            compare(rowFor("braun").Accessible.checked, true, "the applied row is checked");
            verify(!followRow().Accessible.checked, "Follow is no longer the choice");
        }

        function test_flippingThemesKeepsTheListOpen() {
            mouseClick(button);
            tryVerify(() => rowFor("nord") !== null, 2000);
            mouseClick(rowFor("nord"));
            compare(stage.pinnedTheme, "nord");
            mouseClick(rowFor("synthwave84"));
            compare(stage.pinnedTheme, "synthwave84");
            mouseClick(rowFor("handheld"));
            compare(stage.pinnedTheme, "handheld");
            verify(button.pickerOpen, "still open after three choices");
        }

        function test_clickingFollowAppliesFollowAndKeepsTheListOpen() {
            stage.themeMode = "pinned";
            stage.pinnedTheme = "nord";
            mouseClick(button);
            verify(button.pickerOpen);
            tryVerify(() => followRow().visible, 2000);
            mouseClick(followRow());
            compare(stage.themeMode, "follow", "the stub records follow");
            compare(stage.pinnedTheme, "nord", "the pin itself is kept for later");
            verify(!button.pinned);
            verify(button.pickerOpen, "the list stays open");
            verify(followRow().Accessible.checked, "Follow is checked");
        }

        function test_clickOutsideClosesTheList() {
            mouseClick(button);
            verify(button.pickerOpen);
            mouseClick(stage, 10, 470);
            verify(!button.pickerOpen, "a press outside closes it");
            compare(stage.pinnedTheme, "", "and chooses nothing");
        }

        function test_clickingTheButtonAgainClosesThenOpens() {
            mouseClick(button);
            verify(button.pickerOpen);
            mouseClick(button);
            verify(!button.pickerOpen, "the second click closes it and does not reopen it");
            mouseClick(button);
            verify(button.pickerOpen, "the next click opens it again");
        }

        function test_escapeClosesTheList() {
            mouseClick(button);
            verify(button.pickerOpen);
            keyClick(Qt.Key_Escape);
            verify(!button.pickerOpen, "Escape closes it");
        }

        function test_keyboardMovesAndApplies() {
            stage.themeMode = "pinned";
            stage.pinnedTheme = "nord";
            mouseClick(button);
            verify(button.pickerOpen);
            tryVerify(() => rowFor("nord") !== null, 2000);
            const start = list().currentIndex;
            keyClick(Qt.Key_Down);
            compare(list().currentIndex, start + 1, "Down moves the cursor");
            keyClick(Qt.Key_Return);
            compare(stage.pinnedTheme, list().model.get(start + 1).themeId, "Return applies the cursor row");
            verify(button.pickerOpen, "and the list stays open");
            keyClick(Qt.Key_Up);
            compare(list().currentIndex, start, "Up moves back");
            keyClick(Qt.Key_Space);
            compare(stage.pinnedTheme, list().model.get(start).themeId, "Space applies too");
            verify(button.pickerOpen);
        }

        function test_upFromTheFirstThemeReachesFollow() {
            mouseClick(button);
            verify(button.pickerOpen);
            tryVerify(() => list().itemAtIndex(0) !== null, 2000);
            list().currentIndex = 0;
            list().forceActiveFocus();
            keyClick(Qt.Key_Up);
            verify(followRow().activeFocus, "Up from the first theme goes to Follow");
            keyClick(Qt.Key_Return);
            compare(stage.themeMode, "follow");
            verify(button.pickerOpen);
        }

        function test_listScrollsToThePinnedRow() {
            const last = stage.stubThemes[stage.stubThemes.length - 1].id;
            stage.themeMode = "pinned";
            stage.pinnedTheme = last;
            mouseClick(button);
            verify(button.pickerOpen);
            tryVerify(() => list().contentY > list().originY + 1, 2000, "the list scrolled down from its top");
            tryVerify(() => {
                const row = list().itemAtIndex(list().currentIndex);
                return row !== null && row.y + row.height <= list().contentY + list().height + 1;
            }, 2000, "the pinned row is in view");
        }

        function test_popoverIsNarrowAndRightAligned() {
            mouseClick(button);
            verify(button.pickerOpen);
            const popup = picker();
            verify(popup.width <= Kirigami.Units.gridUnit * 11.5 + 0.5, "at most 11.5 grid units wide");
            verify(popup.width >= Kirigami.Units.gridUnit * 8, "not squeezed");
            fuzzyCompare(popup.x + popup.width, button.width, 1.0, "right-aligned under the button");
            verify(popup.height <= stage.height * 0.6 + 1, "at most 60% of the window");
        }
    }
}
