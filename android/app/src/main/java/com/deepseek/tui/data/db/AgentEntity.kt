package com.deepseek.tui.data.db

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "agents")
data class AgentEntity(
    @PrimaryKey val agentId: String,
    val role: String,
    val status: String,       // "idle" | "working" | "error"
    val taskDescription: String? = null,
    val lastUpdated: Long     // epoch millis
)
