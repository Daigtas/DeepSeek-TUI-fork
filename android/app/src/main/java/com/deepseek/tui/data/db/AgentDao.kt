package com.deepseek.tui.data.db

import androidx.room.*
import kotlinx.coroutines.flow.Flow

@Dao
interface AgentDao {

    @Query("SELECT * FROM agents ORDER BY lastUpdated DESC")
    fun getAllAgents(): Flow<List<AgentEntity>>

    @Query("SELECT * FROM agents WHERE status = :status")
    fun getAgentsByStatus(status: String): Flow<List<AgentEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertAgent(agent: AgentEntity)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertAgents(agents: List<AgentEntity>)

    @Query("DELETE FROM agents")
    suspend fun deleteAll()

    @Query("SELECT COUNT(*) FROM agents")
    suspend fun getAgentCount(): Int
}
