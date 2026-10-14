#!/usr/bin/env python3
"""Observe the real SunReactor popup in a private nested plasmashell.

Started by run.sh inside a private dbus-run-session. It starts and owns the
children (kwin_wayland --virtual, the fake daemon, kactivitymanagerd,
plasmashell), keeps their Popen objects and stops them by PID.

The widget runs its own timed script (main.qml, SUNREACTOR_HARNESS_CONFIG):
open at openDelay, Brightness-Weather-Brightness-Weather tab steps 1.5 s apart,
then close and reopen. kwin_probe.js reports the plasmashell windows over
D-Bus on every change; the Probe keeps that timeline and the driver writes it
to OUT/steps.jsonl. check.py samples the timeline at the script's step times.

--panel top moves the panel to the top edge before the popup opens.
"""

import argparse
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import traceback

import dbus
import dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib

HERE = pathlib.Path(__file__).resolve().parent
WIDGET = HERE.parent.parent  # plasma/org.sunreactor.plasma
SCREEN_W, SCREEN_H = 1600, 1000
WIDGET_PLUGIN = "org.sunreactor.plasma"
STORED_HEIGHT, STORED_WIDTH = 494, 342  # the window is 8 px wider: 350, as plain
SERVICE = "org.sunreactor.nest.Driver"
APPLETSRC = "plasma-org.kde.plasma.desktop-appletsrc"
SRWINDOW = re.compile(r"SRWINDOW (\d+) (hidden|-?\d+ -?\d+ \d+ \d+)")
# The widget's script: open at 6 s, then tab steps and a reopen (see main.qml).
HARNESS_CONFIG = "open=1 openDelay=6000 tabs=weather,brightness,weather reopen=1"
TIMELINE = 10.0  # seconds recorded from the first popup window


def popup_of(windows):
    found = [w for w in windows if w["popup"] and not w["dock"] and w["w"] >= 200 and w["h"] >= 120]
    if not found:
        return None
    return {k: found[-1][k] for k in ("x", "y", "w", "h", "active")}


def panel_of(windows):
    for w in windows:
        if w["dock"]:
            return {k: w[k] for k in ("x", "y", "w", "h")}
    return None


class CpuSampler(threading.Thread):
    """CPU seconds of one thread (the main thread: tid == pid), every 0.5 s."""

    def __init__(self, pid):
        super().__init__(daemon=True)
        self.pid = pid
        self.samples = []  # [epoch seconds, CPU seconds]
        self.done = threading.Event()

    def run(self):
        hz = os.sysconf("SC_CLK_TCK")
        while not self.done.is_set():
            try:
                with open("/proc/%d/task/%d/stat" % (self.pid, self.pid)) as f:
                    raw = f.read()
            except OSError:
                break
            fields = raw[raw.rindex(")") + 2:].split()
            self.samples.append([round(time.time(), 3), (int(fields[11]) + int(fields[12])) / hz])
            self.done.wait(0.5)


class Probe(dbus.service.Object):
    """Receives the window snapshots from kwin_probe.js and keeps the timeline."""

    def __init__(self, bus, dump):
        dbus.service.Object.__init__(self, dbus.service.BusName(SERVICE, bus), "/Driver")
        self.snapshot = None
        self.dump = dump  # every distinct snapshot, for debugging
        self.events = []  # (monotonic time, popup or None, panel or None)

    @dbus.service.method(SERVICE, in_signature="s", out_signature="")
    def Snapshot(self, text):
        self.snapshot = json.loads(str(text))
        windows = self.snapshot.get("windows", [])
        self.dump.write(json.dumps({"t": round(time.monotonic(), 3), "windows": windows}) + "\n")
        self.dump.flush()
        popup, panel = popup_of(windows), panel_of(windows)
        key = (popup and {k: popup[k] for k in ("x", "y", "w", "h")}, panel)
        if not self.events or self.events[-1][3] != key:
            self.events.append((time.monotonic(), popup, panel, key))

    def current(self):
        return (self.events[-1][1], self.events[-1][2]) if self.events else (None, None)


class Nest:
    def __init__(self, args):
        self.args = args
        self.out = pathlib.Path(args.out)
        self.rt = os.environ["XDG_RUNTIME_DIR"]
        self.display = "srn-wl"
        self.config_dir = pathlib.Path(os.environ["XDG_CONFIG_HOME"])
        self.children = []
        self.seq = 0
        self.bus = dbus.SessionBus()
        self.probe = Probe(self.bus, (self.out / "windows.jsonl").open("w"))
        self.shell = None
        self.sampler = None
        self.kwin = None
        self.applet = "panels()[0].widgets('%s')[0]" % WIDGET_PLUGIN
        self.log = (self.out / "steps.jsonl").open("w")

    # --- processes -----------------------------------------------------------

    def env(self, **extra):
        env = {k: v for k, v in os.environ.items() if k != "WAYLAND_DISPLAY"}
        env.update(extra)
        return env

    def spawn(self, name, argv, env):
        log = (self.out / (name + ".log")).open("w")
        proc = subprocess.Popen(argv, env=env, stdout=log, stderr=subprocess.STDOUT)
        self.children.append((name, proc))
        return proc

    def stop(self):
        for _, proc in reversed(self.children):
            if proc.poll() is None:
                proc.terminate()
        deadline = time.monotonic() + 8.0
        for _, proc in reversed(self.children):
            while proc.poll() is None and time.monotonic() < deadline:
                time.sleep(0.1)
            if proc.poll() is None:
                proc.kill()
                proc.wait()

    # --- waiting -------------------------------------------------------------

    def pump(self, seconds):
        ctx = GLib.MainContext.default()
        end = time.monotonic() + seconds
        while True:
            while ctx.pending():
                ctx.iteration(False)
            if time.monotonic() >= end:
                return
            time.sleep(0.02)

    def wait(self, predicate, timeout, what):
        deadline = time.monotonic() + timeout
        while True:
            self.pump(0.2)
            try:
                if predicate():
                    return
            except dbus.DBusException:
                pass
            if time.monotonic() > deadline:
                raise RuntimeError("timed out waiting for " + what)

    # --- session setup -------------------------------------------------------

    def start_compositor(self):
        sock = os.path.join(self.rt, self.display)
        self.spawn("kwin", ["kwin_wayland", "--virtual", "--socket", self.display,
                            "--width", str(SCREEN_W), "--height", str(SCREEN_H),
                            "--no-lockscreen", "--no-global-shortcuts"], self.env())
        self.wait(lambda: os.path.exists(sock), 30, "the private Wayland socket")
        self.wait(lambda: self.bus.name_has_owner("org.kde.KWin"), 30, "KWin on the private bus")
        self.kwin = dbus.Interface(self.bus.get_object("org.kde.KWin", "/Scripting"),
                                   "org.kde.kwin.Scripting")
        self.kwin.loadScript(str(HERE / "kwin_probe.js"))
        self.kwin.start()
        self.wait(lambda: self.probe.snapshot is not None, 30, "the KWin probe")

    def start_daemon(self):
        sock = pathlib.Path(self.rt) / "sunreactor" / "control.sock"
        sock.parent.mkdir(parents=True, exist_ok=True)
        self.spawn("daemon", [sys.executable, str(WIDGET / "tools" / "fake_daemon.py"),
                              "--socket", str(sock), "--theme", self.args.theme,
                              "--scenario", "default",
                              "--log", str(self.out / "requests.log")], self.env())
        self.wait(sock.exists, 20, "the fake daemon socket")

    def start_plasma(self):
        self.spawn("activity", ["/usr/lib/kactivitymanagerd"],
                   self.env(QT_QPA_PLATFORM="offscreen"))
        shell = ["plasmashell", "--no-respawn"]
        harness = (HARNESS_CONFIG + " " + self.args.config).strip()
        if self.args.gdb:
            # gdb is plasmashell's parent, so it may attach (ptrace_scope=1). SIGINT
            # stops the process and the batch commands print the stacks.
            shell = ["gdb", "-batch", "-ex", "set pagination off", "-ex", "run",
                     "-ex", "thread apply all bt 30", "--args"] + shell
        self.gdb_proc = self.spawn("plasmashell", shell,
                                   self.env(WAYLAND_DISPLAY=self.display, QT_QPA_PLATFORM="wayland",
                                            QT_FORCE_STDERR_LOGGING="1", SUNREACTOR_HARNESS="1",
                                            SUNREACTOR_HARNESS_CONFIG=harness))
        self.wait(lambda: self.bus.name_has_owner("org.kde.plasmashell"), 120, "plasmashell")
        self.shell = dbus.Interface(self.bus.get_object("org.kde.plasmashell", "/PlasmaShell"),
                                    "org.kde.PlasmaShell")
        self.wait(lambda: self.query("panels().length") == "1", 120, "the default panel")

    def add_widget(self):
        self.js("panels()[0].addWidget('%s'); 'ok'" % WIDGET_PLUGIN)
        self.wait(lambda: self.query("panels()[0].widgets('%s').length" % WIDGET_PLUGIN) == "1",
                  30, "the widget in the panel")

    def store_size(self):
        self.js("var w = %s; w.writeConfig('popupHeight', %d); w.writeConfig('popupWidth', %d); 'ok'"
                % (self.applet, STORED_HEIGHT, STORED_WIDTH))

    def js(self, code):
        """Run plasmashell JavaScript; every call is logged to js.log."""
        if self.shell is None:
            return None
        try:
            result = str(self.shell.evaluateScript(code))
        except dbus.DBusException as error:
            result = "DBusException: " + str(error).splitlines()[0]
        with (self.out / "js.log").open("a") as log:
            log.write(json.dumps({"code": code, "result": result}) + "\n")
        return result

    def query(self, expression):
        """Value of a shell expression as a string, or None. evaluateScript
        returns what the script print()s, so the answer comes back directly."""
        answer = self.js("print(String(%s))" % expression)
        return answer.strip() if answer else None

    def config_values(self):
        def read(key):
            value = self.query("%s.readConfig('%s')" % (self.applet, key))
            return value if value not in (None, "", "undefined") else None
        return {"popupHeight": read("popupHeight"), "popupWidth": read("popupWidth")}

    def set_location(self, location):
        self.js("panels()[0].location = '%s'; 'ok'" % location)
        if location == "top":
            self.wait(lambda: self.probe.current()[1] is not None and self.probe.current()[1]["y"] == 0,
                      15, "the panel at the top edge")
        else:
            self.wait(lambda: self.probe.current()[1] is not None
                      and self.probe.current()[1]["y"] + self.probe.current()[1]["h"] == SCREEN_H,
                      15, "the panel at the bottom edge")

    # --- the observation -----------------------------------------------------

    def observe(self):
        """Record the popup's window timeline from the widget's SRWINDOW lines
        (main.qml's harness probe; KWin does not list a Plasma popup). The panel
        comes from KWin, the dock window."""
        log = self.out / "plasmashell.log"

        def windows():
            text = log.read_text(errors="replace") if log.exists() else ""
            return [(int(ms), rest) for ms, rest in SRWINDOW.findall(text)]

        self.wait(lambda: any(rest != "hidden" for _, rest in windows()), 90,
                  "the widget's popup window")
        first = next(ms for ms, rest in windows() if rest != "hidden")
        if self.args.gdb:
            self.pump(15.0)
            self.gdb_proc.send_signal(signal.SIGINT)
            self.pump(10.0)
        self.pump(TIMELINE)
        panel = self.probe.current()[1]
        events = []
        for ms, rest in windows():
            if ms < first - 500:
                continue
            popup = None if rest == "hidden" else dict(zip(("x", "y", "w", "h"), map(int, rest.split())))
            events.append({"t": round((ms - first) / 1000, 3), "popup": popup, "panel": panel})
        self.emit({"step": "session", "location": self.location, "first_ms": first, "events": events})

    def emit(self, record):
        record = dict(record, theme=self.args.theme, font=self.args.font,
                      size=self.args.size, panel=self.args.panel)
        self.log.write(json.dumps(record) + "\n")
        self.log.flush()

    # --- the scenario --------------------------------------------------------

    def run(self):
        self.location = "bottom"
        self.start_compositor()
        self.start_daemon()
        self.start_plasma()
        self.sampler = CpuSampler(self.gdb_proc.pid)
        self.sampler.start()
        self.add_widget()
        if self.args.size == "stored":
            self.store_size()
        self.wait(lambda: self.probe.current()[1] is not None, 60, "the panel window")
        if self.args.panel == "top":
            self.set_location("top")
            self.location = "top"
        self.emit({"step": "ready", "config": self.config_values(), "panel": self.probe.current()[1]})
        self.observe()

    def finish(self):
        if self.sampler is not None:
            self.sampler.done.set()
            self.emit({"step": "cpu", "samples": self.sampler.samples})
        self.log.close()
        src = self.config_dir / APPLETSRC
        if src.exists():
            shutil.copy(src, self.out / "appletsrc")


def main():
    ap = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    ap.add_argument("--theme", required=True)
    ap.add_argument("--font", required=True, choices=["default", "inter12"])
    ap.add_argument("--size", required=True, choices=["plain", "stored"])
    ap.add_argument("--panel", required=True, choices=["bottom", "top"])
    ap.add_argument("--out", required=True)
    ap.add_argument("--config", default="",
                    help="more harness config keys, 'key=value key=value', added to the widget's script")
    ap.add_argument("--gdb", action="store_true",
                    help="run plasmashell under gdb and print every thread's stack 15 s after the popup opens")
    args = ap.parse_args()

    DBusGMainLoop(set_as_default=True)
    nest = Nest(args)
    status = 0
    try:
        nest.run()
    except Exception:
        traceback.print_exc()
        status = 1
    finally:
        nest.stop()
        nest.finish()
    return status


if __name__ == "__main__":
    sys.exit(main())
