import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root
    property var connectedDevices: []

    width: statusRow.width + 16
    height: 32

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: "#cc222222"
        border.color: "#444444"
        border.width: 1
    }

    RowLayout {
        id: statusRow
        anchors.centerIn: parent
        spacing: 6

        Rectangle {
            width: 8
            height: 8
            radius: 4
            color: root.connectedDevices.length > 0 ? "#4caf50" : "#666666"
        }

        Label {
            text: root.connectedDevices.length > 0
                  ? root.connectedDevices.length + " connected"
                  : "No peers"
            color: "#ffffff"
            font.pixelSize: 12
        }
    }

    MouseArea {
        anchors.fill: parent
        onClicked: devicePanel.visible = !devicePanel.visible
    }
}
