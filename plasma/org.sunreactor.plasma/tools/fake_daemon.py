#!/usr/bin/env python3
"""Stand-in for sunreactord's control socket, for headless widget screenshots.

Speaks the real protocol: one JSON request line per connection, one JSON
response line back, then the connection closes. It never touches hardware and
refuses to bind the real daemon socket.

  --theme ID     status.theme comes from fixtures/themes.json
  --old          behave like a daemon that predates the theme registry:
                 legacy flat status.theme only, and list_themes / get_theme /
                 day_curve answer `invalid_request` exactly like serde would
  --scenario S   default | manual | paused | single | unknown | noweather |
                 triple (3rd display unavailable) | longnames (+ override) |
                 offline (stops answering after two status replies)
  --live         report the real clock instead of the fixed demo moment
  --theme-warning TEXT   status.theme_warning, as when a user theme is broken
  --log FILE     append every request as one JSON line (the harness waits on it)

Commands (set_override, suspend, set_monitor_limits, ...) are acknowledged and
applied to the in-memory state so the widget can be exercised interactively.
"""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import json
import math
import os
import signal
import socket
import sys
import time
from pathlib import Path
from zoneinfo import ZoneInfo

PROTOCOL_VERSION = 1
HERE = Path(__file__).resolve().parent
TZ = ZoneInfo("Europe/Istanbul")
# The demo moment: an October afternoon, sun well up, clouds dimming a little.
DEMO_LOCAL = dt.datetime(2026, 10, 13, 14, 30, tzinfo=TZ)
LEGACY_THEME_KEYS = ("name", "accent", "secondary_accent", "bg", "fg", "text_muted")


def real_socket_paths() -> set[str]:
    uid = os.getuid()
    paths = {f"/run/user/{uid}/sunreactor/control.sock"}
    original = os.environ.get("SUNREACTOR_REAL_XDG_RUNTIME_DIR")
    if original:
        paths.add(os.path.join(original, "sunreactor", "control.sock"))
    return {os.path.realpath(p) for p in paths}


def local_day(now_epoch: int) -> tuple[dt.datetime, int, int]:
    local = dt.datetime.fromtimestamp(now_epoch, TZ)
    midnight = local.replace(hour=0, minute=0, second=0, microsecond=0)
    start = int(midnight.timestamp())
    end = int((midnight + dt.timedelta(days=1)).timestamp())
    return midnight, start, end


def sun_times(midnight: dt.datetime) -> dict[str, int]:
    def at(h: int, m: int) -> int:
        return int(midnight.replace(hour=h, minute=m).timestamp())

    rise, sset = at(7, 13), at(18, 38)
    return {
        "dawn": at(6, 46),
        "sunrise": rise,
        "solar_noon": rise + (sset - rise) // 2,
        "sunset": sset,
        "dusk": at(19, 4),
    }


def daylight_factor(t: int, sun: dict[str, int]) -> float:
    """A smooth stand-in for the solar policy: 0 at night, 1 at noon."""
    if t <= sun["dawn"] or t >= sun["dusk"]:
        return 0.0
    if t < sun["sunrise"]:
        return 0.06 * (t - sun["dawn"]) / (sun["sunrise"] - sun["dawn"])
    if t > sun["sunset"]:
        return 0.06 * (sun["dusk"] - t) / (sun["dusk"] - sun["sunset"])
    phase = (t - sun["sunrise"]) / (sun["sunset"] - sun["sunrise"])
    return 0.06 + 0.94 * math.sin(math.pi * phase) ** 1.4


class FakeDaemon:
    def __init__(self, args: argparse.Namespace) -> None:
        self.args = args
        self.themes = json.loads((HERE / "fixtures" / "themes.json").read_text())
        self.themes.pop("_comment", None)
        if args.theme not in self.themes:
            raise SystemExit(f"unknown fixture theme {args.theme!r}; have {sorted(self.themes)}")
        self.theme_id = args.theme
        self.suspended_until: int | None = None
        self.suspended = False
        self.global_override: tuple[int, int | None] | None = None
        self.monitors = [
            {"logical_id": "mi-monitor", "backend": "ddc", "enabled": True, "override_percent": None,
             "override_until": None, "min_pct": 1, "max_pct": 50, "topology": "present"},
            {"logical_id": "len-p24h-20-v305ptda", "backend": "ddc", "enabled": True,
             "override_percent": None, "override_until": None, "min_pct": 5, "max_pct": 60,
             "topology": "present"},
        ]
        scenario = args.scenario
        if scenario == "single":
            self.monitors = self.monitors[:1]
        if scenario in ("triple", "longnames"):
            # A third display that is not answering, with a name too long for any row.
            self.monitors.append(
                {"logical_id": "dell-ultrasharp-u2723qe-left-portrait-hdmi2", "backend": "ddc", "enabled": True,
                 "override_percent": None, "override_until": None, "min_pct": 10, "max_pct": 80,
                 "topology": "temporarily_unavailable"})
        if scenario == "longnames":
            self.monitors[0]["logical_id"] = "samsung-odyssey-g9-neo-49-ultrawide-dp1"
            self.monitors[1]["override_percent"] = 33
            self.monitors[1]["override_until"] = self.now() + 5400
        if scenario == "manual":
            self.monitors[1]["override_percent"] = 70
            self.monitors[1]["override_until"] = self.now() + 3600
        if scenario == "paused":
            self.suspended = True
            self.suspended_until = self.now() + 2 * 3600

    # -- clock -------------------------------------------------------------
    def now(self) -> int:
        return int(time.time()) if self.args.live else int(DEMO_LOCAL.timestamp())

    # -- policy stand-in ---------------------------------------------------
    def weather_multiplier(self, t: int, sun: dict[str, int]) -> float:
        if self.args.scenario == "noweather":
            return 1.0
        return 0.88 if sun["sunrise"] <= t <= sun["sunset"] else 1.0

    def target(self, monitor: dict, t: int, sun: dict[str, int], weather: bool = True) -> int:
        f = daylight_factor(t, sun)
        lo, hi = monitor["min_pct"], monitor["max_pct"]
        pct = lo + (hi - lo) * f
        if weather:
            pct = lo + (pct - lo) * self.weather_multiplier(t, sun)
        return int(round(pct))

    def day_curve(self) -> dict:
        now = self.now()
        midnight, start, end = local_day(now)
        sun = sun_times(midnight)
        step = 10 * 60
        monitors = []
        for m in self.monitors:
            samples = [
                {"t": t, "pct": self.target(m, t, sun), "pct_no_weather": self.target(m, t, sun, False)}
                for t in range(start, end + 1, step)
            ]
            half = m["min_pct"] + (m["max_pct"] - m["min_pct"]) // 2
            rise_half = next((s for s in samples if s["t"] > sun["sunrise"] and s["pct_no_weather"] >= half), None)
            set_half = next((s for s in reversed(samples) if s["t"] < sun["sunset"] and s["pct_no_weather"] >= half), None)
            milestones = [{"id": "rise_start", "label": "Dawn", "t": sun["dawn"], "pct": self.target(m, sun["dawn"], sun)}]
            if rise_half:
                milestones.append({"id": "rise_50", "label": "Rise 50%", "t": rise_half["t"], "pct": rise_half["pct"]})
            milestones.append({"id": "peak", "label": "Peak", "t": sun["solar_noon"], "pct": self.target(m, sun["solar_noon"], sun)})
            if set_half:
                milestones.append({"id": "set_50", "label": "Set 50%", "t": set_half["t"], "pct": set_half["pct"]})
            milestones.append({"id": "night_floor", "label": "Dusk", "t": sun["dusk"], "pct": self.target(m, sun["dusk"], sun)})
            monitors.append({"monitor_id": m["logical_id"], "min_pct": m["min_pct"], "max_pct": m["max_pct"],
                             "samples": samples, "milestones": milestones})
        return {
            "date": midnight.date().isoformat(),
            "step_minutes": 10,
            "generated_at_epoch_s": now,
            "weather_multiplier": self.weather_multiplier(now, sun),
            "sun": sun,
            "monitors": monitors,
        }

    # -- status --------------------------------------------------------------
    def theme_status(self, theme_id: str) -> dict:
        theme = copy.deepcopy(self.themes[theme_id])
        pal = theme["palette"]
        flat = {"name": theme["name"], "accent": pal["accent"], "secondary_accent": pal["secondary_accent"],
                "bg": pal["bg"], "fg": pal["fg"], "text_muted": pal["text_muted"]}
        if self.args.old:
            return flat
        return {**flat, **theme}

    def status(self) -> dict:
        now = self.now()
        midnight, _, _ = local_day(now)
        sun = sun_times(midnight)
        if self.suspended and self.suspended_until and now >= self.suspended_until:
            self.suspended, self.suspended_until = False, None
        monitors = []
        per_monitor_until = None
        for i, m in enumerate(self.monitors):
            applied = self.target(m, now, sun)
            if self.args.scenario == "unknown" and i == 1:
                applied = None
            override = m["override_percent"]
            if self.global_override is not None:
                override = self.global_override[0]
            if m["override_until"]:
                per_monitor_until = max(per_monitor_until or 0, m["override_until"])
            monitors.append({
                "logical_id": m["logical_id"], "backend": m["backend"], "enabled": m["enabled"],
                "override_percent": override,
                "last_applied_percent": override if override is not None else applied,
                "last_applied_at_epoch_s": now - 30, "backoff_until_epoch_s": None,
                "topology": m["topology"], "min_pct": m["min_pct"], "max_pct": m["max_pct"],
            })
        weather = None
        if self.args.scenario != "noweather":
            forecast = []
            conditions = ["partly_cloudy", "cloudy", "rain", "cloudy", "partly_cloudy", "clear", "clear", "clear"]
            for h in range(1, 9):
                t = now - now % 3600 + h * 3600
                forecast.append({"dt_epoch_s": t, "temperature": round(15.2 - 0.6 * h, 1),
                                 "condition": conditions[h - 1],
                                 "day_phase": "day" if sun["sunrise"] <= t <= sun["sunset"] else "night",
                                 "cloud_cover_percent": 40 + 5 * h, "precipitation_percent": 10 * (h % 4)})
            weather = {
                "enabled": True, "state": "ready", "active": True, "stale": False, "provider": "open_meteo",
                "fetched_at_epoch_s": now - 600, "valid_at_epoch_s": now - 600, "source_kind": "current",
                "last_refresh_attempt_epoch_s": now - 600, "next_refresh_at_epoch_s": now + 1200,
                "consecutive_failures": 0, "last_error": None, "cloud_cover_percent": 40,
                "temperature": 15.2, "condition": "partly_cloudy", "condition_description": "partly cloudy",
                "day_phase": "day" if sun["sunrise"] <= now <= sun["sunset"] else "night",
                "forecast": forecast, "multiplier": self.weather_multiplier(now, sun),
                "details": {"feels_like": 13.9, "humidity_percent": 62, "pressure_hpa": 1016,
                            "wind_speed_mps": 3.4, "visibility_m": 10000, "precipitation_percent": 10,
                            "air_quality": {"us_aqi": 42}},
            }
        manual = self.global_override is not None or any(m["override_percent"] is not None for m in self.monitors)
        noon = sun["solar_noon"]
        elevation = 38.0 * math.cos(math.pi * (now - noon) / (sun["sunset"] - sun["sunrise"]))
        extra = {}
        if self.args.theme_warning and not self.args.old:
            extra["theme_warning"] = self.args.theme_warning
        return {
            **extra,
            "daemon_alive": True, "config_path": "/fake/config.toml", "tick_seconds": 60, "dry_run": True,
            "suspended": self.suspended, "desktop_idle_dimmed": False,
            "suspend_until_epoch_s": self.suspended_until if self.suspended else None,
            "manual_override_active": manual,
            "per_monitor_override_until_epoch_s": per_monitor_until,
            "global_override_percent": self.global_override[0] if self.global_override else None,
            "global_override_until_epoch_s": self.global_override[1] if self.global_override else None,
            "configured_monitors": len(self.monitors), "stateful_monitors": len(self.monitors),
            "weather": weather, "monitors": monitors, "solar_elevation": round(elevation, 2),
            "now_epoch_s": now, "sunrise_epoch_s": sun["sunrise"], "sunset_epoch_s": sun["sunset"],
            "lunar_phase": None, "theme": self.theme_status(self.theme_id),
        }

    # -- dispatch ------------------------------------------------------------
    @staticmethod
    def envelope(kind: str, **fields) -> dict:
        return {"version": PROTOCOL_VERSION, "response": kind, **fields}

    def error(self, code: str, message: str) -> dict:
        return self.envelope("error", code=code, message=message)

    def unknown_variant(self, name: str) -> dict:
        return self.error("invalid_request",
                          f"failed to decode IPC request: unknown variant `{name}`, expected one of "
                          "`status`, `suspend`, `resume`, `idle_dim`, `idle_wake`, `set_override`, "
                          "`set_monitor_limits`, `clear_override`, `reload_config`, `refresh_weather`, "
                          "`ping`, `run_once`, `external_brightness_change`")

    def handle(self, req: dict) -> dict:
        if req.get("version") != PROTOCOL_VERSION:
            return self.error("unsupported_version", f"unsupported IPC protocol version {req.get('version')}")
        name = req.get("request")
        now = self.now()
        if name == "status":
            return self.envelope("status", status=self.status())
        if name == "ping":
            return self.envelope("pong", message="pong")
        if name in ("list_themes", "get_theme", "day_curve") and self.args.old:
            return self.unknown_variant(name)
        if name == "list_themes":
            themes = []
            for tid in self.themes:
                t = self.themes[tid]
                p = t["palette"]
                themes.append({"id": tid, "name": t["name"], "family": t["family"], "bg": p["bg"],
                               "fg": p["fg"], "accent": p["accent"], "secondary_accent": p["secondary_accent"]})
            return self.envelope("themes", themes=themes)
        if name == "get_theme":
            tid = str(req.get("id", ""))
            if tid not in self.themes:
                return self.error("invalid_request", f"unknown theme `{tid}`")
            return self.envelope("theme", theme=self.theme_status(tid))
        if name == "day_curve":
            return self.envelope("day_curve", day_curve=self.day_curve())
        if name == "set_override":
            pct = int(req.get("percent", 0))
            minutes = req.get("minutes")
            until = now + int(minutes) * 60 if minutes else None
            mid = req.get("monitor_id")
            if mid is None:
                self.global_override = (pct, until)
            for m in self.monitors:
                if m["logical_id"] == mid:
                    m["override_percent"], m["override_until"] = pct, until
            return self.envelope("ack", message="override set")
        if name == "clear_override":
            mid = req.get("monitor_id")
            if mid is None:
                self.global_override = None
                if not req.get("global"):
                    for m in self.monitors:
                        m["override_percent"], m["override_until"] = None, None
            for m in self.monitors:
                if m["logical_id"] == mid:
                    m["override_percent"], m["override_until"] = None, None
            return self.envelope("ack", message="override cleared")
        if name == "set_monitor_limits":
            for m in self.monitors:
                if m["logical_id"] == req.get("monitor_id"):
                    m["min_pct"], m["max_pct"] = int(req["min_pct"]), int(req["max_pct"])
            return self.envelope("ack", message="limits set")
        if name == "suspend":
            minutes = req.get("minutes")
            self.suspended = True
            self.suspended_until = now + int(minutes) * 60 if minutes else None
            return self.envelope("ack", message="suspended")
        if name == "resume":
            self.suspended, self.suspended_until = False, None
            return self.envelope("ack", message="resumed")
        if name == "run_once":
            return self.envelope("run_once", message="ok", run_once={
                "tick_duration_ms": 3, "monitors_evaluated": len(self.monitors), "writes_attempted": 0,
                "writes_skipped": len(self.monitors), "writes_succeeded": 0, "writes_failed": 0})
        if name in ("refresh_weather", "reload_config", "idle_dim", "idle_wake", "external_brightness_change"):
            return self.envelope("ack", message="ok")
        return self.unknown_variant(str(name))


def serve(args: argparse.Namespace) -> None:
    path = os.path.abspath(args.socket)
    if os.path.realpath(path) in real_socket_paths():
        raise SystemExit(f"refusing to bind the real daemon socket {path}")
    if os.path.exists(path):
        probe = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            probe.connect(path)
            raise SystemExit(f"something is already listening on {path}; not touching it")
        except (ConnectionRefusedError, FileNotFoundError):
            os.unlink(path)  # a stale socket file we can replace
        finally:
            probe.close()
    os.makedirs(os.path.dirname(path), mode=0o700, exist_ok=True)

    daemon = FakeDaemon(args)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(path)
    server.listen(8)
    log = open(args.log, "a", buffering=1) if args.log else None

    def stop(*_):
        server.close()
        try:
            os.unlink(path)
        except FileNotFoundError:
            pass
        sys.exit(0)

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    print(f"READY {path}", flush=True)

    served_status = 0
    while True:
        conn, _ = server.accept()
        if args.scenario == "offline" and served_status >= 2:
            # The daemon "dies" after two status answers: the connection is
            # dropped unanswered, so the widget goes offline with a theme it
            # already knows (DESIGN.md section 5).
            conn.close()
            continue
        with conn:
            conn.settimeout(2.0)
            buf = b""
            try:
                while b"\n" not in buf and len(buf) < 65536:
                    chunk = conn.recv(4096)
                    if not chunk:
                        break
                    buf += chunk
            except socket.timeout:
                continue
            line = buf.split(b"\n", 1)[0].strip()
            if not line:
                continue
            try:
                req = json.loads(line)
                resp = daemon.handle(req)
            except (ValueError, KeyError, TypeError) as exc:
                req, resp = {}, daemon.error("invalid_request", f"failed to decode IPC request: {exc}")
            if req.get("request") == "status":
                served_status += 1
            if log:
                log.write(json.dumps({"t": time.time(), "request": req.get("request"), "id": req.get("id"),
                                      "response": resp.get("response"), "code": resp.get("code")}) + "\n")
            try:
                conn.sendall(json.dumps(resp, separators=(",", ":")).encode() + b"\n")
            except (BrokenPipeError, ConnectionResetError):
                pass


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--socket", required=True)
    ap.add_argument("--theme", default="handheld")
    ap.add_argument("--old", action="store_true")
    ap.add_argument("--scenario", default="default",
                    choices=["default", "manual", "paused", "single", "unknown", "noweather",
                             "triple", "longnames", "offline"])
    ap.add_argument("--live", action="store_true")
    ap.add_argument("--theme-warning", default="",
                    help="send this as status.theme_warning (the daemon fell back to another theme)")
    ap.add_argument("--log")
    ap.add_argument("--dump", choices=["status", "day_curve", "themes"],
                    help="print one response body and exit (no socket)")
    args = ap.parse_args()
    if args.dump:
        d = FakeDaemon(args)
        body = {"status": d.status, "day_curve": d.day_curve,
                "themes": lambda: d.handle({"version": 1, "request": "list_themes"})}[args.dump]()
        print(json.dumps(body, indent=1))
        return
    serve(args)


if __name__ == "__main__":
    main()
