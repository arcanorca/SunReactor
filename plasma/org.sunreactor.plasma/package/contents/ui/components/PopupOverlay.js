.pragma library

/*
 * Tracks floating sub-menus/popups (such as ModeDurationMenu or ThemePicker)
 * inside the Plasma popup so auto-close on window deactivation waits while a
 * menu is open.
 */

function sync(item, savedHost, registered, active) {
    if (active === registered) {
        return { host: savedHost, registered: registered };
    }
    var target = savedHost;
    if (!target) {
        var cur = item;
        while (cur) {
            if (typeof cur.activePopupMenus === "number") {
                target = cur;
                break;
            }
            cur = cur.parent;
        }
    }
    if (target) {
        target.activePopupMenus = Math.max(0, target.activePopupMenus + (active ? 1 : -1));
    }
    return { host: active ? target : null, registered: active };
}
