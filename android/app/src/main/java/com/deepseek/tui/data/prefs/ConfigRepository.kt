package com.deepseek.tui.data.prefs

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKeys
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken

/**
 * Stores and retrieves connection configuration using
 * EncryptedSharedPreferences (AES-256-GCM, Keystore-backed).
 */
class ConfigRepository(context: Context) {

    private val prefs: SharedPreferences by lazy {
        val masterKey = MasterKeys.getOrCreate(MasterKeys.AES256_GCM_SPEC)
        EncryptedSharedPreferences.create(
            "deepseek_config",
            masterKey,
            context,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    }

    private val gson = Gson()

    fun loadConfig(): ConnectionConfig {
        val json = prefs.getString("connection_config", null) ?: return ConnectionConfig()
        return try {
            gson.fromJson(json, ConnectionConfig::class.java)
        } catch (e: Exception) {
            ConnectionConfig()
        }
    }

    fun saveConfig(config: ConnectionConfig) {
        prefs.edit()
            .putString("connection_config", gson.toJson(config))
            .apply()
    }

    fun getHostKeyFingerprint(host: String): String? {
        return prefs.getString("known_host_$host", null)
    }

    fun saveHostKeyFingerprint(host: String, fingerprint: String) {
        prefs.edit()
            .putString("known_host_$host", fingerprint)
            .apply()
    }

    fun clearHostKey(host: String) {
        prefs.edit()
            .remove("known_host_$host")
            .apply()
    }

    /**
     * Clear all stored configuration data.
     */
    fun clear() {
        prefs.edit().clear().apply()
    }
}
