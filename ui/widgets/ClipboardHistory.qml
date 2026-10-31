import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    property var clipboardItems: []

    width: 300
    height: parent.height * 0.6
    radius: 8
    color: "#dd222222"
    border.color: "#444444"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        Label {
            text: "Clipboard History"
            color: "#ffffff"
            font.pixelSize: 16
            font.bold: true
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 4

            model: root.clipboardItems

            delegate: Rectangle {
                width: ListView.view.width
                height: 48
                radius: 4
                color: mouseArea.containsMouse ? "#44ffffff" : "#33ffffff"

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 8

                    Image {
                        source: modelData.type === "text" ? "qrc:/icons/text.svg" : "qrc:/icons/file.svg"
                        width: 24
                        height: 24
                    }

                    Label {
                        text: modelData.preview || "No preview"
                        color: "#ffffff"
                        font.pixelSize: 12
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Label {
                        text: modelData.timestamp || ""
                        color: "#888888"
                        font.pixelSize: 10
                    }
                }

                MouseArea {
                    id: mouseArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        daemonClient.copyToClipboard(modelData.data)
                        root.visible = false
                    }
                }
            }
        }
    }
}
