.pragma library

/*
 * Footer parts registry - the same lookup as modules/Modules.js, so a family
 * or flagship skin can replace one footer part without touching the others.
 *
 * Part contract - every file listed here:
 *   required property PopupFooter footer   theme, client, theme-choice state
 *                                          and the follow/pin requests
 *   implicitWidth / implicitHeight         from its content
 * Parts read colours, fonts and metrics from footer.theme only.
 *
 * SKINS keys are "id:<theme id>" (wins) or a renderer name; values map a part
 * id to a path relative to ui/ (e.g. skins/pixel/FooterThemeChip.qml).
 */

var FILES = {
    pause: "components/footer/PauseControl.qml",
    theme: "components/footer/ThemeChip.qml",
    refresh: "components/footer/RefreshControl.qml"
};

/*! Left to right; a null entry is the flexible gap. */
var ORDER = ["pause", null, "theme", "refresh"];

var SKINS = {
    "pixel": {},
    "terminal": {},
    "vector": {}
};

function source(part, themeId, renderer) {
    var keys = ["id:" + themeId, renderer];
    for (var i = 0; i < keys.length; ++i) {
        var skin = SKINS[keys[i]];
        if (skin && skin[part]) {
            return skin[part];
        }
    }
    return FILES[part] || "";
}
