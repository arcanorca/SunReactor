/*
 * The copy-command field in the not-connected notice (qmltestrunner, offscreen):
 * the field is read-only, a click on Copy puts the command on the clipboard and
 * the button shows the checkmark until the feedback timer ends.
 */

import QtQuick
import QtTest

import "../package/contents/ui/components"

Item {
    id: stage

    width: 240
    height: 80

    CopyCommand {
        id: cmd

        anchors.centerIn: parent
        width: 200
        command: "systemctl --user start sunreactord"
    }

    TextEdit {
        id: sink
    }

    TestCase {
        name: "CopyCommand"
        when: windowShown

        function test_copy() {
            verify(cmd.field.readOnly);
            compare(cmd.field.text, "systemctl --user start sunreactord");
            verify(cmd.width <= stage.width);
            verify(!cmd.copied);
            cmd.copy();
            verify(cmd.copied);
            compare(cmd.field.selectedText, "");
            sink.paste();
            compare(sink.text, "systemctl --user start sunreactord");
            tryVerify(function () { return !cmd.copied; }, 3000);
        }
    }
}
