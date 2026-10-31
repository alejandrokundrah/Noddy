// SPDX-License-Identifier: GPL-3.0-or-later
pragma Singleton
import QtQuick
import app.noddy

// The theme: Glass, the app's only look, in light or dark (resolved from
// Preferences and the system). Colors come from Tokens (generated from
// docs/design/tokens.json); Material's "onX" roles are named "xContent"
// because QML reserves the "on" prefix.
QtObject {
    id: theme

    readonly property bool glass: true
    readonly property bool dark: Preferences.colorMode === "dark"
                                 || (Preferences.colorMode === "system" && AppController.systemDark)
    readonly property bool reduceMotion: AppController.reduceMotion

    // Left over from the themes Glass replaced; consumers still read them,
    // and they never change. Fold their branches away as each file is touched.
    readonly property bool hive: false
    readonly property bool graphite: false
    readonly property bool amoled: false
    readonly property bool isPhoneSeed: false
    readonly property bool isArtworkSeed: false
    readonly property bool artworkAmbientActive: false
    readonly property bool ambientBlurActive: true
    function phonePalette(deviceId) { return null }

    // Glass's fixed colors: the default (Honey) preset.
    readonly property var palette: Tokens.data.themes.bloom.seeds[Tokens.data.themes.bloom.defaultSeed][dark ? "dark" : "light"]
    readonly property var status: Tokens.data.themes.bloom.status[dark ? "dark" : "light"]

    // ---- Color roles ----
    readonly property color primary: palette.primary
    readonly property color primaryContent: palette.onPrimary
    readonly property color primaryContainer: palette.primaryContainer
    readonly property color primaryContainerContent: palette.onPrimaryContainer
    readonly property color secondaryContainer: palette.secondaryContainer
    readonly property color secondaryContainerContent: palette.onSecondaryContainer
    readonly property color surface: palette.surface
    readonly property color surfaceLow: palette.surfaceContainerLow
    readonly property color surfaceContainerLow: palette.surfaceContainerLow
    readonly property color surfaceContainer: palette.surfaceContainer
    readonly property color surfaceContainerHigh: palette.surfaceContainerHigh
    readonly property color surfaceContainerHighest: palette.surfaceContainerHighest
    readonly property color surfaceContent: palette.onSurface
    readonly property color surfaceContentVariant: palette.onSurfaceVariant
    readonly property color outline: palette.outline
    readonly property color outlineVariant: palette.outlineVariant
    // Success never gets its own (green) color: it uses the theme's ink.
    readonly property color success: primary
    readonly property color warning: status.warning
    readonly property color error: status.error
    readonly property color errorContent: status.onError
    // A quiet error surface (the "sharing your screen" pill): the error hue
    // over the surface, with the surface's own ink so text stays readable.
    readonly property color errorContainer: Qt.tint(surfaceContainerHigh, Qt.rgba(error.r, error.g, error.b, dark ? 0.22 : 0.14))
    readonly property color errorContainerContent: surfaceContent

    // ---- Component colors (docs/design/assets/nl.css) ----
    readonly property color cardColor: Qt.tint(
        Qt.rgba(surfaceContainer.r, surfaceContainer.g, surfaceContainer.b, dark ? 0.36 : 0.52),
        Qt.rgba(primary.r, primary.g, primary.b, dark ? 0.08 : 0.06)
    )
    readonly property color cardBorder: dark ? Qt.rgba(1, 1, 1, 0.08) : Qt.rgba(1, 1, 1, 0.28)
    readonly property color tileColor: Qt.tint(
        Qt.rgba(1, 1, 1, dark ? 0.08 : 0.48),
        Qt.rgba(primary.r, primary.g, primary.b, dark ? 0.08 : 0.06)
    )
    readonly property color heroColor: Qt.tint(
        Qt.rgba(surface.r, surface.g, surface.b, dark ? 0.34 : 0.48),
        Qt.rgba(primaryContainer.r, primaryContainer.g, primaryContainer.b, dark ? 0.30 : 0.38)
    )
    readonly property color heroContent: primaryContainerContent
    readonly property color railColor: Qt.rgba(surfaceLow.r, surfaceLow.g, surfaceLow.b, dark ? 0.28 : 0.42)
    readonly property color indicator: secondaryContainer
    readonly property color indicatorContent: secondaryContainerContent
    // The halo around a primary control on hover.
    readonly property color glow: Qt.rgba(primary.r, primary.g, primary.b, dark ? 0.30 : 0.22)
    readonly property color buttonShadow: "transparent"
    readonly property color scrim: Qt.rgba(0, 0, 0, dark ? 0.5 : 0.32)

    // ---- Shape ----
    readonly property var shape: Tokens.data.themes.bloom.shape
    readonly property int radiusXs: shape.xs
    readonly property int radiusSm: shape.sm
    readonly property int radiusMd: shape.md
    readonly property int radiusLg: shape.lg
    readonly property int radiusXl: shape.xl
    // "Pill": half the height of whatever uses it.
    function pill(height) { return height / 2 }

    // ---- Type ----
    // Bundled with the app (assets/fonts).
    readonly property string fontUi: "Figtree"
    readonly property string fontDisplay: "Figtree"
    readonly property string fontMono: "Space Mono"
    // Caption button glyphs: Segoe Fluent Icons ships with Windows 11 only;
    // Windows 10's Segoe MDL2 Assets has the same code points.
    readonly property string captionIconFont: Qt.fontFamilies().indexOf("Segoe Fluent Icons") >= 0
        ? "Segoe Fluent Icons" : "Segoe MDL2 Assets"
    readonly property var typography: Tokens.data.typography
    readonly property var labelStyle: Tokens.data.themes.bloom.label

    // ---- Layout and spacing ----
    readonly property var layout: Tokens.data.layout.desktop
    readonly property int railWidth: layout.railWidth
    readonly property int topBarHeight: layout.topBarHeight
    readonly property int contentPadding: layout.contentPadding
    readonly property int gutter: layout.gutter
    function space(step) { return Tokens.data.spacing.scale[step] }

    // Focus rings show for keyboard focus only, not after a click (TapHandler
    // doesn't take activeFocus on click; activeFocus on a control comes from Tab).
    function focusVisible(item) {
        return item !== null && item !== undefined && item.activeFocus
            && (item.focusReason === undefined
                || item.focusReason === Qt.TabFocusReason
                || item.focusReason === Qt.BacktabFocusReason)
    }

    // ---- Motion ----
    // QML springs take a strength and a damping ratio; these are tuned to
    // match the token springs (gentle / standard / snappy).
    readonly property real springGentle: 2.4
    readonly property real dampingGentle: 0.36
    readonly property real springStandard: 3.6
    readonly property real dampingStandard: 0.32
    readonly property real springSnappy: 5.6
    readonly property real dampingSnappy: 0.34
    readonly property int fadeFast: Tokens.data.motion.fade.fast
    readonly property int fadeNormal: Tokens.data.motion.fade.normal
    readonly property real hoverLift: reduceMotion ? 0 : Tokens.data.motion.hoverLift
    readonly property real pressScale: reduceMotion ? 1 : Tokens.data.motion.pressScale
}
