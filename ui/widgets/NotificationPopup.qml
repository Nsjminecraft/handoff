import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    property string title: ""
    property string body: ""
    property var actions: []
    property bool autoDismiss: true

    width: parent.width
    height: notifLayout.height + 24
    radius: 8
    color: "#dd333333"
    border.color: "#555555"

    function show(t, b, a) {
        title = t
        body = b
        actions = a || []
        visible = true
        if (autoDismiss) dismissTimer.restart()
    }

    function dismiss() {
        visible = false
        dismissTimer.stop()
    }

    Timer {
        id: dismissTimer
        interval: 5000
        onTriggered: root.dismiss()
    }

    ColumnLayout {
        id: notifLayout
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 12
        spacing: 6

        Label {
            text: root.title
            color: "#ffffff"
            font.pixelSize: 14
            font.bold: true
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        Label {
            text: root.body
            color: "#cccccc"
            font.pixelSize: 12
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        RowLayout {
            visible: root.actions.length > 0
            spacing: 8

            Repeater {
                model: root.actions
                delegate: Button {
                    text: modelData.label
                    onClicked: {
                        daemonClient.sendAction(root.title, modelData.id)
                        root.dismiss()
                    }
                }
            }
        }
    }
}
