// SPDX-License-Identifier: GPL-3.0-or-later
package app.noddy.android.ui

import android.content.Context
import androidx.core.content.edit
import app.noddy.android.ui.theme.Appearance
import app.noddy.android.ui.walkthrough.WalkthroughPersistence
import app.noddy.android.ui.walkthrough.WalkthroughStep
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** The app's look, saved in shared preferences and observable. */
@androidx.compose.runtime.Stable
class Preferences(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("appearance", Context.MODE_PRIVATE)
    private val _appearance = MutableStateFlow(load())
    val appearance: StateFlow<Appearance> = _appearance.asStateFlow()

    private val _touchpadSensitivity = MutableStateFlow(prefs.getFloat(TOUCHPAD_SENSITIVITY, 1.0f).coerceIn(0.4f, 3.0f))
    val touchpadSensitivity: StateFlow<Float> = _touchpadSensitivity.asStateFlow()

    private val _suggestClipboardActions = MutableStateFlow(prefs.getBoolean(SUGGEST_CLIPBOARD_ACTIONS, true))
    val suggestClipboardActions: StateFlow<Boolean> = _suggestClipboardActions.asStateFlow()

    private val _volumeKeysControlPc = MutableStateFlow(prefs.getBoolean(VOLUME_KEYS_CONTROL_PC, true))
    val volumeKeysControlPc: StateFlow<Boolean> = _volumeKeysControlPc.asStateFlow()

    private val _backgroundServiceEnabled = MutableStateFlow(prefs.getBoolean(BACKGROUND_SERVICE_ENABLED, true))
    val backgroundServiceEnabled: StateFlow<Boolean> = _backgroundServiceEnabled.asStateFlow()

    private val preferenceChangeListener =
        android.content.SharedPreferences.OnSharedPreferenceChangeListener { _, key ->
            if (key == BACKGROUND_SERVICE_ENABLED) {
                _backgroundServiceEnabled.value = prefs.getBoolean(BACKGROUND_SERVICE_ENABLED, true)
            }
        }

    init {
        prefs.registerOnSharedPreferenceChangeListener(preferenceChangeListener)
    }

    private val _walkthrough = MutableStateFlow(loadWalkthrough())
    val walkthrough: StateFlow<WalkthroughPersistence> = _walkthrough.asStateFlow()

    private fun load(): Appearance {
        val defaults = Appearance()
        return sanitizeAppearance(
            mode = prefs.getString(MODE, defaults.mode),
            theme = prefs.getString(THEME, defaults.theme),
        )
    }

    private fun loadWalkthrough(): WalkthroughPersistence {
        val completed = prefs.getBoolean(WALKTHROUGH_COMPLETED, false)
        val rawStep = prefs.getInt(WALKTHROUGH_STEP, 0)
        val clampedStep = rawStep.coerceIn(0, WalkthroughStep.entries.lastIndex)
        val dismissed = prefs.getBoolean(WHATS_NEW_GUIDE_DISMISSED, false)
        return WalkthroughPersistence(
            completed = completed,
            currentStepIndex = clampedStep,
            whatsNewGuideDismissed = dismissed,
        )
    }

    fun update(change: (Appearance) -> Appearance) {
        val next = sanitizeAppearance(change(_appearance.value))
        prefs.edit {
            putString(THEME, next.theme)
            putString(MODE, next.mode)
        }
        _appearance.value = next
    }

    fun updateTouchpadSensitivity(sensitivity: Float) {
        val clamped = sensitivity.coerceIn(0.4f, 3.0f)
        prefs.edit { putFloat(TOUCHPAD_SENSITIVITY, clamped) }
        _touchpadSensitivity.value = clamped
    }

    fun updateSuggestClipboardActions(enabled: Boolean) {
        prefs.edit { putBoolean(SUGGEST_CLIPBOARD_ACTIONS, enabled) }
        _suggestClipboardActions.value = enabled
        if (!enabled) {
            app.noddy.android.links.LinkNotifications.dismissClipSuggestion(appContext)
        }
    }

    fun updateVolumeKeysControlPc(enabled: Boolean) {
        prefs.edit { putBoolean(VOLUME_KEYS_CONTROL_PC, enabled) }
        _volumeKeysControlPc.value = enabled
        (appContext as app.noddy.android.NoddyApplication).core.pcMedia.volumeKeys(enabled)
    }

    fun updateBackgroundServiceEnabled(enabled: Boolean) {
        prefs.edit { putBoolean(BACKGROUND_SERVICE_ENABLED, enabled) }
        _backgroundServiceEnabled.value = enabled
    }

    fun updateWalkthrough(change: (WalkthroughPersistence) -> WalkthroughPersistence) {
        val current = _walkthrough.value
        val next = change(current).let {
            it.copy(currentStepIndex = it.currentStepIndex.coerceIn(0, WalkthroughStep.entries.lastIndex))
        }
        if (next == current) return
        prefs.edit {
            putBoolean(WALKTHROUGH_COMPLETED, next.completed)
            putInt(WALKTHROUGH_STEP, next.currentStepIndex)
            putBoolean(WHATS_NEW_GUIDE_DISMISSED, next.whatsNewGuideDismissed)
        }
        _walkthrough.value = next
    }

    private val appContext = context.applicationContext

    companion object {
        private const val THEME = "theme"
        private const val MODE = "mode"
        private const val TOUCHPAD_SENSITIVITY = "touchpad_sensitivity"
        private const val SUGGEST_CLIPBOARD_ACTIONS = "suggest_clipboard_actions"
        private const val VOLUME_KEYS_CONTROL_PC = "volume_keys_control_pc"
        internal const val BACKGROUND_SERVICE_ENABLED = "background_service_enabled"
        private const val AWAY_MODE = "away_mode"
        private const val WALKTHROUGH_COMPLETED = "walkthrough_completed"
        private const val WALKTHROUGH_STEP = "walkthrough_step"
        private const val WHATS_NEW_GUIDE_DISMISSED = "whats_new_guide_dismissed"

        val VALID_THEMES = setOf("glass", "ember")
        val VALID_MODES = setOf("system", "light", "dark")

        /**
         * Keeps a valid theme ("glass" or "ember") and mode. Anything else, including the
         * themes that no longer exist and old color choices, falls back to the defaults.
         */
        fun sanitizeAppearance(mode: String?, theme: String? = null): Appearance {
            val defaults = Appearance()
            return Appearance(
                theme = theme?.takeIf { it in VALID_THEMES } ?: defaults.theme,
                mode = mode?.takeIf { it in VALID_MODES } ?: defaults.mode,
            )
        }

        fun sanitizeAppearance(appearance: Appearance): Appearance =
            sanitizeAppearance(mode = appearance.mode, theme = appearance.theme)

        fun isSuggestClipboardActionsEnabled(context: Context): Boolean =
            context.applicationContext
                .getSharedPreferences("appearance", Context.MODE_PRIVATE)
                .getBoolean(SUGGEST_CLIPBOARD_ACTIONS, true)

        fun isVolumeKeysControlPcEnabled(context: Context): Boolean =
            context.applicationContext
                .getSharedPreferences("appearance", Context.MODE_PRIVATE)
                .getBoolean(VOLUME_KEYS_CONTROL_PC, true)

        fun isAwayModeEnabled(context: Context): Boolean =
            context.applicationContext
                .getSharedPreferences("appearance", Context.MODE_PRIVATE)
                .getBoolean(AWAY_MODE, false)

        fun setAwayModeEnabled(context: Context, enabled: Boolean) {
            context.applicationContext
                .getSharedPreferences("appearance", Context.MODE_PRIVATE)
                .edit { putBoolean(AWAY_MODE, enabled) }
        }

        fun isBackgroundServiceEnabled(context: Context): Boolean =
            context.applicationContext
                .getSharedPreferences("appearance", Context.MODE_PRIVATE)
                .getBoolean(BACKGROUND_SERVICE_ENABLED, true)

        fun setBackgroundServiceEnabled(context: Context, enabled: Boolean) {
            context.applicationContext
                .getSharedPreferences("appearance", Context.MODE_PRIVATE)
                .edit { putBoolean(BACKGROUND_SERVICE_ENABLED, enabled) }
        }
    }
}

