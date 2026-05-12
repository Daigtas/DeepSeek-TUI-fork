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

data class SessionSummary(
    val id: String,
    val name: String,
    val workspace: String = "",
    val turnCount: Int = 0,
    val model: String = "",
    val createdAt: String = "",
    val updatedAt: String = ""
)

data class SessionsUiState(
    val sessions: List<SessionSummary> = emptyList(),
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val exportResult: String? = null
)

class SessionsViewModel(application: Application) : AndroidViewModel(application) {
    private val app = application as DeepSeekApp
    private val api = app.appContainer.apiClient
    private val gson = Gson()

    private val _uiState = MutableStateFlow(SessionsUiState())
    val uiState: StateFlow<SessionsUiState> = _uiState.asStateFlow()

    // Don't load sessions at init — wait for explicit refresh after connect
    init {}

    fun loadSessions() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            val result = api.sessionList()
            if (result.isSuccess) {
                try {
                    val list: List<Map<String, Any?>> = gson.fromJson(
                        result.getOrThrow(),
                        object : TypeToken<List<Map<String, Any?>>>() {}.type
                    )
                    val sessions = list.map { item ->
                        SessionSummary(
                            id = item["id"] as? String ?: "",
                            name = item["name"] as? String ?: "Untitled",
                            workspace = item["workspace_path"] as? String ?: "",
                            turnCount = (item["turn_count"] as? Double)?.toInt() ?: 0,
                            model = item["model"] as? String ?: "",
                            createdAt = item["created_at"] as? String ?: "",
                            updatedAt = item["updated_at"] as? String ?: ""
                        )
                    }
                    _uiState.update { it.copy(sessions = sessions, isLoading = false) }
                } catch (e: Exception) {
                    _uiState.update { it.copy(errorMessage = "Parse error: ${e.message}", isLoading = false) }
                }
            } else {
                _uiState.update { it.copy(errorMessage = result.exceptionOrNull()?.message, isLoading = false) }
            }
        }
    }

    fun deleteSession(id: String) {
        viewModelScope.launch {
            val result = api.sessionDelete(id)
            if (result.isSuccess) {
                loadSessions()
            } else {
                _uiState.update { it.copy(errorMessage = "Delete failed: ${result.exceptionOrNull()?.message}") }
            }
        }
    }

    fun exportSession(id: String) {
        viewModelScope.launch {
            val result = api.sessionExport(id)
            if (result.isSuccess) {
                _uiState.update { it.copy(exportResult = result.getOrThrow()) }
            } else {
                _uiState.update { it.copy(errorMessage = "Export failed: ${result.exceptionOrNull()?.message}") }
            }
        }
    }
}
