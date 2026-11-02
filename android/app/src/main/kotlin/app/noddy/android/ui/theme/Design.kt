// SPDX-License-Identifier: GPL-3.0-or-later
package app.noddy.android.ui.theme

import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.painter.Painter
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/*
 * The app's shared building blocks, after the remote-control design: solid dark round
 * buttons, pill rows, a glowing ring pad with one orange action in the middle, and a
 * rounded dock. Everything is a solid color from the theme, never translucent.
 */

private const val DISABLED_ALPHA = 0.38f

/** A solid round button holding one icon. */
@Composable
fun RoundIconButton(
    icon: Painter,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    size: Dp = 52.dp,
    enabled: Boolean = true,
    tint: Color = MaterialTheme.colorScheme.onSurface,
    container: Color = MaterialTheme.colorScheme.surfaceContainerHigh,
) {
    Box(
        modifier = modifier
            .size(size)
            .clip(CircleShape)
            .drawBehind { drawCircle(container) }
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .semantics { this.contentDescription = contentDescription }
            .alpha(if (enabled) 1f else DISABLED_ALPHA),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icon, contentDescription = null, tint = tint, modifier = Modifier.size(size * 0.42f))
    }
}

/**
 * A rounded pill button with an optional icon. [accent] fills it orange (the one main
 * action of a row); otherwise it is a quiet dark pill.
 */
@Composable
fun PillButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    icon: Painter? = null,
    enabled: Boolean = true,
    accent: Boolean = false,
) {
    val scheme = MaterialTheme.colorScheme
    val container = if (accent) scheme.primary else scheme.surfaceContainerHigh
    val content = if (accent) scheme.onPrimary else scheme.onSurface
    Row(
        modifier = modifier
            .heightIn(min = 48.dp)
            .clip(CircleShape)
            .drawBehind { drawRect(container) }
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .alpha(if (enabled) 1f else DISABLED_ALPHA)
            .padding(horizontal = 18.dp, vertical = 10.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) Icon(icon, contentDescription = null, tint = content, modifier = Modifier.size(18.dp))
        Text(
            label,
            style = MaterialTheme.typography.labelLarge,
            color = content,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

/** A small muted caption above a group of controls. */
@Composable
fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Text(
        text,
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = modifier.padding(start = 6.dp),
    )
}

/** One key around the ring: a round dark button with an icon and a tiny label. */
data class PadKey(
    val icon: Painter?,
    val label: String,
    val onClick: () -> Unit,
    val enabled: Boolean = true,
    /** What shows on a key without an icon ("OK"); the label is for screen readers. */
    val shortLabel: String = label,
)

/**
 * The glowing ring pad: a ring with an orange glow, four keys on a dark disc around one
 * large orange button ([center]). Pass `null` to leave a position empty.
 */
@Composable
fun RingPad(
    center: PadKey,
    top: PadKey?,
    left: PadKey?,
    right: PadKey?,
    bottom: PadKey?,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    val reduced = LocalReducedMotion.current
    BoxWithConstraints(
        modifier = modifier
            .aspectRatio(1f)
            .drawBehind {
                val stroke = 5.dp.toPx()
                val radius = size.minDimension / 2f - 14.dp.toPx()
                val c = Offset(size.width / 2f, size.height / 2f)
                // The glow: wider and fainter strokes under the crisp ring.
                val layers = if (reduced) 3 else 7
                for (i in layers downTo 1) {
                    drawCircle(
                        color = scheme.primary.copy(alpha = 0.05f + 0.02f * (layers - i)),
                        radius = radius,
                        center = c,
                        style = Stroke(width = stroke + i * 4.dp.toPx()),
                    )
                }
                // The dark disc inside the ring, then the ring itself.
                drawCircle(color = scheme.surfaceContainerLow, radius = radius - stroke / 2f, center = c)
                drawCircle(color = scheme.primary, radius = radius, center = c, style = Stroke(width = stroke))
            },
        contentAlignment = Alignment.Center,
    ) {
        // Everything scales with the ring, so it fits between the side strips.
        val ring = maxWidth
        val inset = ring * 0.12f
        val satellite = ring * 0.21f
        top?.let { PadSatellite(it, satellite, Modifier.align(Alignment.TopCenter).padding(top = inset)) }
        bottom?.let { PadSatellite(it, satellite, Modifier.align(Alignment.BottomCenter).padding(bottom = inset)) }
        left?.let { PadSatellite(it, satellite, Modifier.align(Alignment.CenterStart).padding(start = inset)) }
        right?.let { PadSatellite(it, satellite, Modifier.align(Alignment.CenterEnd).padding(end = inset)) }
        Column(
            modifier = Modifier
                .size(ring * 0.34f)
                .clip(CircleShape)
                .drawBehind { drawCircle(scheme.primary) }
                .clickable(enabled = center.enabled, role = Role.Button, onClick = center.onClick)
                .semantics { this.contentDescription = center.label }
                .alpha(if (center.enabled) 1f else DISABLED_ALPHA),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            center.icon?.let {
                Icon(it, contentDescription = null, tint = scheme.onPrimary, modifier = Modifier.size(30.dp))
                Spacer(Modifier.size(2.dp))
            }
            Text(
                if (center.icon == null) center.shortLabel else center.label,
                style = if (center.icon == null) MaterialTheme.typography.titleLarge else MaterialTheme.typography.labelMedium,
                color = scheme.onPrimary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
private fun PadSatellite(key: PadKey, size: Dp, modifier: Modifier) {
    val scheme = MaterialTheme.colorScheme
    Column(
        modifier = modifier
            .size(size)
            .clip(CircleShape)
            .drawBehind { drawCircle(scheme.surfaceContainerHighest) }
            .clickable(enabled = key.enabled, role = Role.Button, onClick = key.onClick)
            .semantics { this.contentDescription = key.label }
            .alpha(if (key.enabled) 1f else DISABLED_ALPHA),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        if (key.icon != null) {
            Icon(key.icon, contentDescription = null, tint = scheme.onSurface, modifier = Modifier.size(size * 0.4f))
        } else {
            Text(
                key.shortLabel,
                style = MaterialTheme.typography.labelMedium,
                color = scheme.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(horizontal = 4.dp),
            )
        }
    }
}

/** One dock item: its icon, an optional status dot, and a label for screen readers. */
data class DockItem(
    val icon: Painter,
    val label: String,
    val dot: Color? = null,
)

/**
 * The bottom dock: a rounded dark bar of icons. The selected one sits on a lighter rounded
 * tile with an orange icon.
 */
@Composable
fun BottomDock(
    items: List<DockItem>,
    selectedIndex: Int,
    onSelect: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    Row(
        modifier = modifier
            .fillMaxWidth()
            .softCard(cornerRadius = 32.dp, baseColor = scheme.surfaceContainer)
            .padding(horizontal = 12.dp, vertical = 10.dp)
            .selectableGroup(),
        horizontalArrangement = Arrangement.SpaceEvenly,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        items.forEachIndexed { index, item ->
            val selected = index == selectedIndex
            Box(
                modifier = Modifier
                    .size(56.dp)
                    .clip(RoundedCornerShape(20.dp))
                    .drawBehind {
                        if (selected) drawRect(scheme.surfaceContainerHighest)
                    }
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                        role = Role.Tab,
                        onClick = { onSelect(index) },
                    )
                    .semantics {
                        this.contentDescription = item.label
                        this.selected = selected
                        role = Role.Tab
                    },
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    item.icon,
                    contentDescription = null,
                    tint = if (selected) scheme.primary else scheme.onSurfaceVariant,
                    modifier = Modifier.size(26.dp),
                )
                item.dot?.let { dot ->
                    Box(
                        Modifier
                            .align(Alignment.TopEnd)
                            .padding(10.dp)
                            .size(7.dp)
                            .drawBehind { drawCircle(dot) },
                    )
                }
            }
        }
    }
}
