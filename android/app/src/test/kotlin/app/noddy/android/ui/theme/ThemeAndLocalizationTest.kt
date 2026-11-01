// SPDX-License-Identifier: GPL-3.0-or-later
package app.noddy.android.ui.theme

import app.noddy.android.ui.Preferences
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import androidx.compose.ui.graphics.Color
import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Element

class ThemeAndLocalizationTest {

    @Test
    fun `sanitizeAppearance keeps valid modes and falls back to system on unknown or null values`() {
        for (mode in listOf("system", "light", "dark")) {
            assertEquals(mode, Preferences.sanitizeAppearance(Appearance(mode = mode)).mode)
        }

        val defaults = Appearance()
        assertEquals("system", defaults.mode)
        assertEquals(defaults, Preferences.sanitizeAppearance(Appearance(mode = "unknown_mode")))
        assertEquals(defaults, Preferences.sanitizeAppearance(Appearance(mode = "")))
        assertEquals(defaults, Preferences.sanitizeAppearance(mode = null))
    }

    @Test
    fun `sanitizeAppearance keeps glass and ember and falls back to glass on anything else`() {
        assertEquals("glass", Appearance().theme)
        for (theme in listOf("glass", "ember")) {
            assertEquals(theme, Preferences.sanitizeAppearance(Appearance(theme = theme)).theme)
            assertEquals(theme, Preferences.sanitizeAppearance(mode = "dark", theme = theme).theme)
        }
        // Themes that no longer exist, unknown values and nothing saved yet are Glass.
        for (old in listOf("hive", "bloom", "graphite", "sepia", "", "EMBER")) {
            assertEquals("glass", Preferences.sanitizeAppearance(mode = "dark", theme = old).theme)
        }
        assertEquals("glass", Preferences.sanitizeAppearance(mode = "dark", theme = null).theme)
        // The mode is kept next to Ember, which simply ignores it.
        assertEquals("light", Preferences.sanitizeAppearance(mode = "light", theme = "ember").mode)
        assertTrue(Appearance(theme = "ember").isEmber)
        assertFalse(Appearance().isEmber)
    }

    @Test
    fun `Glass has distinct light and dark palettes for its fixed orange seed`() {
        val seed = Tokens.glassSeeds[Tokens.DEFAULT_SEED]
        assertNotNull("the default seed '${Tokens.DEFAULT_SEED}' must exist", seed)
        seed!!
        assertNotEquals("light and dark surfaces must differ", seed.light.surface, seed.dark.surface)
        assertNotEquals("light and dark onSurface must differ", seed.light.onSurface, seed.dark.onSurface)
        assertNotEquals("light and dark primary must differ", seed.light.primary, seed.dark.primary)
        assertNotEquals(Tokens.errorLight, Tokens.errorDark)
        assertEquals(Color(0xFFB23808), seed.light.primary)
        assertEquals(Color(0xFFEB6030), seed.dark.primary)
    }

    private fun resolveResDir(): File {
        val candidates = listOf(
            File("src/main/res"),
            File("app/src/main/res"),
            File("android/app/src/main/res"),
        )
        return candidates.firstOrNull { it.isDirectory }
            ?: error("Could not locate src/main/res from working directory ${File(".").absolutePath}")
    }

    private fun parsePluralsXml(file: File): Map<String, Map<String, String>> {
        val doc = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(file)
        doc.documentElement.normalize()

        val plurals = linkedMapOf<String, Map<String, String>>()
        val pluralNodes = doc.getElementsByTagName("plurals")
        for (i in 0 until pluralNodes.length) {
            val el = pluralNodes.item(i) as? Element ?: continue
            if (el.getAttribute("translatable") == "false") continue
            val name = el.getAttribute("name")
            val items = linkedMapOf<String, String>()
            val itemNodes = el.getElementsByTagName("item")
            for (j in 0 until itemNodes.length) {
                val item = itemNodes.item(j) as? Element ?: continue
                items[item.getAttribute("quantity")] = item.textContent
            }
            if (name.isNotEmpty()) {
                plurals[name] = items
            }
        }
        return plurals
    }

    private val placeholderRegex = Regex("""%(?:\d+\$)?(?:\.\d+)?[sdf]""")

    private fun extractPlaceholders(text: String): Set<String> =
        placeholderRegex.findAll(text.replace("%%", "")).map { it.value }.toSet()

    @Test
    fun `Simplified Chinese localization covers all translatable plurals with matching placeholders`() {
        val resDir = resolveResDir()
        val enFile = File(resDir, "values/strings.xml")
        val zhFile = File(resDir, "values-zh-rCN/strings.xml")
        assertTrue("values/strings.xml must exist", enFile.isFile)
        assertTrue("values-zh-rCN/strings.xml must exist", zhFile.isFile)

        val enPlurals = parsePluralsXml(enFile)
        val zhPlurals = parsePluralsXml(zhFile)

        assertTrue("Expected plurals in values/strings.xml", enPlurals.isNotEmpty())

        val missingPlurals = enPlurals.keys - zhPlurals.keys
        assertTrue("Missing zh-rCN plural translations: $missingPlurals", missingPlurals.isEmpty())

        for ((key, enItems) in enPlurals) {
            val zhItems = zhPlurals.getValue(key)
            assertTrue("Empty zh-rCN plural items for '$key'", zhItems.isNotEmpty())
            val expectedPlaceholders = enItems.values.flatMap { extractPlaceholders(it) }.toSet()
            for ((qty, zhText) in zhItems) {
                assertFalse("Blank zh-rCN plural '$key' quantity '$qty'", zhText.isBlank())
                assertEquals(
                    "Placeholder mismatch in plural '$key' quantity '$qty'",
                    expectedPlaceholders,
                    extractPlaceholders(zhText),
                )
            }
        }
    }
}
