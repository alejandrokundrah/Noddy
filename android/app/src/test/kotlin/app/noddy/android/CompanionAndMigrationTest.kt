// SPDX-License-Identifier: GPL-3.0-or-later
package app.noddy.android

import app.noddy.android.companion.PcCompanion
import app.noddy.android.ui.Preferences
import app.noddy.android.ui.theme.Appearance
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class CompanionAndMigrationTest {

    @Test
    fun `z-base-32 decodes like the core encodes device IDs`() {
        // The z-base-32 specification's own example.
        assertArrayEquals(
            byteArrayOf(0xF0.toByte(), 0xBF.toByte(), 0xC7.toByte()),
            PcCompanion.decodeZBase32("6n9hq"),
        )
        // The device ID whose 32 bytes are 0, 1, 2 ... 31, as the core writes it
        // (pinned on the Rust side in core/noddy-protocol/src/id.rs).
        assertArrayEquals(
            ByteArray(32) { it.toByte() },
            PcCompanion.decodeZBase32("yyyoryarywdyqnyjbefoadeqbhebnrounoktcfaadrpbs8y7daxo"),
        )
        assertEquals(null, PcCompanion.decodeZBase32("not-an-id"))
    }

    @Test
    fun `the scan filter matches the beacon a PC broadcasts for its device ID`() {
        // What desktop/app/src/win/ble_beacon.rs broadcasts: the magic, then the
        // ID's first 8 bytes, here A5 3C 00 FF 12 34 56 78, which no hex reading of
        // the ID could produce.
        val id = "ww6yb9a1gtm8tgw4ukpjigw4ukpjigw4ukpjigw4ukpjigw4ukpy"
        val broadcast = PcCompanion.BEACON_MAGIC + byteArrayOf(
            0xA5.toByte(), 0x3C, 0x00, 0xFF.toByte(), 0x12, 0x34, 0x56, 0x78,
        )
        assertArrayEquals(broadcast, PcCompanion.buildBeaconPayload(id))
        assertArrayEquals(broadcast, PcCompanion.buildBeaconPayload(id.uppercase()))

        val maskWithId = PcCompanion.buildBeaconMask(hasDeviceId = true)
        assertEquals(13, maskWithId.size)
        assertTrue(maskWithId.all { it == 0xFF.toByte() })

        // Not a device ID: match any Noddy PC.
        assertArrayEquals(PcCompanion.BEACON_MAGIC, PcCompanion.buildBeaconPayload("abcd"))
        assertArrayEquals(PcCompanion.BEACON_MAGIC, PcCompanion.buildBeaconPayload("0123456789abcdef-fedcba9876543210"))
        assertArrayEquals(PcCompanion.BEACON_MAGIC, PcCompanion.buildBeaconPayload(null))
        val maskWithoutId = PcCompanion.buildBeaconMask(hasDeviceId = false)
        assertEquals(5, maskWithoutId.size)
        assertTrue(maskWithoutId.all { it == 0xFF.toByte() })
    }

    @Test
    fun `Preferences sanitizeAppearance ignores legacy theme values and keeps only a valid mode`() {
        // Old installs saved a theme (hive, bloom, graphite), a seed color, dynamic color, AMOLED
        // and comb motion; only glass and ember exist now, and an unknown mode falls back to
        // following the system.
        assertEquals("glass", Preferences.sanitizeAppearance(mode = "dark", theme = "hive").theme)
        assertEquals("glass", Preferences.sanitizeAppearance(mode = "dark", theme = "graphite").theme)
        assertEquals("system", Preferences.sanitizeAppearance(mode = "neon").mode)
        assertEquals("system", Preferences.sanitizeAppearance(mode = null).mode)
        assertEquals("system", Preferences.sanitizeAppearance(Appearance(mode = "")).mode)

        assertEquals("light", Preferences.sanitizeAppearance(mode = "light").mode)
        assertEquals("dark", Preferences.sanitizeAppearance(Appearance(mode = "dark")).mode)
    }
}
