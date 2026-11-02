// SPDX-License-Identifier: GPL-3.0-or-later
package app.noddy.android.ui.theme

import android.animation.ValueAnimator
import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner

val LocalReducedMotion = compositionLocalOf { false }

/** True when Android's "Remove animations" (animator duration scale 0) is enabled. */
@Composable
fun rememberReducedMotion(): Boolean {
    var reduced by remember { mutableStateOf(!ValueAnimator.areAnimatorsEnabled()) }
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                reduced = !ValueAnimator.areAnimatorsEnabled()
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
    return reduced
}

/** The user's look choice (see Preferences): light, dark, or follow the system. */
@androidx.compose.runtime.Immutable
data class Appearance(
    val mode: String = "system",
)

private fun Palette.toScheme(dark: Boolean, error: Color): ColorScheme {
    val base = if (dark) darkColorScheme() else lightColorScheme()
    return base.copy(
        primary = primary,
        onPrimary = onPrimary,
        primaryContainer = primaryContainer,
        onPrimaryContainer = onPrimaryContainer,
        secondary = primary,
        onSecondary = onPrimary,
        secondaryContainer = secondaryContainer,
        onSecondaryContainer = onSecondaryContainer,
        tertiary = primary,
        onTertiary = onPrimary,
        tertiaryContainer = secondaryContainer,
        onTertiaryContainer = onSecondaryContainer,
        background = surface,
        onBackground = onSurface,
        surface = surface,
        onSurface = onSurface,
        surfaceVariant = surfaceContainerHigh,
        onSurfaceVariant = onSurfaceVariant,
        surfaceContainerLowest = surface,
        surfaceContainerLow = surfaceContainerLow,
        surfaceContainer = surfaceContainer,
        surfaceContainerHigh = surfaceContainerHigh,
        surfaceContainerHighest = surfaceContainerHighest,
        outline = outline,
        outlineVariant = outlineVariant,
        error = error,
    )
}

/** The app's fixed palette: near-black (or near-white) surfaces with one orange accent. */
private fun noddyScheme(dark: Boolean): ColorScheme {
    val error = if (dark) Tokens.errorDark else Tokens.errorLight
    val seed = Tokens.glassSeeds.getValue(Tokens.DEFAULT_SEED)
    return (if (dark) seed.dark else seed.light).toScheme(dark, error)
}

/** Large, soft radii: round buttons, pill rows and generous cards. */
private val noddyShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(24.dp),
    extraLarge = RoundedCornerShape(32.dp),
)

/** Switch colors: an orange track when on, a quiet one when off. */
@Composable
fun noddySwitchColors(): androidx.compose.material3.SwitchColors {
    val scheme = MaterialTheme.colorScheme
    return androidx.compose.material3.SwitchDefaults.colors(
        checkedThumbColor = Color.White,
        checkedTrackColor = scheme.primary,
        checkedBorderColor = Color.Transparent,
        uncheckedThumbColor = scheme.onSurface.copy(alpha = 0.82f),
        uncheckedTrackColor = scheme.surfaceContainerHighest,
        uncheckedBorderColor = scheme.outlineVariant,
    )
}

/** The app's theme, light or dark. */
@Composable
fun NoddyTheme(
    appearance: Appearance = Appearance(),
    content: @Composable () -> Unit,
) {
    val systemDark = isSystemInDarkTheme()
    val dark = when (appearance.mode) {
        "light" -> false
        "dark" -> true
        else -> systemDark
    }
    val targetScheme = remember(dark) { noddyScheme(dark) }
    val reducedMotion = rememberReducedMotion()

    // System bar icons follow the app's theme, which can differ from the system's.
    val view = LocalView.current
    if (!view.isInEditMode) {
        androidx.compose.runtime.LaunchedEffect(view, dark) {
            val window = (view.context as? Activity)?.window ?: return@LaunchedEffect
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !dark
                isAppearanceLightNavigationBars = !dark
            }
        }
    }
    val assets = LocalContext.current.assets
    val fonts = remember(assets) { AppFonts(assets) }
    val currentTypography = remember(fonts) { typography(fonts) }
    CompositionLocalProvider(
        LocalAppFonts provides fonts,
        LocalReducedMotion provides reducedMotion,
    ) {
        MaterialTheme(
            colorScheme = targetScheme,
            shapes = noddyShapes,
            typography = currentTypography,
        ) {
            androidx.compose.foundation.layout.Box(
                Modifier
                    .fillMaxSize()
                    .background(targetScheme.background)
                    .noddyGlow(targetScheme.primary, dark),
            ) {
                content()
            }
        }
    }
}

/** A faint warm glow in the top-right corner, as in the design: dark mode only. */
private fun Modifier.noddyGlow(accent: Color, dark: Boolean): Modifier =
    if (!dark) this else this.drawWithCache {
        val brush = androidx.compose.ui.graphics.Brush.radialGradient(
            colors = listOf(accent.copy(alpha = 0.20f), Color.Transparent),
            center = Offset(size.width, 0f),
            radius = size.maxDimension * 0.75f,
        )
        onDrawBehind { drawRect(brush) }
    }

/** A solid rounded surface: the one card style of the app. */
fun Modifier.softCard(cornerRadius: Dp = 24.dp, baseColor: Color): Modifier =
    this.drawWithCache {
        val radiusPx = cornerRadius.toPx()
        val cr = CornerRadius(radiusPx, radiusPx)
        onDrawBehind { drawRoundRect(color = baseColor, cornerRadius = cr) }
    }

/** A smaller solid well inside a card: rows, strips and chips. */
fun Modifier.insetWell(cornerRadius: Dp = 16.dp, fillColor: Color): Modifier =
    softCard(cornerRadius, fillColor)

/** The container color of modal sheets and dialogs. */
@Composable
fun modalContainerColor(): Color = MaterialTheme.colorScheme.surfaceContainerHigh

/** The dimmed backdrop behind sheets and dialogs. */
@Composable
fun modalScrimColor(): Color = Color.Black.copy(alpha = 0.62f)
