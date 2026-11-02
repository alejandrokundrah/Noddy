// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import app.noddy

// The folders another paired computer shares: look through them and download
// files. What it shares is its owner's decision, so a computer that hasn't
// allowed this one says how to.
Sheet {
    id: sheet
    cardWidth: 640
    onDismissed: RemoteFiles.close()

    function openFor(device) {
        RemoteFiles.open(device)
        open()
    }
    function closeBrowser() {
        RemoteFiles.close()
        close()
    }

    readonly property var items: {
        try { return JSON.parse(RemoteFiles.entries) } catch (e) { return [] }
    }
    readonly property var steps: {
        try { return JSON.parse(RemoteFiles.crumbs) } catch (e) { return [] }
    }
    readonly property bool busy: RemoteFiles.phase === "loading"

    function sizeText(bytes) {
        if (bytes < 1024) return qsTr("%1 B").arg(bytes)
        if (bytes < 1024 * 1024) return qsTr("%1 KB").arg(Math.round(bytes / 1024))
        if (bytes < 1024 * 1024 * 1024) return qsTr("%1 MB").arg((bytes / (1024 * 1024)).toFixed(1))
        return qsTr("%1 GB").arg((bytes / (1024 * 1024 * 1024)).toFixed(1))
    }

    Column {
        width: parent.width
        spacing: 12

        Item {
            width: parent.width
            height: Math.max(title.height, headerButtons.height)
            Txt {
                id: title
                anchors.left: parent.left
                anchors.right: headerButtons.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                text: qsTr("Files on %1").arg(RemoteFiles.deviceName)
                role: "headline"
                elide: Text.ElideRight
            }
            Row {
                id: headerButtons
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4
                IconButton {
                    iconPath: Icons.back
                    label: qsTr("Up one folder")
                    enabled: RemoteFiles.canGoUp && !sheet.busy
                    onClicked: RemoteFiles.up()
                }
                IconButton {
                    iconPath: Icons.refresh
                    label: qsTr("Refresh")
                    enabled: !sheet.busy
                    onClicked: RemoteFiles.refresh()
                }
                IconButton {
                    iconPath: Icons.close
                    label: qsTr("Close")
                    onClicked: sheet.closeBrowser()
                }
            }
        }

        // Where you are: each step can be clicked to go back to it.
        Flow {
            width: parent.width
            spacing: 4
            Repeater {
                model: sheet.steps
                delegate: Row {
                    id: step
                    required property var modelData
                    required property int index
                    spacing: 4
                    Icon {
                        visible: step.index > 0
                        anchors.verticalCenter: parent.verticalCenter
                        width: 14; height: 14
                        path: Icons.chevronRight
                        color: Theme.surfaceContentVariant
                    }
                    Button {
                        variant: step.index === sheet.steps.length - 1 ? "tonal" : "outline"
                        size: "sm"
                        text: step.modelData.name
                        enabled: step.index < sheet.steps.length - 1 && !sheet.busy
                        onClicked: RemoteFiles.list(step.modelData.path)
                    }
                }
            }
        }

        // Loading, nothing there, or not allowed / failed.
        Column {
            width: parent.width
            visible: RemoteFiles.phase !== "ready" || sheet.items.length === 0
            spacing: 10
            topPadding: 24
            bottomPadding: 24
            Spinner {
                anchors.horizontalCenter: parent.horizontalCenter
                visible: sheet.busy
                width: 24; height: 24
            }
            Txt {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                role: "body"
                muted: true
                text: sheet.busy ? qsTr("Opening the folder…")
                    : (RemoteFiles.phase === "ready" ? qsTr("This folder is empty.") : RemoteFiles.message)
            }
        }

        Flickable {
            width: parent.width
            height: Math.min(380, listColumn.height)
            visible: RemoteFiles.phase === "ready" && sheet.items.length > 0
            contentHeight: listColumn.height
            clip: true
            boundsBehavior: Flickable.StopAtBounds

            Column {
                id: listColumn
                width: parent.width
                Repeater {
                    model: sheet.items
                    delegate: Item {
                        id: entry
                        required property var modelData
                        required property int index
                        width: listColumn.width
                        height: 52
                        readonly property bool downloadingThis: RemoteFiles.downloading === modelData.path

                        Rectangle {
                            anchors.fill: parent
                            radius: Theme.radiusMd
                            color: Theme.surfaceContent
                            opacity: tap.pressed ? 0.12 : (hover.hovered ? 0.06 : 0)
                        }
                        Rectangle {
                            anchors.fill: parent
                            radius: Theme.radiusMd
                            color: "transparent"
                            border.width: Theme.focusVisible(entry) ? 2 : 0
                            border.color: Theme.primary
                        }
                        activeFocusOnTab: true
                        Accessible.role: Accessible.Button
                        Accessible.name: modelData.isDir ? qsTr("Open folder %1").arg(modelData.name)
                                                          : qsTr("Download %1").arg(modelData.name)
                        function act() {
                            if (modelData.isDir) RemoteFiles.list(modelData.path)
                            else if (!entry.downloadingThis) RemoteFiles.download(modelData.path)
                        }
                        Accessible.onPressAction: entry.act()
                        Keys.onReturnPressed: entry.act()
                        Keys.onEnterPressed: entry.act()
                        Keys.onSpacePressed: entry.act()
                        HoverHandler { id: hover }
                        TapHandler { id: tap; onTapped: entry.act() }

                        // How much of the file has arrived.
                        Rectangle {
                            visible: entry.downloadingThis
                            anchors.left: parent.left
                            anchors.bottom: parent.bottom
                            anchors.leftMargin: 10
                            height: 3
                            radius: 1.5
                            width: Math.max(0, (parent.width - 20) * RemoteFiles.progress)
                            color: Theme.primary
                        }
                        Icon {
                            id: kindIcon
                            anchors.left: parent.left
                            anchors.leftMargin: 10
                            anchors.verticalCenter: parent.verticalCenter
                            width: 20; height: 20
                            path: entry.modelData.isDir ? Icons.folder : Icons.file
                            color: entry.modelData.isDir ? Theme.primary : Theme.surfaceContentVariant
                        }
                        Column {
                            anchors.left: kindIcon.right
                            anchors.leftMargin: 12
                            anchors.right: trailing.left
                            anchors.rightMargin: 10
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 1
                            Txt {
                                width: parent.width
                                text: entry.modelData.name
                                role: "title"
                                size: 14
                                elide: Text.ElideMiddle
                            }
                            Txt {
                                visible: !entry.modelData.isDir
                                width: parent.width
                                text: entry.downloadingThis
                                      ? (RemoteFiles.progress > 0
                                         ? qsTr("Downloading… %1%").arg(Math.round(RemoteFiles.progress * 100))
                                         : qsTr("Downloading…"))
                                      : sheet.sizeText(entry.modelData.size)
                                role: "bodySmall"
                                muted: true
                                elide: Text.ElideRight
                            }
                        }
                        Item {
                            id: trailing
                            anchors.right: parent.right
                            anchors.rightMargin: 12
                            anchors.verticalCenter: parent.verticalCenter
                            width: 22; height: 22
                            Spinner {
                                anchors.fill: parent
                                visible: entry.downloadingThis
                            }
                            Icon {
                                anchors.fill: parent
                                visible: !entry.downloadingThis
                                path: entry.modelData.isDir ? Icons.chevronRight : Icons.download
                                color: Theme.surfaceContentVariant
                            }
                        }
                    }
                }
            }
        }

        Txt {
            width: parent.width
            wrapMode: Text.WordWrap
            role: "bodySmall"
            muted: true
            text: qsTr("You can look through these folders and download files. Downloads are saved in Downloads/Noddy. Nothing here can be changed from this computer.")
        }
    }
}
