package com.deepseek.tui

import android.app.Application
import org.bouncycastle.jce.provider.BouncyCastleProvider
import java.security.Security

class DeepSeekApp : Application() {

    lateinit var appContainer: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        instance = this

        // Android ships a stripped BouncyCastle without X25519/EdDSA.
        // Replace with full provider before any SSH connections.
        try {
            Security.removeProvider(BouncyCastleProvider.PROVIDER_NAME)
            Security.insertProviderAt(BouncyCastleProvider(), 1)
        } catch (e: Exception) {
            android.util.Log.e("DeepSeekApp", "Failed to install BouncyCastle provider", e)
        }

        appContainer = AppContainer(this)
    }

    companion object {
        lateinit var instance: DeepSeekApp
            private set
    }
}

/**
 * Manual DI container — no Hilt/Dagger dependency.
 * All singletons created here and scoped to application lifetime.
 */
class AppContainer(private val app: DeepSeekApp) {

    val configRepository: com.deepseek.tui.data.prefs.ConfigRepository by lazy {
        com.deepseek.tui.data.prefs.ConfigRepository(app)
    }

    val keyStoreManager: com.deepseek.tui.connection.KeyStoreManager by lazy {
        com.deepseek.tui.connection.KeyStoreManager(app)
    }

    val sshTunnelManager: com.deepseek.tui.connection.SshTunnelManager by lazy {
        com.deepseek.tui.connection.SshTunnelManager(app)
    }

    val apiClient: com.deepseek.tui.connection.ApiClient by lazy {
        com.deepseek.tui.connection.ApiClient()
    }

    val database: com.deepseek.tui.data.db.AppDatabase by lazy {
        com.deepseek.tui.data.db.AppDatabase.build(app)
    }

    val messageRepository: com.deepseek.tui.data.repository.MessageRepository by lazy {
        com.deepseek.tui.data.repository.MessageRepository(database.messageDao())
    }

    /**
     * Clear all local data: Room DB tables, encrypted prefs, and key store.
     * Must be called from a coroutine (Room clearAllTables is a suspend function).
     */
    suspend fun clearAllData() {
        database.clearAllTables()
        configRepository.clear()
        keyStoreManager.clearKeys()
    }
}
