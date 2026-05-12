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
    val keyFingerprint: String? = null
)

class ConnectionViewModel(application: Application) : AndroidViewModel(application) {

    private val app = application as DeepSeekApp
    private val tunnelManager = app.appContainer.sshTunnelManager
    private val keyStore = app.appContainer.keyStoreManager
    private val configRepo = app.appContainer.configRepository

    private val _uiState = MutableStateFlow(ConnectionUiState())
    val uiState: StateFlow<ConnectionUiState> = _uiState.asStateFlow()

    init {
        // Observe tunnel state
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

        // Load config and key status
        refreshKeyStatus()
    }

    fun connect() {
        viewModelScope.launch {
            val config = configRepo.loadConfig()
            tunnelManager.connect()
        }
    }

    fun disconnect() {
        tunnelManager.disconnect()
    }

    fun refreshKeyStatus() {
        val hasKey = keyStore.hasKey() || keyStore.getSshKeyBytes() != null
        _uiState.update {
            it.copy(
                hasSshKey = hasKey,
                keyFingerprint = keyStore.getPublicKeyFingerprint(),
                host = configRepo.loadConfig().host
            )
        }
    }
}
