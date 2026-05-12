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

data class HiveEntry(
    val key: String,
    val value: String,
    val author: String = "",
    val version: Long = 0,
    val timestamp: String = ""
)

data class HiveUiState(
    val entries: List<HiveEntry> = emptyList(),
    val isRefreshing: Boolean = false,
    val queryResult: String? = null,
    val queryKey: String = "",
    val injectMessage: String? = null,
    val filterPrefix: String = ""
)

class HiveViewModel(application: Application) : AndroidViewModel(application) {

    private val app = application as DeepSeekApp
    private val apiClient = app.appContainer.apiClient
    private val gson = Gson()

    private val _uiState = MutableStateFlow(HiveUiState())
    val uiState: StateFlow<HiveUiState> = _uiState.asStateFlow()

    fun refreshSnapshot() {
        viewModelScope.launch {
            _uiState.update { it.copy(isRefreshing = true) }
            try {
                val result = apiClient.hiveSnapshot()
                if (result.isSuccess) {
                    val body = result.getOrThrow()
                    val rawEntries: List<Map<String, Any>> = gson.fromJson(
                        body,
                        object : TypeToken<List<Map<String, Any>>>() {}.type
                    )
                    val entries = rawEntries.map { entry ->
                        HiveEntry(
                            key = entry["key"] as? String ?: "",
                            value = gson.toJson(entry["value"]),
                            author = entry["author"] as? String ?: "",
                            version = (entry["version"] as? Double)?.toLong() ?: 0,
                            timestamp = entry["timestamp"] as? String ?: ""
                        )
                    }
                    _uiState.update { it.copy(entries = entries) }
                }
            } catch (_: Exception) {
                // keep stale data
            } finally {
                _uiState.update { it.copy(isRefreshing = false) }
            }
        }
    }

    fun queryByKey(key: String) {
        viewModelScope.launch {
            _uiState.update { it.copy(queryKey = key, queryResult = null) }
            try {
                val result = apiClient.hiveQuery(key)
                _uiState.update { state ->
                    state.copy(
                        queryResult = if (result.isSuccess) result.getOrThrow() else "Error: ${result.exceptionOrNull()?.message}"
                    )
                }
            } catch (e: Exception) {
                _uiState.update { it.copy(queryResult = "Error: ${e.message}") }
            }
        }
    }

    fun injectEntry(key: String, value: String) {
        viewModelScope.launch {
            try {
                val result = apiClient.hiveInject(key, value)
                _uiState.update { state ->
                    state.copy(
                        injectMessage = if (result.isSuccess) "Injected" else "Failed: ${result.exceptionOrNull()?.message}"
                    )
                }
                refreshSnapshot()
            } catch (e: Exception) {
                _uiState.update { it.copy(injectMessage = "Error: ${e.message}") }
            }
        }
    }

    fun setFilterPrefix(prefix: String) {
        _uiState.update { it.copy(filterPrefix = prefix) }
    }

    fun clearMessages() {
        _uiState.update { it.copy(injectMessage = null, queryResult = null, queryKey = "") }
    }
}
