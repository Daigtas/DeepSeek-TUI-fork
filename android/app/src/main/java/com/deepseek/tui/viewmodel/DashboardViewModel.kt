package com.deepseek.tui.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.deepseek.tui.DeepSeekApp
import com.deepseek.tui.data.db.AgentEntity
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class DashboardUiState(
    val agents: List<AgentEntity> = emptyList(),
    val activeTaskCount: Long = 0,
    val connectedClientCount: Int = 0,
    val daemonUptime: String = "",
    val progressLines: List<String> = emptyList(),
    val isRefreshing: Boolean = false
)

class DashboardViewModel(application: Application) : AndroidViewModel(application) {

    private val app = application as DeepSeekApp
    private val apiClient = app.appContainer.apiClient
    private val agentDao = app.appContainer.database.agentDao()
    private val gson = Gson()

    private val _uiState = MutableStateFlow(DashboardUiState())
    val uiState: StateFlow<DashboardUiState> = _uiState.asStateFlow()

    private var pollingJob: Job? = null

    init {
        // Observe agent DB changes
        viewModelScope.launch {
            agentDao.getAllAgents().collect { agents ->
                _uiState.update { it.copy(agents = agents) }
            }
        }
    }

    fun startPolling() {
        pollingJob?.cancel()
        pollingJob = viewModelScope.launch {
            while (true) {
                refreshDashboard()
                delay(3_000) // 3-second polling; cancellable
            }
        }
    }

    fun stopPolling() {
        pollingJob?.cancel()
        pollingJob = null
    }

    suspend fun refreshDashboard() {
        _uiState.update { it.copy(isRefreshing = true) }
        try {
            // Fetch daemon status
            val statusResult = apiClient.daemonStatus()
            if (statusResult.isSuccess) {
                val json = gson.fromJson<Map<String, Any>>(
                    statusResult.getOrThrow(),
                    object : TypeToken<Map<String, Any>>() {}.type
                )
                _uiState.update { state ->
                    state.copy(
                        activeTaskCount = (json["active_tasks"] as? Double)?.toLong() ?: 0,
                        connectedClientCount = (json["connected_clients"] as? Double)?.toInt() ?: 0,
                        daemonUptime = json["started_at"] as? String ?: ""
                    )
                }
            }

            // Fetch progress log
            val progressResult = apiClient.daemonProgress()
            if (progressResult.isSuccess) {
                val body = progressResult.getOrThrow()
                val lines = body.lines().takeLast(10)
                _uiState.update { it.copy(progressLines = lines) }
            }

            // Fetch agents
            val agentsResult = apiClient.swarmAgents()
            if (agentsResult.isSuccess) {
                val body = agentsResult.getOrThrow()
                try {
                    val agentList: List<Map<String, Any>> = gson.fromJson(
                        body,
                        object : TypeToken<List<Map<String, Any>>>() {}.type
                    )
                    val entities = agentList.map { agent ->
                        AgentEntity(
                            agentId = agent["id"] as? String ?: "",
                            role = agent["role"] as? String ?: "worker",
                            status = agent["status"] as? String ?: "idle",
                            taskDescription = agent["task"] as? String,
                            lastUpdated = System.currentTimeMillis()
                        )
                    }
                    agentDao.deleteAll()
                    agentDao.insertAgents(entities)
                } catch (_: Exception) {
                    // Agents endpoint might return object, not array — handle gracefully
                }
            }
        } catch (_: Exception) {
            // Silent fail — UI shows stale/empty state
        } finally {
            _uiState.update { it.copy(isRefreshing = false) }
        }
    }

    override fun onCleared() {
        stopPolling()
        super.onCleared()
    }
}
