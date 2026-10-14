// SunReactor nest: reports the plasmashell windows to the driver (driver.py).
// Loaded only into the private KWin of a nest session. Every 100 ms it sends
// a snapshot of the plasmashell windows over D-Bus when anything changed.
// Read-only: it moves and focuses nothing.
const SERVICE = "org.sunreactor.nest.Driver";
const OBJECT = "/Driver";
const INTERFACE = "org.sunreactor.nest.Driver";
let last = "";

function snapshot() {
    const out = [];
    const wins = workspace.windowList();
    for (let i = 0; i < wins.length; i++) {
        const w = wins[i];
        if (String(w.resourceClass) !== "plasmashell" && !w.popupWindow && !w.dock) {
            continue;
        }
        const g = w.frameGeometry;
        out.push({
            id: String(w.internalId),
            cls: String(w.resourceClass),
            caption: String(w.caption),
            dock: !!w.dock,
            popup: !!w.popupWindow,
            transient: !!w.transient,
            desktop: !!w.desktopWindow,
            active: !!w.active,
            x: g.x,
            y: g.y,
            w: g.width,
            h: g.height,
        });
    }
    return JSON.stringify({ windows: out });
}

const timer = new QTimer();
timer.interval = 100;
timer.timeout.connect(function () {
    const text = snapshot();
    if (text !== last) {
        last = text;
        callDBus(SERVICE, OBJECT, INTERFACE, "Snapshot", text);
    }
});
timer.start();
