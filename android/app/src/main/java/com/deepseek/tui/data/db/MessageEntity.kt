package com.deepseek.tui.data.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "messages")
data class MessageEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val conversationId: String,
    val role: String,         // "user" | "assistant" | "system"
    val content: String,
    val timestamp: Long,      // epoch millis
    val agentId: String? = null,
    val agentRole: String? = null,
    val thinkingTokens: String? = null,
    val isStreaming: Boolean = false
)
