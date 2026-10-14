/*
 * The popup's backdrop for the Wii skin: a clean vertical gradient, from the
 * theme's near-white at the top to a pale grey-blue at the bottom. Loaded by
 * FullRepresentation (Modules.BACKDROP) and handed to the popup Frame as its
 * fill; the channel tiles sit on it.
 */

import QtQuick

import "../../engine"

Gradient {
    id: root

    required property Theme theme

    GradientStop {
        position: 0
        color: root.theme.bg
    }
    GradientStop {
        position: 1
        color: root.theme.mix(root.theme.bg, root.theme.borderInactive, 0.5)
    }
}
