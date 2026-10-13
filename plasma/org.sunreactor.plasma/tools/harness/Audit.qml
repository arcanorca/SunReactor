/*
 * Layout audit of the popup, run by tools/overflow_audit.sh (harness only).
 *
 * tools/shot.sh --audit makes the harness containment load this file (it copies
 * it next to the containment's main.qml). Once the daemon has data and the
 * theme is in, and 800 ms have passed, it walks the popup's content tree and
 * prints one marker line per finding, then SUNREACTOR_AUDIT_END. The shell
 * parses those lines from the viewer's log; nothing is written by QML itself.
 *
 *   SUNREACTOR_AUDIT_META     {theme, page, popup size, scroll pages, ...}
 *   SUNREACTOR_AUDIT_FINDING  {kind, against, path, file, box, detail}
 *   SUNREACTOR_AUDIT_END      {findings, attempts}
 *
 * Kinds:
 *   OVERFLOW   a visible item's box leaves its nearest container: a skin frame
 *              (class name ending in Frame, Panel, Card, Window, Section, Bezel,
 *              Channel or Strip), a clipping item (clip: true), or the scroll
 *              viewport of a Flickable (horizontal edges only: vertical
 *              overflow there is scrolling). Otherwise it leaves the popup's
 *              content viewport horizontally. Containers are checked innermost
 *              first; one finding per item.
 *   ELIDED     text cut with an ellipsis: QtQuick Text with truncated, a
 *              PixelLabel/PixelGlyphs with elided, or a PixelText whose natural
 *              width (measured by a probe with the same font and no limit)
 *              exceeds maximumArtWidth.
 *   CLIPPED    QtQuick Text with elide: none whose contentWidth exceeds width
 *              by more than 1 px.
 *   COLLISION  two text items in the same row or layout (Row, Grid, Column,
 *              Flow, RowLayout, ...) whose boxes overlap by more than 2 px in
 *              both directions. Only the outermost text of each subtree counts.
 *              Not counted: the same text a pixel or two away (shadow, glow)
 *              and rules of box-drawing or dash characters (a label on a rule).
 *
 * Skipped: items with `auditIgnore: true` (with their subtree), items not
 * visible or with zero opacity (with their subtree). Decorative art that
 * deliberately paints outside its card (glows, bezels) should carry
 * `property bool auditIgnore: true`; skins are not edited for it here.
 */

import QtQuick
import org.sunreactor.plasma

Item {
    id: audit

    visible: false

    /*! The popup's FullRepresentation: the audited viewport. Set by the containment. */
    property var representation: null
    property bool finished: false
    property int attempts: 0
    readonly property int maxAttempts: 40

    readonly property var client: representation ? representation.client : null
    readonly property bool ready: representation !== null && client !== null
        && client.isConnected && client.monitors.length > 0
        && representation.theme.available === true

    /*! Debugging aid: every visited item as SUNREACTOR_AUDIT_NODE (shot.sh --config "auditDump=1"). */
    readonly property bool dump: representation !== null && representation.harness
        ? representation.harness.auditDump === 1 : false

    readonly property var frameRe: /(Frame|Panel|Card|Window|Section|Bezel|Channel|Strip)$/
    readonly property var rowRe: /(Row|Grid|Column|Flow)(Layout)?$/
    readonly property var ruleRe: /^[\s\u2500-\u259F·.\-_=~•]+$/

    onReadyChanged: {
        if (audit.ready && !audit.finished && audit.attempts === 0) {
            settle.restart();
        }
    }

    Timer {
        id: settle

        interval: 800
        onTriggered: audit.attempt()
    }

    Timer {
        id: retry

        interval: 150
        onTriggered: audit.attempt()
    }

    function attempt(): void {
        if (audit.finished || !audit.ready) {
            return;
        }
        audit.attempts += 1;
        let state;
        try {
            state = audit.measure();
        } catch (error) {
            console.info("SUNREACTOR_AUDIT_ERROR " + JSON.stringify(String(error)));
            audit.finished = true;
            return;
        }
        // Items still loading (Loader, Image) would move: measure again later.
        if (state.pending > 0 && audit.attempts < audit.maxAttempts) {
            retry.restart();
            return;
        }
        audit.emitState(state);
        audit.finished = true;
    }

    function emitState(state: var): void {
        console.info("SUNREACTOR_AUDIT_META " + JSON.stringify(state.meta));
        for (let i = 0; i < state.findings.length; ++i) {
            console.info("SUNREACTOR_AUDIT_FINDING " + JSON.stringify(state.findings[i]));
        }
        console.info("SUNREACTOR_AUDIT_END " + JSON.stringify({
            findings: state.findings.length,
            attempts: audit.attempts,
            pending: state.pending,
        }));
    }

    function measure(): var {
        const vp = audit.representation;
        const vpBox = { l: 0, t: 0, r: vp.width, b: vp.height };
        const st = { findings: [], candidates: [], scroll: [], pending: 0, nodes: 0, vp: vp };
        const top = { opacity: 1, path: [], file: "", containers: [], row: null, inText: false, reportedText: false };
        audit.walk([vp.contentItem, vp.footer], top, st, vpBox);
        audit.collide(st);
        const rep = vp;
        const meta = {
            theme: rep.theme.themeId,
            page: rep.shownPage,
            tabs: rep.tabsShown,
            popup: { w: rep.width, h: rep.height },
            dpr: Window.window ? Window.window.devicePixelRatio : 1,
            popupHeight: rep.popupHeight,
            popupMaxHeight: rep.popupMaxHeight,
            nodes: st.nodes,
            scroll: st.scroll,
        };
        return { meta: meta, findings: st.findings, pending: st.pending };
    }

    /*! The class name without the QML suffix ("PixelPanel_QML_3" -> "PixelPanel"). */
    function classOf(item): string {
        // QObject's string form is "ClassName(0x...)"; QML types are "Name_QML_n" or "Name_QMLTYPE_n".
        return String(item).replace(/\(0x[0-9a-fA-F]+\)$/, "").replace(/_QML(TYPE)?_\d+$/, "").replace(/^QQuick/, "");
    }

    /*! The item's box in viewport coordinates, or null when it has no size. */
    function boxOf(item, vp): var {
        if (!(item.width > 0 && item.height > 0)) {
            return null;
        }
        const a = item.mapToItem(vp, 0, 0);
        const b = item.mapToItem(vp, item.width, item.height);
        return {
            l: Math.min(a.x, b.x), t: Math.min(a.y, b.y),
            r: Math.max(a.x, b.x), b: Math.max(a.y, b.y),
        };
    }

    function round2(value: real): real {
        return Math.round(value * 100) / 100;
    }

    function plainBox(box: var): var {
        return { l: round2(box.l), t: round2(box.t), r: round2(box.r), b: round2(box.b),
                 w: round2(box.r - box.l), h: round2(box.b - box.t) };
    }

    /*! The worst edge of `box` beyond `outer` (more than half a pixel), or null. */
    function edgeOver(box: var, outer: var, horizontalOnly: bool): var {
        const tolerance = 0.5;
        const edges = [
            { edge: "left", px: outer.l - box.l },
            { edge: "right", px: box.r - outer.r },
        ];
        if (!horizontalOnly) {
            edges.push({ edge: "top", px: outer.t - box.t });
            edges.push({ edge: "bottom", px: box.b - outer.b });
        }
        let worst = null;
        for (const e of edges) {
            if (e.px > tolerance && (worst === null || e.px > worst.px)) {
                worst = e;
            }
        }
        return worst;
    }

    /*! Where an item leaves its containers, innermost first, else the viewport. */
    function overflowOf(box: var, containers: var, vpBox: var): var {
        for (let k = containers.length - 1; k >= 0; --k) {
            const outer = containers[k];
            const hit = audit.edgeOver(box, outer.box, outer.kind === "scroll");
            if (hit) {
                return { against: outer.kind, label: outer.label, bound: audit.plainBox(outer.box), edge: hit.edge, px: round2(hit.px) };
            }
        }
        const hit = audit.edgeOver(box, vpBox, true);
        return hit ? { against: "viewport", label: "popup", bound: audit.plainBox(vpBox), edge: hit.edge, px: round2(hit.px) } : null;
    }

    /*! Natural width (art px) of a PixelText's text, measured by a probe with no limit. */
    function naturalArtWidth(item): int {
        const probe = Qt.createQmlObject(
            'import org.sunreactor.plasma; PixelText { visible: false; maximumArtWidth: 0 }', item);
        probe.text = item.text;
        probe.fontFamily = item.fontFamily;
        probe.pixelSize = item.pixelSize;
        probe.artScale = item.artScale;
        probe.letterSpacing = item.letterSpacing;
        const width = probe.artWidth;
        probe.destroy();
        return width;
    }

    /*! The text problem of one text item (ELIDED or CLIPPED), or null. */
    function textProblem(item): var {
        if (typeof item.truncated === "boolean" && typeof item.contentWidth === "number") {
            const detail = {
                elide: item.elide, width: round2(item.width), contentWidth: round2(item.contentWidth),
                pixelSize: item.font.pixelSize,
            };
            if (item.truncated) {
                return { kind: "ELIDED", detail: detail };
            }
            if (item.elide === Text.ElideNone && item.contentWidth > item.width + 1) {
                return { kind: "CLIPPED", detail: detail };
            }
            return null;
        }
        if (typeof item.elided === "boolean") {
            return item.elided ? { kind: "ELIDED", detail: { artWidth: item.artWidth, maximumArtWidth: item.maximumArtWidth } } : null;
        }
        if (typeof item.fontFamily === "string" && typeof item.pixelSize === "number" && item.maximumArtWidth > 0) {
            const natural = audit.naturalArtWidth(item);
            if (natural > item.maximumArtWidth) {
                return { kind: "ELIDED", detail: { artWidth: item.artWidth, natural: natural, maximumArtWidth: item.maximumArtWidth } };
            }
        }
        return null;
    }

    function walk(items: var, ctx: var, st: var, vpBox: var): void {
        for (let i = 0; i < items.length; ++i) {
            const c = items[i];
            if (!c || c.visible !== true || c.auditIgnore === true) {
                continue;
            }
            const opacity = ctx.opacity * c.opacity;
            if (!(opacity > 0)) {
                continue;
            }
            st.nodes += 1;
            const base = audit.classOf(c);
            const label = c.objectName ? base + "#" + c.objectName : base;
            const next = {
                opacity: opacity,
                path: ctx.path.concat([label]),
                file: ctx.file,
                containers: ctx.containers,
                row: ctx.row,
                inText: ctx.inText,
                reportedText: ctx.reportedText,
            };
            if (c.progress !== undefined && c.status === 2) {
                st.pending += 1; // Loader or Image still loading
            }
            if (c.item !== undefined && c.source !== undefined && String(c.source).length > 0) {
                next.file = String(c.source);
            }

            const box = audit.boxOf(c, st.vp);
            if (audit.dump) {
                console.info("SUNREACTOR_AUDIT_NODE " + JSON.stringify({ path: next.path.join("/"),
                    box: box ? audit.plainBox(box) : null, opacity: opacity,
                    text: typeof c.text === "string" ? c.text.slice(0, 40) : undefined }));
            }
            if (box) {
                const hit = audit.overflowOf(box, ctx.containers, vpBox);
                if (hit) {
                    st.findings.push({ kind: "OVERFLOW", against: hit.against, label: hit.label, bound: hit.bound,
                        edge: hit.edge, px: hit.px, path: next.path, file: next.file, box: audit.plainBox(box) });
                }
                // A ScrollView (its contentItem is a Flickable) is not a frame: its Flickable is the container.
                const scrollView = c.contentItem !== undefined && c.contentItem !== null
                    && c.contentItem.flickableDirection !== undefined;
                if (audit.frameRe.test(base) && !scrollView) {
                    next.containers = ctx.containers.concat([{ kind: "frame", box: box, label: label }]);
                } else if (c.flickableDirection !== undefined && c.contentHeight !== undefined) {
                    next.containers = ctx.containers.concat([{ kind: "scroll", box: box, label: label }]);
                    st.scroll.push({ page: c.parent && c.parent.modelData !== undefined ? c.parent.modelData : "",
                        height: round2(c.height), contentHeight: round2(c.contentHeight),
                        needsVertical: c.contentHeight > c.height + 0.5 });
                } else if (c.clip === true && !scrollView) {
                    next.containers = ctx.containers.concat([{ kind: "clip", box: box, label: label }]);
                }
                if (audit.rowRe.test(base)) {
                    next.row = c;
                }
            }

            if (typeof c.text === "string" && c.text.length > 0) {
                if (!ctx.inText && box && ctx.row !== null) {
                    st.candidates.push({ row: ctx.row, box: box, label: label, path: next.path, file: next.file,
                        text: c.text.slice(0, 80) });
                }
                next.inText = true;
                if (!ctx.reportedText) {
                    const problem = audit.textProblem(c);
                    if (problem) {
                        st.findings.push({ kind: problem.kind, path: next.path, file: next.file,
                            text: c.text.slice(0, 80), box: box ? audit.plainBox(box) : null, detail: problem.detail });
                        next.reportedText = true;
                    }
                }
            }

            audit.walk(c.children, next, st, vpBox);
        }
    }

    /*! COLLISION: text candidates in the same row whose boxes overlap by more than 2 px both ways. */
    function collide(st: var): void {
        const cands = st.candidates;
        for (let i = 0; i < cands.length; ++i) {
            for (let j = i + 1; j < cands.length; ++j) {
                const a = cands[i];
                const b = cands[j];
                if (a.row !== b.row) {
                    continue;
                }
                // The same text a pixel or two away is a shadow or glow, not a collision.
                const shadow = a.text === b.text && Math.abs(a.box.l - b.box.l) < 3 && Math.abs(a.box.t - b.box.t) < 3;
                // A rule of box-drawing or dash characters is meant to run under a label.
                const rule = audit.ruleRe.test(a.text) || audit.ruleRe.test(b.text);
                const ox = Math.min(a.box.r, b.box.r) - Math.max(a.box.l, b.box.l);
                const oy = Math.min(a.box.b, b.box.b) - Math.max(a.box.t, b.box.t);
                if (ox > 2 && oy > 2 && !shadow && !rule) {
                    st.findings.push({ kind: "COLLISION", path: a.path, file: a.file, text: a.text,
                        other: { path: b.path, file: b.file, text: b.text, box: audit.plainBox(b.box) },
                        box: audit.plainBox(a.box), detail: { overlapX: round2(ox), overlapY: round2(oy) } });
                }
            }
        }
    }
}
