/*
 * A combo box over { value, label } entries that follows a settings value:
 * it shows the entry matching `configValue` and reports the user's choice
 * through `valueSelected`, so the page keeps one cfg_ property as the truth
 * (and the Defaults button moves the box too).
 */

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

QQC2.ComboBox {
    id: root

    /*! The current settings value. */
    property var configValue

    /*! The user picked an entry; `value` is its `value`. */
    signal valueSelected(var value)

    function sync() {
        currentIndex = indexOfValue(configValue);
    }

    Layout.minimumWidth: Kirigami.Units.gridUnit * 12

    textRole: "label"
    valueRole: "value"
    // A saved value this list does not offer (a display that is switched off,
    // a theme that was removed) is shown as it is rather than as another entry.
    displayText: currentIndex >= 0 ? currentText : String(configValue === undefined ? "" : configValue)

    onActivated: root.valueSelected(currentValue)
    onConfigValueChanged: sync()
    onCountChanged: sync()
    // A rebuilt model resets the selection after this signal has fired.
    onModelChanged: Qt.callLater(sync)
    Component.onCompleted: sync()
}
