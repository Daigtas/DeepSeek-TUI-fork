package com.deepseek.tui.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.deepseek.tui.DeepSeekApp
import com.deepseek.tui.connection.SshTunnelManager
import com.deepseek.tui.data.prefs.ConnectionConfig
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

data class ConnectionUiState(
    val state: SshTunnelManager.TunnelState = SshTunnelManager.TunnelState.DISCONNECTED,
    val errorMessage: String? = null,
    val latencyMs: Long? = null,
    val reconnectCount: Int = 0,
    val host: String = "boottify.com",
    val hasSshKey: Boolean = false,
    val keyFingerprint: String? = null,
    val logMessages: List<String> = emptyList(),
    val pendingHostKey: SshTunnelManager.PendingHostKey? = null,
    val config: ConnectionConfig = ConnectionConfig()
)

class ConnectionViewModel(application: Application) : AndroidViewModel(application) {

    private val app = application as DeepSeekApp
    private val tunnelManager = app.appContainer.sshTunnelManager
    private val keyStore = app.appContainer.keyStoreManager
    private val configRepo = app.appContainer.configRepository
    private val apiClient = app.appContainer.apiClient

    private val _uiState = MutableStateFlow(ConnectionUiState())
    val uiState: StateFlow<ConnectionUiState> = _uiState.asStateFlow()

    init {
        viewModelScope.launch { tunnelManager.state.collect { _uiState.update { s -> s.copy(state = it) } } }
        viewModelScope.launch { tunnelManager.errorMessage.collect { _uiState.update { s -> s.copy(errorMessage = it) } } }
        viewModelScope.launch { tunnelManager.latencyMs.collect { _uiState.update { s -> s.copy(latencyMs = it) } } }
        viewModelScope.launch { tunnelManager.reconnectCount.collect { _uiState.update { s -> s.copy(reconnectCount = it) } } }
        viewModelScope.launch { tunnelManager.logMessages.collect { _uiState.update { s -> s.copy(logMessages = it) } } }
        viewModelScope.launch { tunnelManager.pendingHostKey.collect { _uiState.update { s -> s.copy(pendingHostKey = it) } } }
        refreshKeyStatus()
        refreshConfig()
    }

    fun connect() { viewModelScope.launch { tunnelManager.connect() } }
    fun disconnect() { tunnelManager.disconnect() }
    fun acceptHostKey() { tunnelManager.acceptHostKey() }
    fun rejectHostKey() { tunnelManager.rejectHostKey() }

    fun detachDaemon() { viewModelScope.launch { apiClient.daemonDetach() } }
    fun attachDaemon() { viewModelScope.launch { apiClient.daemonAttach() } }
    fun saveCheckpoint() { viewModelScope.launch { apiClient.daemonCheckpoint() } }

    fun saveConfig(config: ConnectionConfig) { configRepo.saveConfig(config); refreshConfig() }

    fun refreshKeyStatus() {
        val hasKey = keyStore.hasKey() || keyStore.getSshKeyBytes() != null
        _uiState.update { it.copy(hasSshKey = hasKey, keyFingerprint = keyStore.getPublicKeyFingerprint()) }
    }

    private fun refreshConfig() {
        val config = configRepo.loadConfig()
        _uiState.update { it.copy(host = config.host, config = config) }
    }
}
