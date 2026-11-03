// SPDX-License-Identifier: GPL-3.0-or-later
package app.noddy.android.ui.home

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.noddy.android.R
import app.noddy.android.clipboard.SendActivity
import app.noddy.android.core.CoreState
import app.noddy.android.core.Device
import app.noddy.android.ui.theme.softCard

private const val DISABLED_ALPHA = 0.38f

/**
 * Home: a status card with quick actions, two big tiles (the PC's screen and its remote), a
 * notice row, and a grid of what this phone can do with the PC. Everything else (transfers,
 * media, notifications, history) is on the Devices tab.
 */
@Composable
fun HomeDashboard(
    state: CoreState,
    selectedPcId: String?,
    onSelectPc: (String) -> Unit,
    backgroundServiceEnabled: Boolean,
    micStreamingTo: String?,
    onPairNew: () -> Unit,
    onRefresh: () -> Unit,
    onWake: (pcId: String) -> Unit,
    onOpenRemote: (pcId: String) -> Unit,
    onOpenScreen: (pcId: String) -> Unit,
    onOpenDevices: () -> Unit,
    onOpenSettings: () -> Unit,
    onSendFiles: (pcId: String, uris: List<Uri>) -> Unit,
    onWebcam: (pcId: String) -> Unit,
    onRecord: (pcId: String) -> Unit,
    onDeck: (pcId: String) -> Unit,
    onStartMic: (pcId: String) -> Unit,
    onStopMic: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val pcs = state.devices.filter { !it.isAndroid }
    val pc = pcs.firstOrNull { it.id == selectedPcId } ?: pcs.firstOrNull { it.online } ?: pcs.firstOrNull()
    val ready = pc?.online == true
    val pickFiles = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        if (pc != null && uris.isNotEmpty()) onSendFiles(pc.id, uris)
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .statusBarsPadding()
            .navigationBarsPadding()
            // The dock floats over the bottom of the screen: leave room so the last row scrolls clear of it.
            .padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 96.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        StatusCard(
            pcs = pcs,
            pc = pc,
            onSelectPc = onSelectPc,
            onPairNew = onPairNew,
            onRefresh = onRefresh,
            onWake = { pc?.let { onWake(it.id) } },
            onOpenDevices = onOpenDevices,
        )

        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
            BigTile(
                label = stringResource(R.string.remote_mode_screen),
                accent = true,
                modifier = Modifier.weight(1f),
                onClick = { if (pc != null) onOpenScreen(pc.id) else onPairNew() },
                glyph = { ScreenGlyph(it) },
            )
            BigTile(
                label = stringResource(R.string.home_tile_remote),
                accent = false,
                modifier = Modifier.weight(1f),
                onClick = { if (pc != null) onOpenRemote(pc.id) else onPairNew() },
                glyph = { RemoteGlyph(it) },
            )
        }

        val (noticeText, noticeAction) = when {
            pc == null -> stringResource(R.string.home_dash_notice_none) to onPairNew
            !pc.online -> stringResource(R.string.home_dash_notice_offline, pc.name) to { onWake(pc.id) }
            !backgroundServiceEnabled -> stringResource(R.string.home_dash_notice_background) to onOpenSettings
            else -> stringResource(R.string.home_dash_notice_ok) to onOpenDevices
        }
        NoticeRow(text = noticeText, alert = pc == null || !pc.online || !backgroundServiceEnabled, onClick = noticeAction)

        FeatureGrid(
            tiles = listOf(
                FeatureTile(R.drawable.ic_folder, stringResource(R.string.home_tile_files), ready) {
                    pickFiles.launch(arrayOf("*/*"))
                },
                FeatureTile(R.drawable.ic_clipboard, stringResource(R.string.home_tile_clipboard), ready) {
                    pc?.let { context.startActivity(SendActivity.sendClipboardIntent(context, it.id)) }
                },
                FeatureTile(R.drawable.ic_camera, stringResource(R.string.action_webcam_pc), ready) {
                    pc?.let { onWebcam(it.id) }
                },
                FeatureTile(
                    R.drawable.ic_mic,
                    stringResource(R.string.home_stat_microphone),
                    ready || micStreamingTo != null,
                    active = micStreamingTo != null,
                ) {
                    if (micStreamingTo != null) onStopMic() else pc?.let { onStartMic(it.id) }
                },
                FeatureTile(R.drawable.ic_deck, stringResource(R.string.action_deck_pc), ready) {
                    pc?.let { onDeck(it.id) }
                },
                FeatureTile(R.drawable.ic_record, stringResource(R.string.action_record_pc), ready) {
                    pc?.let { onRecord(it.id) }
                },
            ),
        )
    }
}

/** The white card of the design, in the app's surface color: device chip, status, a graphic, four actions. */
@Composable
private fun StatusCard(
    pcs: List<Device>,
    pc: Device?,
    onSelectPc: (String) -> Unit,
    onPairNew: () -> Unit,
    onRefresh: () -> Unit,
    onWake: () -> Unit,
    onOpenDevices: () -> Unit,
) {
    val scheme = MaterialTheme.colorScheme
    var picking by remember { mutableStateOf(false) }
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .softCard(cornerRadius = 32.dp, baseColor = scheme.surfaceContainer),
    ) {
        Box(Modifier.fillMaxWidth()) {
            OrbitGraphic(
                Modifier
                    .align(Alignment.TopEnd)
                    .padding(top = 6.dp, end = 8.dp)
                    .size(width = 150.dp, height = 128.dp),
            )
            Column(
                modifier = Modifier.padding(start = 20.dp, top = 22.dp, end = 20.dp),
                verticalArrangement = Arrangement.spacedBy(18.dp),
            ) {
                Box {
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(16.dp))
                            .background(scheme.primary.copy(alpha = 0.16f))
                            .clickable(enabled = pcs.size > 1, role = Role.Button) { picking = true }
                            .padding(horizontal = 18.dp, vertical = 14.dp),
                    ) {
                        Text(
                            (pc?.name ?: stringResource(R.string.home_dash_chip)).uppercase(),
                            style = MaterialTheme.typography.titleMedium,
                            color = scheme.primary,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.widthIn(max = 190.dp),
                        )
                    }
                    DropdownMenu(expanded = picking, onDismissRequest = { picking = false }) {
                        for (candidate in pcs) {
                            DropdownMenuItem(
                                text = { Text(candidate.name) },
                                onClick = {
                                    picking = false
                                    onSelectPc(candidate.id)
                                },
                            )
                        }
                    }
                }
                Text(
                    when {
                        pc == null -> stringResource(R.string.home_dash_none)
                        pc.online -> stringResource(R.string.home_dash_connected, pc.name)
                        pc.reconnect -> stringResource(R.string.home_dash_searching, pc.name)
                        else -> stringResource(R.string.home_dash_offline, pc.name)
                    },
                    style = MaterialTheme.typography.headlineSmall,
                    color = scheme.onSurface,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.fillMaxWidth(0.62f),
                )
            }
        }
        Spacer(Modifier.height(16.dp))
        Row(
            modifier = Modifier.fillMaxWidth().padding(start = 12.dp, end = 12.dp, bottom = 18.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            val waking = pc != null && !pc.online
            QuickAction(R.drawable.ic_qr, stringResource(R.string.action_pair), true, onPairNew, Modifier.weight(1f))
            QuickDivider()
            QuickAction(R.drawable.ic_refresh, stringResource(R.string.action_refresh), true, onRefresh, Modifier.weight(1f))
            QuickDivider()
            QuickAction(R.drawable.ic_power, stringResource(R.string.action_wake_pc), waking, onWake, Modifier.weight(1f))
            QuickDivider()
            QuickAction(R.drawable.ic_devices, stringResource(R.string.home_dock_devices), true, onOpenDevices, Modifier.weight(1f))
        }
    }
}

@Composable
private fun QuickDivider() {
    Box(
        Modifier
            .width(1.dp)
            .height(22.dp)
            .background(MaterialTheme.colorScheme.outlineVariant),
    )
}

@Composable
private fun QuickAction(icon: Int, label: String, enabled: Boolean, onClick: () -> Unit, modifier: Modifier) {
    Box(
        modifier = modifier
            .height(48.dp)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .semantics { contentDescription = label }
            .alpha(if (enabled) 1f else DISABLED_ALPHA),
        contentAlignment = Alignment.Center,
    ) {
        Icon(painterResource(icon), contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(28.dp))
    }
}

/** The decorative graphic of the card: a glossy ring in the accent color with orbits and beads. */
@Composable
private fun OrbitGraphic(modifier: Modifier) {
    val scheme = MaterialTheme.colorScheme
    val light = androidx.compose.ui.graphics.lerp(scheme.primary, Color.White, 0.45f)
    val deep = androidx.compose.ui.graphics.lerp(scheme.primary, Color.Black, 0.35f)
    Box(
        modifier = modifier.drawBehind {
            val c = Offset(size.width * 0.52f, size.height * 0.52f)
            val orbit = Stroke(width = 1.2.dp.toPx())
            val orbitColor = scheme.primary.copy(alpha = 0.35f)
            rotate(-24f, pivot = c) {
                drawOval(orbitColor, Offset(c.x - size.width * 0.5f, c.y - size.height * 0.30f), Size(size.width, size.height * 0.60f), style = orbit)
            }
            rotate(28f, pivot = c) {
                drawOval(orbitColor, Offset(c.x - size.width * 0.44f, c.y - size.height * 0.40f), Size(size.width * 0.88f, size.height * 0.80f), style = orbit)
            }
            val ring = size.height * 0.30f
            val sweep = Brush.sweepGradient(listOf(light, scheme.primary, deep, scheme.primary, light), center = c)
            drawCircle(sweep, radius = ring, center = c, style = Stroke(width = 20.dp.toPx()))
            drawCircle(Color.White.copy(alpha = 0.35f), radius = ring + 8.dp.toPx(), center = c, style = Stroke(width = 1.5.dp.toPx()))
            drawCircle(deep.copy(alpha = 0.55f), radius = ring - 9.dp.toPx(), center = c, style = Stroke(width = 1.5.dp.toPx()))
            drawCircle(light, radius = 5.dp.toPx(), center = Offset(size.width * 0.12f, size.height * 0.30f))
            drawCircle(scheme.primary, radius = 4.dp.toPx(), center = Offset(size.width * 0.94f, size.height * 0.70f))
            drawCircle(light.copy(alpha = 0.8f), radius = 2.5.dp.toPx(), center = Offset(size.width * 0.20f, size.height * 0.80f))
        },
    )
}

/** One of the two big gradient tiles. [accent] is the orange one; the other is the dark warm one. */
@Composable
private fun BigTile(
    label: String,
    accent: Boolean,
    onClick: () -> Unit,
    glyph: @Composable (Color) -> Unit,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    val brush = if (accent) {
        Brush.linearGradient(listOf(scheme.primary, androidx.compose.ui.graphics.lerp(scheme.primary, Color.White, 0.22f)))
    } else {
        Brush.linearGradient(listOf(scheme.primaryContainer, scheme.surfaceContainerHigh))
    }
    val ink = if (accent) scheme.onPrimary else scheme.onPrimaryContainer
    Box(
        modifier = modifier
            .height(112.dp)
            .clip(RoundedCornerShape(28.dp))
            .background(brush)
            .clickable(role = Role.Button, onClick = onClick),
    ) {
        Text(
            label,
            style = MaterialTheme.typography.titleLarge,
            color = ink,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.padding(start = 20.dp, top = 18.dp, end = 12.dp),
        )
        Box(Modifier.align(Alignment.BottomEnd).padding(end = 16.dp, bottom = 14.dp).size(width = 78.dp, height = 56.dp)) {
            glyph(ink)
        }
    }
}

/** A small phone in front of a screen with a triangle: the PC's screen on this phone. */
@Composable
private fun ScreenGlyph(ink: Color) {
    Box(
        Modifier.fillMaxSize().drawBehind {
            drawRoundRect(ink.copy(alpha = 0.55f), Offset(size.width * 0.30f, 0f), Size(size.width * 0.70f, size.height * 0.88f), CornerRadius(10.dp.toPx()))
            drawRoundRect(ink.copy(alpha = 0.35f), Offset(0f, size.height * 0.28f), Size(size.width * 0.34f, size.height * 0.72f), CornerRadius(7.dp.toPx()))
            val tri = Path().apply {
                moveTo(size.width * 0.66f, size.height * 0.50f)
                lineTo(size.width * 0.80f, size.height * 0.78f)
                lineTo(size.width * 0.52f, size.height * 0.78f)
                close()
            }
            drawPath(tri, ink.copy(alpha = 0.9f))
        },
    )
}

/** A small pad and a round button: the PC's remote. */
@Composable
private fun RemoteGlyph(ink: Color) {
    Box(
        Modifier.fillMaxSize().drawBehind {
            drawRoundRect(ink.copy(alpha = 0.40f), Offset(0f, size.height * 0.10f), Size(size.width * 0.52f, size.height * 0.90f), CornerRadius(12.dp.toPx()))
            val dot = ink.copy(alpha = 0.9f)
            val c = Offset(size.width * 0.26f, size.height * 0.55f)
            for ((dx, dy) in listOf(0f to -11f, 0f to 11f, -11f to 0f, 11f to 0f)) {
                drawCircle(dot, radius = 2.5.dp.toPx(), center = Offset(c.x + dx.dp.toPx(), c.y + dy.dp.toPx()))
            }
            drawRoundRect(ink.copy(alpha = 0.55f), Offset(size.width * 0.46f, 0f), Size(size.width * 0.54f, size.height * 0.78f), CornerRadius(12.dp.toPx()))
            drawCircle(ink.copy(alpha = 0.95f), radius = 9.dp.toPx(), center = Offset(size.width * 0.73f, size.height * 0.39f), style = Stroke(width = 3.dp.toPx()))
        },
    )
}

/** The notice row: a bell, one or two lines, and a chevron. */
@Composable
private fun NoticeRow(text: String, alert: Boolean, onClick: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .softCard(cornerRadius = 28.dp, baseColor = scheme.surfaceContainer)
            .clip(RoundedCornerShape(28.dp))
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 18.dp, vertical = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Box {
            Icon(painterResource(R.drawable.ic_bell), contentDescription = null, tint = scheme.primary, modifier = Modifier.size(28.dp))
            if (alert) {
                Box(
                    Modifier
                        .align(Alignment.BottomEnd)
                        .size(10.dp)
                        .clip(CircleShape)
                        .background(scheme.error),
                )
            }
        }
        Text(
            text,
            style = MaterialTheme.typography.bodyLarge,
            color = scheme.onSurfaceVariant,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        Icon(painterResource(R.drawable.ic_chevron_right), contentDescription = null, tint = scheme.onSurfaceVariant, modifier = Modifier.size(22.dp))
    }
}

private data class FeatureTile(
    val icon: Int,
    val label: String,
    val enabled: Boolean,
    val active: Boolean = false,
    val onClick: () -> Unit,
)

/** The card of six icon tiles in two rows of three. */
@Composable
private fun FeatureGrid(tiles: List<FeatureTile>) {
    val scheme = MaterialTheme.colorScheme
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .softCard(cornerRadius = 32.dp, baseColor = scheme.surfaceContainer)
            .padding(horizontal = 8.dp, vertical = 22.dp),
        verticalArrangement = Arrangement.spacedBy(26.dp),
    ) {
        for (row in tiles.chunked(3)) {
            Row(Modifier.fillMaxWidth()) {
                for (tile in row) {
                    FeatureTileView(tile, Modifier.weight(1f))
                }
            }
        }
    }
}

@Composable
private fun FeatureTileView(tile: FeatureTile, modifier: Modifier) {
    val scheme = MaterialTheme.colorScheme
    Column(
        modifier = modifier
            .clip(RoundedCornerShape(20.dp))
            .clickable(enabled = tile.enabled, role = Role.Button, onClick = tile.onClick)
            .semantics { contentDescription = tile.label }
            .alpha(if (tile.enabled) 1f else DISABLED_ALPHA)
            .padding(vertical = 4.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Box(
            modifier = Modifier
                .size(64.dp)
                .clip(RoundedCornerShape(22.dp))
                .background(
                    if (tile.active) {
                        Brush.linearGradient(listOf(scheme.primary, androidx.compose.ui.graphics.lerp(scheme.primary, Color.White, 0.22f)))
                    } else {
                        Brush.linearGradient(listOf(scheme.primaryContainer, scheme.surfaceContainerHighest))
                    },
                ),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                painterResource(tile.icon),
                contentDescription = null,
                tint = if (tile.active) scheme.onPrimary else scheme.primary,
                modifier = Modifier.size(32.dp),
            )
        }
        Text(
            tile.label,
            style = MaterialTheme.typography.bodyLarge,
            color = scheme.onSurface,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}
