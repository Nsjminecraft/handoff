import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    property var devices: []

    color: "#ee111111"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 16

        Label {
            text: "Paired Devices"
            color: "#ffffff"
            font.pixelSize: 24
            font.bold: true
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 8

            model: root.devices

            delegate: Rectangle {
                width: ListView.view.width
                height: 80
                radius: 8
                color: "#dd333333"

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 12
                    spacing: 12

                    Rectangle {
                        width: 48
                        height: 48
                        radius: 24
                        color: modelData.isOnline ? "#4caf50" : "#666666"

                        Label {
                            anchors.centerIn: parent
                            text: modelData.name.charAt(0).toUpperCase()
                            color: "#ffffff"
                            font.pixelSize: 20
                            font.bold: true
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Label {
                            text: modelData.name
                            color: "#ffffff"
                            font.pixelSize: 16
                            font.bold: true
                        }

                        Label {
                            text: modelData.isOnline ? "Online" : "Offline"
                            color: modelData.isOnline ? "#4caf50" : "#888888"
                            font.pixelSize: 12
                        }

                        Label {
                            text: modelData.addr + ":" + modelData.port
                            color: "#666666"
                            font.pixelSize: 10
                        }
                    }

                    Button {
                        text: "Clipboard"
                        onClicked: daemonClient.sendClipboard(modelData.id)
                    }

                    Button {
                        text: "Unlock"
                        onClicked: daemonClient.requestUnlock(modelData.id)
                    }
                }
            }
        }

        Button {
            text: "Scan for Devices"
            Layout.alignment: Qt.AlignHCenter
            onClicked: daemonClient.startDiscovery()
        }
    }
}
