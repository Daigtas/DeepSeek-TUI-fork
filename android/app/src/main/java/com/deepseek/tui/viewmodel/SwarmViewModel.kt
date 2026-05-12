package com.deepseek.tui.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.deepseek.tui.DeepSeekApp
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class SwarmAgent(
    val agentId: String,
    val role: String,
    val name: String = "",
    val status: String = "idle",
    val taskDescription: String? = null,
    val uptime: String = ""
)

data class SwarmUiState(
    val agents: List<SwarmAgent> = emptyList(),
    val isRefreshing: Boolean = false,
    val spawnMessage: String? = null
)

class SwarmViewModel(application: Application) : AndroidViewModel(application) {

    private val app = application as DeepSeekApp
    private val apiClient = app.appContainer.apiClient
    private val gson = Gson()

    private val _uiState = MutableStateFlow(SwarmUiState())
    val uiState: StateFlow<SwarmUiState> = _uiState.asStateFlow()

    fun refreshAgents() {
        viewModelScope.launch {
            _uiState.update { it.copy(isRefreshing = true) }
            try {
                val result = apiClient.swarmAgents()
                if (result.isSuccess) {
                    val body = result.getOrThrow()
                    val agentList: List<Map<String, Any>> = gson.fromJson(
                        body,
                        object : TypeToken<List<Map<String, Any>>>() {}.type
                    )
                    val agents = agentList.map { agent ->
                        SwarmAgent(
                            agentId = agent["id"] as? String ?: "",
                            role = agent["role"] as? String ?: "worker",
                            name = agent["name"] as? String ?: "",
                            status = agent["status"] as? String ?: "idle",
                            taskDescription = agent["task"] as? String,
                            uptime = agent["uptime"] as? String ?: ""
                        )
                    }
                    _uiState.update { it.copy(agents = agents) }
                }
            } catch (_: Exception) {
                // keep stale data
            } finally {
                _uiState.update { it.copy(isRefreshing = false) }
            }
        }
    }

    fun spawnAgent(role: String, name: String, prompt: String) {
        viewModelScope.launch {
            try {
                val result = apiClient.swarmSpawn(role, name, prompt)
                if (result.isSuccess) {
                    _uiState.update { it.copy(spawnMessage = "Agent spawned") }
                    refreshAgents()
                } else {
                    _uiState.update { it.copy(spawnMessage = "Failed: ${result.exceptionOrNull()?.message}") }
                }
            } catch (e: Exception) {
                _uiState.update { it.copy(spawnMessage = "Error: ${e.message}") }
            }
        }
    }

    fun clearMessage() {
        _uiState.update { it.copy(spawnMessage = null) }
    }
}
