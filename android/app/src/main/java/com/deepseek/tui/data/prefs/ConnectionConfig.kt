package com.deepseek.tui.data.prefs

/**
 * Connection configuration for the SSH tunnel.
 * Stored in EncryptedSharedPreferences.
 */
data class ConnectionConfig(
    val host: String = "boottify.com",
    val port: Int = 22,
    val user: String = "root",
    val postLoginCommands: List<String> = listOf(
        "su boottify",
        "cd /var/www"
    ),
    val remotePort: Int = 8787,
    val localPort: Int = 18787,
    val healthCheckIntervalSec: Int = 15,
    val reconnectMaxBackoffSec: Int = 30,
    val useDirectHttps: Boolean = false,  // bypass SSH tunnel, use deepseek.boottify.com directly
    val directHttpsUrl: String = "https://deepseek.boottify.com",
    val daemonAutoStart: Boolean = true
)
