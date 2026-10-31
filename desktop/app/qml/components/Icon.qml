// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtQuick.Shapes
import app.noddy

// A stroke icon from Icons (an SVG path on a 24×24 grid), or, with `filled`,
// a solid shape such as the Noddy mark.
Item {
    id: icon
    property string path
    property color color: Theme.surfaceContent
    property real stroke: 1.8
    property bool filled: false

    implicitWidth: 20
    implicitHeight: 20

    Shape {
        anchors.centerIn: parent
        width: 24
        height: 24
        scale: Math.min(icon.width, icon.height) / 24
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: icon.color
            // A negative width turns the stroke off for a filled shape.
            strokeWidth: icon.filled ? -1 : icon.stroke
            fillColor: icon.filled ? icon.color : "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: icon.path }
        }
    }
}
