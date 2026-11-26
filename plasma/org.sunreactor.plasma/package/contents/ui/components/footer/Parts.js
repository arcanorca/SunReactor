.pragma library

/*
 * Footer parts registry - the same lookup as modules/Modules.js, so a family
 * or flagship skin can replace one footer part without touching the others.
 *
 * Part contract - every file listed here:
 *   required property PopupFooter footer   theme, client and the domain
 *   implicitWidth / implicitHeight         from its content
 * Parts read colours, fonts and metrics from footer.theme only.
 *
 * SKINS keys are "id:<theme id>" (wins) or a renderer name; values map a part
 * id to a path relative to ui/ (e.g. skins/pixel/FooterRefresh.qml).
 */

var FILES = {
    refresh: "components/footer/RefreshControl.qml"
};

/*! Left to right; empty by default since sunreactord syncs automatically and
    the panel context menu keeps an on-demand Update Now action. */
var ORDER = [];

var SKINS = {
    "pixel": {},
    "terminal": {},
    "vector": {},
    "id:cyberpunk": {
        refresh: "skins/cyberpunk/RefreshControl.qml"
    },
    "id:nothing": {
        refresh: "skins/nothing/RefreshControl.qml"
    },
    "id:xbox360": {
        refresh: "skins/xbox360/RefreshControl.qml"
    },
    "id:wii": {
        refresh: "skins/wii/RefreshControl.qml"
    },
    "id:ps3": {
        refresh: "skins/ps3/RefreshControl.qml"
    }
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
