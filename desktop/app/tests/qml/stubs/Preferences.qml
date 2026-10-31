// SPDX-License-Identifier: GPL-3.0-or-later
pragma Singleton
import QtQuick

// Stands in for the Rust Preferences in QML tests: light.
QtObject {
    property string colorMode: "light"
    property string commandPaletteHotkey: ""
    property bool streamPcAudioToPhone: false
}
