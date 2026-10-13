/*
 * Automation mode, shared by every skin's Automation module: which of three
 * choices the daemon is in, what that means in words, and what each choice does
 * when it is picked. Skins only draw it; no skin decides a state.
 *
 *   - The choice is what the daemon does. Paused wins over set by hand, and set
 *     by hand wins over automatic. "Dimmed while idle" is automatic with a note.
 *   - Automatic is the way back in one step: it resumes a pause and drops every
 *     override. resume() drops overrides too; clearAllOverrides() is sent as
 *     well so the rule holds if that ever changes.
 *   - Manual never writes a brightness. It focuses the first display slider and
 *     says "Drag a slider to take over". A paused daemon ignores overrides, so
 *     it is resumed first.
 *   - Paused suspends for the configured default. pause(minutes) takes any
 *     duration; 0 means until resumed.
 *
 * Strings live here, in QML: JavaScript resources have no i18n.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../Format.js" as Format

QtObject {
    id: root

    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain

    /*! "automatic", "manual", "paused", or "" while the daemon does not answer. */
    readonly property string choice: {
        if (!client.isConnected) {
            return "";
        }
        switch (client.mode) {
        case SunReactorClient.Paused:
            return "paused";
        case SunReactorClient.Manual:
            return "manual";
        default:
            return "automatic";
        }
    }

    /*! Left to right: the order of the choices and of keyboard focus. */
    readonly property var order: ["automatic", "manual", "paused"]
    readonly property var choices: [
        {
            id: "automatic",
            label: i18ndc(domain, "@action:button Mode: follow the sun again", "Automatic"),
            tip: i18nd(domain, "Back to automatic brightness: resumes and drops every setting made by hand")
        },
        {
            id: "manual",
            label: i18ndc(domain, "@action:button Mode: set brightness by hand", "Manual"),
            tip: i18nd(domain, "Set brightness by hand: drag a display's slider")
        },
        {
            id: "paused",
            label: i18ndc(domain, "@action:button Mode: stop adjusting for a while", "Paused"),
            tip: i18nd(domain, "Stop adjusting brightness for a while. The arrow offers other durations")
        }
    ]

    readonly property bool idleDimmed: choice !== "" && client.mode === SunReactorClient.IdleDimmed
    readonly property string note: idleDimmed ? i18nd(domain, "Dimmed while idle") : ""

    // --- the day curve of the first display ----------------------------------

    readonly property var curveMonitor: client.dayCurve && client.dayCurve.monitors
        && client.dayCurve.monitors.length > 0 ? client.dayCurve.monitors[0] : null
    readonly property var milestones: curveMonitor && curveMonitor.milestones ? curveMonitor.milestones : []
    readonly property var nextMilestone: {
        for (const milestone of milestones) {
            if (milestone.t > client.nowEpochS) {
                return milestone;
            }
        }
        return null;
    }

    /*! "Next 4:20 PM · Set 50% → 23%": the daemon's milestone name, then its target. */
    readonly property string nextText: {
        const m = nextMilestone;
        if (!m) {
            return "";
        }
        let step = m.label;
        if (m.pct >= 0) {
            step = i18ndc(domain, "Milestone name, then the brightness it targets", "%1 → %2%", m.label, m.pct);
        }
        return i18ndc(domain, "Time of the next step, then what happens then", "Next %1 · %2",
                      Format.timeText(m.t), step);
    }

    // --- the sentence under the choices -------------------------------------

    function joinParts(parts) {
        return parts.filter(part => part.length > 0).join(" · ");
    }

    /*! "1 h 12 min left", counting down with the daemon's clock; "" when past or unknown. */
    function remaining(untilEpochS) {
        if (!(client.nowEpochS > 0) || !(untilEpochS > client.nowEpochS)) {
            return "";
        }
        const minutes = Math.max(1, Math.round((untilEpochS - client.nowEpochS) / 60));
        const hours = Math.floor(minutes / 60);
        if (hours > 0) {
            return minutes % 60 === 0
                ? i18ndc(domain, "Time left, whole hours", "%1 h left", hours)
                : i18ndc(domain, "Time left, hours and minutes", "%1 h %2 min left", hours, minutes % 60);
        }
        return i18ndc(domain, "Time left, minutes", "%1 min left", minutes);
    }

    readonly property string statusText: {
        switch (choice) {
        case "paused":
            return client.suspendUntilEpochS > client.nowEpochS
                ? i18ndc(domain, "Placeholder is a time of day", "Paused · resumes %1",
                         Format.timeText(client.suspendUntilEpochS))
                : i18nd(domain, "Paused · until you resume it");
        case "manual":
            return joinParts([
                client.overrideUntilEpochS > client.nowEpochS
                    ? i18ndc(domain, "Placeholder is a time of day", "Set by hand · until %1",
                             Format.timeText(client.overrideUntilEpochS))
                    : i18nd(domain, "Set by hand · until you switch back"),
                remaining(client.overrideUntilEpochS)
            ]);
        case "automatic": {
            const title = client.hasWeatherReading
                ? i18nd(domain, "Following the sun and the weather")
                : i18nd(domain, "Following the sun");
            let next = nextText;
            if (next.length === 0 && milestones.length > 0) {
                next = i18nd(domain, "Nothing more today");
            }
            return joinParts([title, next]);
        }
        default:
            return "";
        }
    }

    /*! Shown in place of the sentence for a few seconds after Manual moved focus. */
    property bool hinting: false
    /*! Whether the last Manual found a slider to focus (the terminal skin has none). */
    property bool sliderFocused: false
    readonly property string hintText: sliderFocused
        ? i18nd(domain, "Drag a slider to take over")
        : i18nd(domain, "Change a display to take over")
    readonly property string displayText: hinting ? hintText : statusText

    // The hint goes once the user has taken over. A resume out of a pause changes
    // the choice too, and must not clear the hint it was asked for.
    onChoiceChanged: {
        if (choice === "manual") {
            hinting = false;
            hintTimer.stop();
        }
    }

    readonly property Timer hintTimer: Timer {
        interval: 6000
        onTriggered: root.hinting = false
    }

    // --- actions -------------------------------------------------------------

    /*! Pauses automation for `minutes` (0 = until resumed). Works from any choice,
        so the arrow can also change the length of a pause already running. */
    function pause(minutes) {
        if (client.isConnected) {
            client.suspend(minutes);
        }
    }

    /*! The user picked a choice (click, Space or Enter). `anchor` is any item of
        the popup; Manual uses its window to find the first display slider. */
    function pick(id, anchor) {
        if (!client.isConnected) {
            return;
        }
        switch (id) {
        case "automatic":
            if (choice !== "automatic") {
                if (client.isSuspended) {
                    client.resume();
                }
                client.clearAllOverrides();
            }
            break;
        case "manual":
            if (client.isSuspended) {
                client.resume();
            }
            sliderFocused = focusDisplay(anchor);
            if (choice !== "manual") {
                hinting = true;
                hintTimer.restart();
            }
            break;
        case "paused":
            if (choice !== "paused") {
                pause(settings.defaultSuspendMinutes);
            }
            break;
        }
    }

    /*! Moves keyboard focus to the first display slider of the popup. Sliders are
        found by their Accessible role, so this follows whichever skin draws them.
        Returns false when there is none (for example a keyboard-driven table). */
    function focusDisplay(anchor) {
        const win = anchor && anchor.Window.window ? anchor.Window.window : null;
        const slider = win ? firstSlider(win.contentItem) : null;
        if (slider) {
            slider.forceActiveFocus(Qt.TabFocusReason);
        }
        return slider !== null;
    }

    /*! A slider: an explicit Slider role (pixel cells), or a Quick Controls slider,
        which has the range properties and no role of its own. */
    function isSlider(item) {
        if (item.Accessible.role === Accessible.Slider) {
            return true;
        }
        return item.stepSize !== undefined && item.from !== undefined && item.to !== undefined
            && item.value !== undefined;
    }

    /*! Breadth-first, so the shallowest slider wins; hidden subtrees are skipped. */
    function firstSlider(scene) {
        const queue = [scene];
        for (let i = 0; i < queue.length; ++i) {
            for (const child of queue[i].children) {
                if (!child.visible) {
                    continue;
                }
                if (child.enabled && isSlider(child)) {
                    return child;
                }
                queue.push(child);
            }
        }
        return null;
    }
}
