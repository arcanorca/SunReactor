/*
 * The popup's backdrop for synthwave84: a vertical night gradient, from night
 * purple at the top to a deeper indigo at the bottom. Loaded by
 * FullRepresentation (Modules.BACKDROP) and handed to the popup Frame as its
 * fill; the Frame keeps the neon-free outline of the theme.
 */

import QtQuick

import "../../engine"

Gradient {
    id: root

    required property Theme theme

    GradientStop {
        position: 0
        color: root.theme.mix(root.theme.bg, root.theme.borderInactive, 0.14)
    }
    GradientStop {
        position: 1
        color: root.theme.mix(root.theme.bg, Qt.rgba(0, 0, 0, 1), 0.42)
    }
}
