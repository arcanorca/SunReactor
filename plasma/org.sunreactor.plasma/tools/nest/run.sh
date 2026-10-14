#!/usr/bin/env bash
# Nested real-popup session for the SunReactor Plasma widget.
#
#   tools/nest/run.sh [--theme NAME] [--font default|inter12] [--size plain|stored]
#                     [--panel bottom|top] [--out DIR] [--keep]
#
# One private session per call: kwin_wayland --virtual (1600x1000) and
# plasmashell on a private D-Bus session, the fake daemon on a private socket,
# the working-tree package installed into the private data dir, the widget in
# the default panel. The widget runs its own timed script (main.qml, the nest
# harness: open, Brightness-Weather-Brightness-Weather, close, reopen) because
# nothing outside the widget can open or page it here. driver.py records the
# popup and panel geometry KWin reports, to DIR/steps.jsonl. --size stored
# writes popupHeight/popupWidth into the applet's config before the first open.
# DIR defaults to build/nest/<theme>-<font>-<size>-<panel> (build/ is ignored).
#
# Isolation: env -i, a private HOME and XDG_* tree under a mktemp directory, a
# private runtime dir with the Wayland socket and the daemon socket, and a
# private D-Bus. Only processes started by the driver are signalled, by PID.
# Needs the built plugin (build/qml) and the Plasma 6 binaries on PATH.
set -u

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WIDGET="$(cd "$HERE/../.." && pwd)"
THEME=braun
FONT=default
SIZE=plain
PANEL=bottom
PACKAGE="$WIDGET/package"
GDB=0
EXTRA=""
OUT=""
KEEP=0

while [ $# -gt 0 ]; do
    case "$1" in
        --theme) THEME="$2"; shift 2 ;;
        --font) FONT="$2"; shift 2 ;;
        --size) SIZE="$2"; shift 2 ;;
        --panel) PANEL="$2"; shift 2 ;;
        --package) PACKAGE="$2"; shift 2 ;;
        --gdb) GDB=1; shift ;;
        --config) EXTRA="$2"; shift 2 ;;
        --out) OUT="$2"; shift 2 ;;
        --keep) KEEP=1; shift ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
        *) echo "run.sh: unknown argument: $1" >&2; exit 2 ;;
    esac
done
case "$FONT" in default|inter12) ;; *) echo "run.sh: --font is default or inter12" >&2; exit 2 ;; esac
case "$SIZE" in plain|stored) ;; *) echo "run.sh: --size is plain or stored" >&2; exit 2 ;; esac
case "$PANEL" in bottom|top) ;; *) echo "run.sh: --panel is bottom or top" >&2; exit 2 ;; esac
[ -d "$WIDGET/build/qml/org" ] || { echo "run.sh: $WIDGET/build/qml is missing; build the plugin first" >&2; exit 2; }
for tool in kwin_wayland plasmashell kpackagetool6 dbus-run-session python3 /usr/lib/kactivitymanagerd; do
    command -v "$tool" >/dev/null 2>&1 || [ -x "$tool" ] || { echo "run.sh: $tool not found" >&2; exit 2; }
done
[ -n "$OUT" ] || OUT="$WIDGET/build/nest/$THEME-$FONT-$SIZE-$PANEL"
mkdir -p "$OUT" || exit 2

TMP_ROOT="${TMPDIR:-/tmp}"
# Safety: nothing here may touch the user's real runtime dir /run/user/<uid>, where
# sunreactord keeps its socket. The private temp dir must not lie under it.
case "$(realpath -m -- "$TMP_ROOT")" in
    /run/user/*) echo "run.sh: refusing: TMPDIR=$TMP_ROOT is under /run/user" >&2; exit 2 ;;
esac
RUNDIR="$(mktemp -d "$TMP_ROOT/srnest.XXXXXX")" || exit 2
cleanup() {
    if [ "$KEEP" = 1 ]; then
        echo "run.sh: kept $RUNDIR"
    else
        case "$RUNDIR" in "$TMP_ROOT"/srnest.*) rm -rf -- "$RUNDIR" ;; esac
    fi
}
trap cleanup EXIT

RT="$RUNDIR/rt"
X="$RUNDIR/xdg"
# Safety: the runtime dir this session exports must lie under the private temp dir
# and must not resolve under /run/user; any other value aborts before anything runs.
for dir in "$RT" "$RUNDIR"; do
    case "$(realpath -m -- "$dir")" in
        /run/user/*) echo "run.sh: refusing: $dir resolves under /run/user" >&2; exit 2 ;;
    esac
done
case "$(realpath -m -- "$RT")" in
    "$(realpath -m -- "$RUNDIR")"/*) ;;
    *) echo "run.sh: refusing: the runtime dir is not under the private temp dir" >&2; exit 2 ;;
esac
mkdir -m 700 "$RT" || exit 2
mkdir -p "$RT/sunreactor" "$X/home" "$X/config" "$X/data" "$X/cache" "$X/state" || exit 2
if [ "$FONT" = inter12 ]; then
    {
        echo "[General]"
        echo "font=Inter,12,-1,5,400,0,0,0,0,0"
        echo "menuFont=Inter,12,-1,5,400,0,0,0,0,0"
        echo "toolBarFont=Inter,12,-1,5,400,0,0,0,0,0"
    } > "$X/config/kdeglobals"
fi

NEST_ENV=(
    HOME="$X/home" PATH=/usr/bin:/usr/sbin:/bin LANG=en_US.UTF-8 TZ=Europe/Istanbul
    XDG_RUNTIME_DIR="$RT" XDG_CONFIG_HOME="$X/config" XDG_DATA_HOME="$X/data"
    XDG_CACHE_HOME="$X/cache" XDG_STATE_HOME="$X/state"
    DBUS_SYSTEM_BUS_ADDRESS="unix:path=$RT/no-system-bus"
    XDG_CURRENT_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true
    QT_QPA_PLATFORMTHEME=KDE LIBGL_ALWAYS_SOFTWARE=1 PYTHONDONTWRITEBYTECODE=1
    QML_IMPORT_PATH="$WIDGET/build/qml"
)

# 1. The working-tree package, into the private data dir only.
env -i "${NEST_ENV[@]}" kpackagetool6 --type Plasma/Applet --install "$PACKAGE" \
    > "$OUT/install.log" 2>&1 || { echo "run.sh: install failed, see $OUT/install.log" >&2; exit 1; }

# 2. The scenario, on a private D-Bus session.
env -i "${NEST_ENV[@]}" dbus-run-session -- python3 "$HERE/driver.py" \
    --theme "$THEME" --font "$FONT" --size "$SIZE" --panel "$PANEL" --out "$OUT" \
    $([ "$GDB" = 1 ] && echo --gdb) ${EXTRA:+--config "$EXTRA"}
status=$?
echo "run.sh: theme=$THEME font=$FONT size=$SIZE panel=$PANEL exit=$status out=$OUT"
exit $status
