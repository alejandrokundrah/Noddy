// SPDX-License-Identifier: GPL-3.0-or-later
// Generated from docs/design/tokens.json by `cargo xtask tokens`. Do not edit.
package app.noddy.android.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

/** Material color roles of one theme variant. */
@Immutable
data class Palette(
    val primary: Color,
    val onPrimary: Color,
    val primaryContainer: Color,
    val onPrimaryContainer: Color,
    val secondaryContainer: Color,
    val onSecondaryContainer: Color,
    val surface: Color,
    val surfaceContainerLow: Color,
    val surfaceContainer: Color,
    val surfaceContainerHigh: Color,
    val surfaceContainerHighest: Color,
    val onSurface: Color,
    val onSurfaceVariant: Color,
    val outline: Color,
    val outlineVariant: Color,
)

/** A Glass color preset: its seed swatch and light and dark palettes. */
@Immutable
data class Seed(val color: Color, val light: Palette, val dark: Palette)

object Tokens {
    const val DEFAULT_SEED = "orange"

    val glassSeeds: Map<String, Seed> = linkedMapOf(
        "orange" to Seed(
            color = Color(0xFFE85C29),
            light = Palette(
                primary = Color(0xFFB23808),
                onPrimary = Color(0xFFFFFFFF),
                primaryContainer = Color(0xFFFFDBCE),
                onPrimaryContainer = Color(0xFF3A0F00),
                secondaryContainer = Color(0xFFE9E9EC),
                onSecondaryContainer = Color(0xFF1B1B1E),
                surface = Color(0xFFFAFAFA),
                surfaceContainerLow = Color(0xFFF4F4F5),
                surfaceContainer = Color(0xFFEEEEF0),
                surfaceContainerHigh = Color(0xFFE8E8EA),
                surfaceContainerHighest = Color(0xFFE2E2E5),
                onSurface = Color(0xFF18181B),
                onSurfaceVariant = Color(0xFF52525B),
                outline = Color(0xFF8A8A92),
                outlineVariant = Color(0xFFD4D4D8),
            ),
            dark = Palette(
                primary = Color(0xFFEB6030),
                onPrimary = Color(0xFF2A0D00),
                primaryContainer = Color(0xFF5A2412),
                onPrimaryContainer = Color(0xFFFFDBCE),
                secondaryContainer = Color(0xFF2C2C30),
                onSecondaryContainer = Color(0xFFEDEDEF),
                surface = Color(0xFF0A0A0B),
                surfaceContainerLow = Color(0xFF0E0E0F),
                surfaceContainer = Color(0xFF141415),
                surfaceContainerHigh = Color(0xFF1B1B1D),
                surfaceContainerHighest = Color(0xFF242427),
                onSurface = Color(0xFFF4F4F5),
                onSurfaceVariant = Color(0xFFA1A1A6),
                outline = Color(0xFF6E6E74),
                outlineVariant = Color(0xFF2A2A2D),
            ),
        ),
    )

    val emberDark = Palette(
        primary = Color(0xFFEB6030),
        onPrimary = Color(0xFF2A0D00),
        primaryContainer = Color(0xFF5A2412),
        onPrimaryContainer = Color(0xFFFFDBCE),
        secondaryContainer = Color(0xFF2C2C30),
        onSecondaryContainer = Color(0xFFEDEDEF),
        surface = Color(0xFF0A0A0B),
        surfaceContainerLow = Color(0xFF0E0E0F),
        surfaceContainer = Color(0xFF141415),
        surfaceContainerHigh = Color(0xFF1B1B1D),
        surfaceContainerHighest = Color(0xFF242427),
        onSurface = Color(0xFFF4F4F5),
        onSurfaceVariant = Color(0xFFA1A1A6),
        outline = Color(0xFF6E6E74),
        outlineVariant = Color(0xFF2A2A2D),
    )

    val emberWarning = Color(0xFFF2C14E)
    val emberError = Color(0xFFFF8A7A)
    val emberOnError = Color(0xFF4A0A02)

    val warningLight = Color(0xFF7D5200)
    val warningDark = Color(0xFFF2C14E)
    val errorLight = Color(0xFFBA1A1A)
    val errorDark = Color(0xFFFFB4AB)
}
