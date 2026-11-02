// SPDX-License-Identifier: GPL-3.0-or-later
package app.noddy.android.ui.remote

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.noddy.android.R
import app.noddy.android.core.Core
import app.noddy.android.core.Device
import app.noddy.android.ui.theme.PadKey
import app.noddy.android.ui.theme.PillButton
import app.noddy.android.ui.theme.RingPad

/** The PC's volume moves by this much per press. */
private const val VOLUME_STEP = 5

/**
 * The pad: a glowing ring with the arrow keys around one orange Enter button, volume
 * and paging on curved side strips, and the keys a remote has under it.
 */
@Composable
fun RemotePadView(
    device: Device,
    core: Core,
    onStatus: (Core.RemoteAccess) -> Unit,
    modifier: Modifier = Modifier,
    /** What sits under BACK / HOME / EXIT; the default is a row of Tab, Space and Task view. */
    footer: (@Composable androidx.compose.foundation.layout.ColumnScope.() -> Unit)? = null,
) {
    val id = device.id
    val on = device.online
    val key: (String, List<String>) -> Unit = { name, mods -> core.remoteKey(id, name, mods, onStatus) }
    val volume = device.deckState?.volume?.toInt() ?: 50
    val setVolume: (Int) -> Unit = { next -> core.setPcAudio(id, volume = next.coerceIn(0, 100)) }

    Column(
        modifier = modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(20.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            SideStrip(
                top = PadKey(
                    painterResource(R.drawable.ic_plus),
                    stringResource(R.string.remote_pad_volume_up),
                    { setVolume(volume + VOLUME_STEP) },
                    on,
                ),
                middle = painterResource(R.drawable.ic_volume),
                bottom = PadKey(
                    painterResource(R.drawable.ic_minus),
                    stringResource(R.string.remote_pad_volume_down),
                    { setVolume(volume - VOLUME_STEP) },
                    on,
                ),
            )
            RingPad(
                modifier = Modifier.weight(1f),
                center = PadKey(
                    icon = null,
                    label = stringResource(R.string.remote_pad_ok_desc),
                    onClick = { key("enter", emptyList()) },
                    enabled = on,
                    shortLabel = stringResource(R.string.remote_pad_ok),
                ),
                top = PadKey(painterResource(R.drawable.ic_chevron_up), stringResource(R.string.remote_key_up_desc), { key("up", emptyList()) }, on),
                bottom = PadKey(painterResource(R.drawable.ic_chevron_down), stringResource(R.string.remote_key_down_desc), { key("down", emptyList()) }, on),
                left = PadKey(painterResource(R.drawable.ic_chevron_left), stringResource(R.string.remote_key_left_desc), { key("left", emptyList()) }, on),
                right = PadKey(painterResource(R.drawable.ic_chevron_right), stringResource(R.string.remote_key_right_desc), { key("right", emptyList()) }, on),
            )
            SideStrip(
                top = PadKey(
                    painterResource(R.drawable.ic_chevron_up),
                    stringResource(R.string.remote_pad_page_up),
                    { key("page_up", emptyList()) },
                    on,
                ),
                middleLabel = stringResource(R.string.remote_pad_page),
                bottom = PadKey(
                    painterResource(R.drawable.ic_chevron_down),
                    stringResource(R.string.remote_pad_page_down),
                    { key("page_down", emptyList()) },
                    on,
                ),
            )
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly,
        ) {
            LabelCircle(stringResource(R.string.remote_pad_back), on) { key("left", listOf("alt")) }
            LabelCircle(stringResource(R.string.remote_key_home), on) { key("home", emptyList()) }
            LabelCircle(stringResource(R.string.remote_pad_exit), on) { key("escape", emptyList()) }
        }

        if (footer != null) {
            footer()
        } else {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                PillButton(stringResource(R.string.remote_key_tab), { key("tab", emptyList()) }, Modifier.weight(1f), enabled = on)
                PillButton(stringResource(R.string.remote_pad_space), { key("space", emptyList()) }, Modifier.weight(1f), enabled = on)
                PillButton(stringResource(R.string.remote_pad_task_view), { key("task_view", emptyList()) }, Modifier.weight(1.3f), enabled = on)
            }
        }
    }
}

/** A tall rounded strip with a key at each end (volume, paging), like the sides of a remote. */
@Composable
private fun SideStrip(
    top: PadKey,
    bottom: PadKey,
    middle: androidx.compose.ui.graphics.painter.Painter? = null,
    middleLabel: String? = null,
) {
    val scheme = MaterialTheme.colorScheme
    Column(
        modifier = Modifier
            .width(52.dp)
            .height(190.dp)
            .clip(RoundedCornerShape(28.dp))
            .drawBehind { drawRoundRect(scheme.surfaceContainer, cornerRadius = CornerRadius(28.dp.toPx())) },
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        StripKey(top)
        if (middle != null) {
            Icon(middle, contentDescription = null, tint = scheme.onSurfaceVariant, modifier = Modifier.size(22.dp))
        } else if (middleLabel != null) {
            Text(middleLabel, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        }
        StripKey(bottom)
    }
}

@Composable
private fun StripKey(key: PadKey) {
    Box(
        modifier = Modifier
            .size(52.dp)
            .clickable(enabled = key.enabled, role = Role.Button, onClick = key.onClick)
            .semantics { contentDescription = key.label },
        contentAlignment = Alignment.Center,
    ) {
        key.icon?.let { Icon(it, contentDescription = null, tint = MaterialTheme.colorScheme.onSurface, modifier = Modifier.size(22.dp)) }
    }
}

/** A round dark button with a word in it: BACK, HOME, EXIT. */
@Composable
private fun LabelCircle(label: String, enabled: Boolean, onClick: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Box(
        modifier = Modifier
            .size(76.dp)
            .clip(androidx.compose.foundation.shape.CircleShape)
            .drawBehind { drawCircle(scheme.surfaceContainer) }
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .semantics { contentDescription = label }
            .alpha(if (enabled) 1f else 0.38f),
        contentAlignment = Alignment.Center,
    ) {
        Text(label.uppercase(), style = MaterialTheme.typography.labelLarge, color = scheme.onSurface, maxLines = 1)
    }
}
