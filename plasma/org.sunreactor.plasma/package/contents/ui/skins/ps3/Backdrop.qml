/*
 * The popup's fill for the PS3 skin: the XMB blue, a deep navy at the top and
 * the bottom, lifted into a brighter band through the middle where the
 * category row and the selected item sit. Loaded by FullRepresentation
 * (Modules.BACKDROP) and handed to the popup Frame as its gradient.
 */

import QtQuick

import "../../engine"

Gradient {
    id: root

    required property Theme theme

    GradientStop {
        position: 0
        color: root.theme.mix(root.theme.bg, root.theme.borderInactive, 0.55)
    }
    GradientStop {
        position: 0.42
        color: root.theme.mix(root.theme.bg, root.theme.accent, 0.30)
    }
    GradientStop {
        position: 0.66
        color: root.theme.mix(root.theme.bg, root.theme.borderInactive, 0.45)
    }
    GradientStop {
        position: 1
        color: root.theme.bg
    }
}
