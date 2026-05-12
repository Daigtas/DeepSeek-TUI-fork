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

    private val _uiState = MutableStateFlow(ConnectionUiState())
    val uiState: StateFlow<ConnectionUiState> = _uiState.asStateFlow()

    init {
        viewModelScope.launch {
            tunnelManager.state.collect { state ->
                _uiState.update { it.copy(state = state) }
            }
        }
        viewModelScope.launch {
            tunnelManager.errorMessage.collect { error ->
                _uiState.update { it.copy(errorMessage = error) }
            }
        }
        viewModelScope.launch {
            tunnelManager.latencyMs.collect { latency ->
                _uiState.update { it.copy(latencyMs = latency) }
            }
        }
        viewModelScope.launch {
            tunnelManager.reconnectCount.collect { count ->
                _uiState.update { it.copy(reconnectCount = count) }
            }
        }
        viewModelScope.launch {
            tunnelManager.logMessages.collect { msgs ->
                _uiState.update { it.copy(logMessages = msgs) }
            }
        }
        viewModelScope.launch {
            tunnelManager.pendingHostKey.collect { key ->
                _uiState.update { it.copy(pendingHostKey = key) }
            }
        }

        refreshKeyStatus()
        refreshConfig()
    }

    fun connect() {
        viewModelScope.launch { tunnelManager.connect() }
    }

    fun disconnect() {
        tunnelManager.disconnect()
    }

    fun acceptHostKey() {
        tunnelManager.acceptHostKey()
    }

    fun rejectHostKey() {
        tunnelManager.rejectHostKey()
    }

    fun saveConfig(config: ConnectionConfig) {
        configRepo.saveConfig(config)
        refreshConfig()
    }

    fun refreshKeyStatus() {
        val hasKey = keyStore.hasKey() || keyStore.getSshKeyBytes() != null
        _uiState.update {
            it.copy(
                hasSshKey = hasKey,
                keyFingerprint = keyStore.getPublicKeyFingerprint()
            )
        }
    }

    private fun refreshConfig() {
        val config = configRepo.loadConfig()
        _uiState.update { it.copy(host = config.host, config = config) }
    }
}
