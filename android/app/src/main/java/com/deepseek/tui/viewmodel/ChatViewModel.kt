package com.deepseek.tui.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.deepseek.tui.DeepSeekApp
import com.deepseek.tui.data.db.MessageEntity
import com.deepseek.tui.data.repository.MessageRepository
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import java.util.UUID

data class ChatUiState(
    val messages: List<MessageEntity> = emptyList(),
    val inputText: String = "",
    val isSending: Boolean = false,
    val isStreaming: Boolean = false,
    val streamingContent: String = "",
    val streamingThinking: String = "",
    val activeToolName: String? = null,
    val webSocketConnected: Boolean = false,
    val currentConversationId: String = UUID.randomUUID().toString(),
    val conversationIds: List<String> = emptyList(),
    val errorMessage: String? = null
)

class ChatViewModel(application: Application) : AndroidViewModel(application) {

    private val app = application as DeepSeekApp
    private val messageRepo = app.appContainer.messageRepository
    private val apiClient = app.appContainer.apiClient
    private val gson = Gson()

    private val _uiState = MutableStateFlow(ChatUiState())
    val uiState: StateFlow<ChatUiState> = _uiState.asStateFlow()

    private var webSocket: WebSocket? = null
    private var streamingMessageId: Long = 0

    init {
        viewModelScope.launch {
            uiState.collect { state ->
                messageRepo.getMessages(state.currentConversationId).collect { messages ->
                    _uiState.update { it.copy(messages = messages) }
                }
            }
        }
        viewModelScope.launch {
            messageRepo.getConversationIds().collect { ids ->
                _uiState.update { it.copy(conversationIds = ids) }
            }
        }
    }

    fun connectWebSocket() {
        if (webSocket != null) return

        webSocket = apiClient.newWebSocket("/ws", object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                _uiState.update { it.copy(webSocketConnected = true) }
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                handleWebSocketMessage(text)
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                _uiState.update { it.copy(webSocketConnected = false) }
                webSocket.close(1000, null)
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                _uiState.update { it.copy(webSocketConnected = false) }
                // Don't retry — server doesn't have /ws endpoint, use HTTP fallback
                // The 404 is expected; chat works fine via POST /prompt
            }
        })
    }

    fun disconnectWebSocket() {
        webSocket?.close(1000, "User disconnected")
        webSocket = null
        _uiState.update { it.copy(webSocketConnected = false) }
    }

    private fun handleWebSocketMessage(text: String) {
        try {
            val map: Map<String, Any?> = gson.fromJson(
                text,
                object : TypeToken<Map<String, Any?>>() {}.type
            )
            val method = map["method"] as? String ?: return
            val params = map["params"] as? Map<*, *>

            when (method) {
                "response.delta" -> {
                    val delta = params?.get("delta") as? String ?: return
                    _uiState.update { it.copy(
                        streamingContent = it.streamingContent + delta,
                        isStreaming = true
                    )}
                }
                "reasoning.delta" -> {
                    val delta = params?.get("delta") as? String ?: return
                    _uiState.update { it.copy(
                        streamingThinking = it.streamingThinking + delta,
                        isStreaming = true
                    )}
                }
                "tool.started" -> {
                    val toolName = params?.get("name") as? String
                    _uiState.update { it.copy(activeToolName = toolName) }
                }
                "tool.finished" -> {
                    _uiState.update { it.copy(activeToolName = null) }
                }
                "response.complete" -> {
                    val fullText = params?.get("full_text") as? String
                        ?: _uiState.value.streamingContent
                    val thinking = _uiState.value.streamingThinking
                    val conversationId = params?.get("conversation_id") as? String
                        ?: _uiState.value.currentConversationId

                    viewModelScope.launch {
                        if (streamingMessageId > 0) {
                            // Update the streaming placeholder
                            messageRepo.updateMessage(
                                MessageEntity(
                                    id = streamingMessageId,
                                    conversationId = conversationId,
                                    role = "assistant",
                                    content = fullText,
                                    timestamp = System.currentTimeMillis(),
                                    thinkingTokens = thinking.ifBlank { null },
                                    isStreaming = false
                                )
                            )
                        } else {
                            // Insert if no placeholder exists
                            messageRepo.insertMessage(
                                MessageEntity(
                                    conversationId = conversationId,
                                    role = "assistant",
                                    content = fullText,
                                    timestamp = System.currentTimeMillis(),
                                    thinkingTokens = thinking.ifBlank { null },
                                    isStreaming = false
                                )
                            )
                        }
                    }

                    _uiState.update { it.copy(
                        isStreaming = false,
                        streamingContent = "",
                        streamingThinking = "",
                        activeToolName = null,
                        isSending = false
                    )}
                    streamingMessageId = 0
                }
                "error" -> {
                    val errorMsg = params?.get("message") as? String ?: "Unknown error"
                    _uiState.update { it.copy(
                        errorMessage = errorMsg,
                        isSending = false,
                        isStreaming = false
                    )}
                }
            }
        } catch (_: Exception) {
            // Malformed JSON — ignore
        }
    }

    fun onInputChanged(text: String) {
        _uiState.update { it.copy(inputText = text, errorMessage = null) }
    }

    fun sendMessage() {
        val text = _uiState.value.inputText.trim()
        if (text.isEmpty()) return

        viewModelScope.launch {
            _uiState.update { it.copy(isSending = true, inputText = "", errorMessage = null) }

            try {
                val convId = _uiState.value.currentConversationId

                // Save user message
                val userMsg = MessageEntity(
                    conversationId = convId,
                    role = "user",
                    content = text,
                    timestamp = System.currentTimeMillis()
                )
                messageRepo.insertMessage(userMsg)

                // Insert streaming placeholder
                val placeholder = MessageEntity(
                    conversationId = convId,
                    role = "assistant",
                    content = "",
                    timestamp = System.currentTimeMillis(),
                    isStreaming = true
                )
                streamingMessageId = messageRepo.insertMessage(placeholder)

                // Try WebSocket first, fall back to HTTP POST
                if (_uiState.value.webSocketConnected) {
                    val wsPayload = """
                        {"jsonrpc":"2.0","method":"prompt","params":{"message":${escapeJson(text)},"conversation_id":"$convId"},"id":1}
                    """.trimIndent()
                    webSocket?.send(wsPayload)
                } else {
                    // HTTP POST fallback
                    val requestJson = """
                        {
                            "jsonrpc": "2.0",
                            "method": "prompt",
                            "params": {
                                "message": ${escapeJson(text)},
                                "conversation_id": "$convId"
                            },
                            "id": 1
                        }
                    """.trimIndent()

                    val result = apiClient.post("/", requestJson)

                    if (result.isSuccess) {
                        val responseBody = result.getOrThrow()
                        val reply = extractReply(responseBody)

                        // Replace placeholder with actual response
                        if (streamingMessageId > 0) {
                            messageRepo.updateMessage(
                                MessageEntity(
                                    id = streamingMessageId,
                                    conversationId = convId,
                                    role = "assistant",
                                    content = reply,
                                    timestamp = System.currentTimeMillis(),
                                    isStreaming = false
                                )
                            )
                        } else {
                            messageRepo.insertMessage(
                                MessageEntity(
                                    conversationId = convId,
                                    role = "assistant",
                                    content = reply,
                                    timestamp = System.currentTimeMillis()
                                )
                            )
                        }
                        streamingMessageId = 0
                    } else {
                        _uiState.update {
                            it.copy(errorMessage = result.exceptionOrNull()?.message ?: "Send failed")
                        }
                    }
                }
            } catch (e: Exception) {
                _uiState.update { it.copy(errorMessage = e.message) }
            } finally {
                if (!_uiState.value.isStreaming) {
                    _uiState.update { it.copy(isSending = false) }
                }
            }
        }
    }

    fun newConversation() {
        disconnectWebSocket()
        _uiState.update {
            it.copy(
                currentConversationId = UUID.randomUUID().toString(),
                streamingContent = "",
                streamingThinking = "",
                activeToolName = null,
                isStreaming = false,
                isSending = false
            )
        }
        connectWebSocket()
    }

    fun switchConversation(convId: String) {
        _uiState.update { it.copy(currentConversationId = convId) }
    }

    fun deleteConversation(convId: String) {
        viewModelScope.launch {
            messageRepo.deleteConversation(convId)
            if (_uiState.value.currentConversationId == convId) {
                newConversation()
            }
        }
    }

    fun dismissError() {
        _uiState.update { it.copy(errorMessage = null) }
    }

    private fun extractReply(json: String): String {
        return try {
            val map = gson.fromJson<Map<String, Any>>(
                json,
                object : TypeToken<Map<String, Any>>() {}.type
            )
            val result = map["result"] as? Map<*, *>
            result?.get("content") as? String
                ?: result?.get("reply") as? String
                ?: result?.get("message") as? String
                ?: map["result"]?.toString()
                ?: json
        } catch (_: Exception) {
            json
        }
    }

    private fun escapeJson(s: String): String {
        return s.replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
    }

    override fun onCleared() {
        disconnectWebSocket()
        super.onCleared()
    }
}
