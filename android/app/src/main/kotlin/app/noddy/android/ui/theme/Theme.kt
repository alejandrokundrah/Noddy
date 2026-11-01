// SPDX-License-Identifier: GPL-3.0-or-later
package app.noddy.android.ui.theme

import android.animation.ValueAnimator
import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
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
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.core.graphics.ColorUtils
import androidx.core.view.WindowCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner

val LocalReducedMotion = compositionLocalOf { false }

/** True in Glass (translucent cards); false in Ember (solid cards with a hairline). */
val LocalGlassTheme = compositionLocalOf { true }

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

/**
 * The user's look choice (see Preferences): the theme ("glass" or "ember") and, for Glass,
 * light, dark, or follow the system. Ember is always dark.
 */
@androidx.compose.runtime.Immutable
data class Appearance(
    val theme: String = "glass",
    val mode: String = "system",
) {
    val isEmber: Boolean get() = theme == "ember"
}

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

/**
 * Applies adaptive surface translucency in Glass mode with a subtle uniform
 * hairline border and high-legibility modal containers.
 */
private fun ColorScheme.toGlassScheme(dark: Boolean): ColorScheme {
    val tintArgb = primary.let {
        android.graphics.Color.argb(
            (it.alpha * 255).toInt(),
            (it.red * 255).toInt(),
            (it.green * 255).toInt(),
            (it.blue * 255).toInt(),
        )
    }
    fun blendWithTint(c: Color, ratio: Float, alpha: Float): Color {
        val cArgb = android.graphics.Color.argb(
            255,
            (c.red * 255).toInt(),
            (c.green * 255).toInt(),
            (c.blue * 255).toInt(),
        )
        return Color(ColorUtils.blendARGB(cArgb, tintArgb, ratio)).copy(alpha = alpha)
    }
    return if (dark) {
        copy(
            surfaceContainerLowest = surface.copy(alpha = 0.52f),
            surfaceContainerLow = blendWithTint(surfaceContainerLow, 0.08f, 0.62f),
            surfaceContainer = blendWithTint(surfaceContainer, 0.10f, 0.72f),
            surfaceContainerHigh = blendWithTint(surfaceContainerHigh, 0.12f, 0.88f),
            surfaceContainerHighest = blendWithTint(surfaceContainerHighest, 0.14f, 0.94f),
            surfaceVariant = blendWithTint(surfaceVariant, 0.10f, 0.78f),
            primaryContainer = primaryContainer.copy(alpha = 0.82f),
            secondaryContainer = secondaryContainer.copy(alpha = 0.78f),
            tertiaryContainer = tertiaryContainer.copy(alpha = 0.80f),
            outlineVariant = Color.White.copy(alpha = 0.12f),
        )
    } else {
        copy(
            surfaceContainerLowest = Color.White.copy(alpha = 0.64f),
            surfaceContainerLow = blendWithTint(Color.White, 0.04f, 0.74f),
            surfaceContainer = blendWithTint(Color.White, 0.06f, 0.82f),
            surfaceContainerHigh = blendWithTint(surfaceContainerHigh, 0.10f, 0.92f),
            surfaceContainerHighest = blendWithTint(surfaceContainerHighest, 0.12f, 0.96f),
            surfaceVariant = blendWithTint(surfaceVariant, 0.08f, 0.84f),
            primaryContainer = primaryContainer.copy(alpha = 0.86f),
            secondaryContainer = secondaryContainer.copy(alpha = 0.84f),
            tertiaryContainer = tertiaryContainer.copy(alpha = 0.86f),
            outlineVariant = primary.copy(alpha = 0.18f),
        )
    }
}

/** Glass's fixed palette: the Honey preset, light and dark. */
private fun glassScheme(dark: Boolean): ColorScheme {
    val error = if (dark) Tokens.errorDark else Tokens.errorLight
    val seed = Tokens.bloomSeeds.getValue(Tokens.DEFAULT_SEED)
    return (if (dark) seed.dark else seed.light).toScheme(dark, error).toGlassScheme(dark)
}

/** Ember's fixed palette: near-black surfaces with one orange accent (always dark). */
private fun emberScheme(): ColorScheme =
    Tokens.emberDark.toScheme(true, Tokens.emberError).copy(onError = Tokens.emberOnError)

/** Ember shapes from the tokens: large radii. */
private val emberShapes = Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(24.dp),
    extraLarge = RoundedCornerShape(32.dp),
)

/** Glass shapes from the tokens: smoothly rounded. */
private val glassShapes = Shapes(
    extraSmall = RoundedCornerShape(10.dp),
    small = RoundedCornerShape(14.dp),
    medium = RoundedCornerShape(20.dp),
    large = RoundedCornerShape(24.dp),
    extraLarge = RoundedCornerShape(30.dp),
)

/** Frosted Switch colors. */
@Composable
fun nectarSwitchColors(): androidx.compose.material3.SwitchColors {
    val scheme = MaterialTheme.colorScheme
    return androidx.compose.material3.SwitchDefaults.colors(
        checkedThumbColor = Color.White,
        checkedTrackColor = scheme.primary.copy(alpha = 0.62f),
        checkedBorderColor = scheme.outlineVariant,
        uncheckedThumbColor = scheme.onSurface.copy(alpha = 0.82f),
        uncheckedTrackColor = scheme.surfaceContainerHighest.copy(alpha = 0.42f),
        uncheckedBorderColor = scheme.outlineVariant,
    )
}

/** The app's theme: Glass (light or dark) or Ember (dark only). */
@Composable
fun NoddyTheme(
    appearance: Appearance = Appearance(),
    content: @Composable () -> Unit,
) {
    val ember = appearance.isEmber
    val systemDark = isSystemInDarkTheme()
    val dark = ember || when (appearance.mode) {
        "light" -> false
        "dark" -> true
        else -> systemDark
    }
    val targetScheme = remember(dark, ember) { if (ember) emberScheme() else glassScheme(dark) }
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
        LocalGlassTheme provides !ember,
    ) {
        MaterialTheme(
            colorScheme = targetScheme,
            shapes = if (ember) emberShapes else glassShapes,
            typography = currentTypography,
        ) {
            androidx.compose.foundation.layout.Box(Modifier.fillMaxSize()) {
                content()
            }
        }
    }
}

/**
 * Single-pass cached card background with a subtle uniform hairline border and no top-edge glare.
 */
fun Modifier.liquidGlassCard(
    cornerRadius: Dp = 24.dp,
    isGlass: Boolean,
    baseColor: Color,
    @Suppress("UNUSED_PARAMETER") tintColor: Color = Color.Transparent,
    outlineColor: Color = Color.Transparent,
): Modifier = this.drawWithCache {
    val radiusPx = cornerRadius.toPx()
    val cr = CornerRadius(radiusPx, radiusPx)
    val strokePx = 1.dp.toPx()
    val halfStroke = strokePx * 0.5f
    val topLeftOffset = Offset(halfStroke, halfStroke)
    val insetSize = Size(
        (size.width - strokePx).coerceAtLeast(0f),
        (size.height - strokePx).coerceAtLeast(0f),
    )
    val isDarkSurface = (baseColor.red * 0.299f + baseColor.green * 0.587f + baseColor.blue * 0.114f) < 0.5f
    val uniformRimColor = if (isGlass) {
        if (outlineColor.alpha > 0f) {
            outlineColor.copy(alpha = outlineColor.alpha.coerceAtMost(if (isDarkSurface) 0.14f else 0.22f))
        } else {
            Color.White.copy(alpha = if (isDarkSurface) 0.10f else 0.22f)
        }
    } else {
        outlineColor
    }
    val strokeStyle = Stroke(width = strokePx)
    onDrawBehind {
        drawRoundRect(color = baseColor, cornerRadius = cr)
        if (uniformRimColor.alpha > 0f) {
            drawRoundRect(
                color = uniformRimColor,
                topLeft = topLeftOffset,
                size = insetSize,
                cornerRadius = cr,
                style = strokeStyle,
            )
        }
    }
}

/**
 * No-op modifier kept for source compatibility; modal sheets use [glassModalScrimColor]
 * and [glassModalContainerColor] instead of blurring the entire root viewport into an FBO.
 */
fun Modifier.glassModalBlur(@Suppress("UNUSED_PARAMETER") modalOpen: Boolean): Modifier = this

/** High-legibility adaptive frosted container color for modal sheets and dialogs. */
@Composable
fun glassModalContainerColor(): Color {
    val scheme = MaterialTheme.colorScheme
    val luma = scheme.surface.red * 0.299f + scheme.surface.green * 0.587f + scheme.surface.blue * 0.114f
    val base = if (luma < 0.5f) Color(0xFF141720) else Color(0xFFF7F8FC)
    val tinted = Color(
        ColorUtils.blendARGB(
            android.graphics.Color.argb(255, (base.red * 255).toInt(), (base.green * 255).toInt(), (base.blue * 255).toInt()),
            android.graphics.Color.argb(255, (scheme.primary.red * 255).toInt(), (scheme.primary.green * 255).toInt(), (scheme.primary.blue * 255).toInt()),
            if (luma < 0.5f) 0.12f else 0.06f,
        ),
    )
    return tinted.copy(alpha = if (luma < 0.5f) 0.95f else 0.97f)
}

/** Deepened backdrop scrim color when sheets or dialogs open so background text never bleeds through. */
@Composable
fun glassModalScrimColor(): Color = Color.Black.copy(alpha = 0.62f)

/**
 * Lightweight zero-RenderNode interior well for bento strips and inner rows.
 */
fun Modifier.bentoWell(
    cornerRadius: Dp = 16.dp,
    fillColor: Color,
    borderColor: Color = Color.Transparent,
): Modifier = this.drawWithCache {
    val r = cornerRadius.toPx()
    val cr = CornerRadius(r, r)
    val strokePx = 1.dp.toPx()
    val halfStroke = strokePx * 0.5f
    val topLeftOffset = Offset(halfStroke, halfStroke)
    val insetSize = Size(
        (size.width - strokePx).coerceAtLeast(0f),
        (size.height - strokePx).coerceAtLeast(0f),
    )
    val strokeStyle = Stroke(width = strokePx)
    onDrawBehind {
        drawRoundRect(color = fillColor, cornerRadius = cr)
        if (borderColor.alpha > 0f) {
            drawRoundRect(
                color = borderColor,
                topLeft = topLeftOffset,
                size = insetSize,
                cornerRadius = cr,
                style = strokeStyle,
            )
        }
    }
}

