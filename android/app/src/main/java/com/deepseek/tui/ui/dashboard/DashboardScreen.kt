package com.deepseek.tui.ui.dashboard

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.deepseek.tui.data.db.AgentEntity
import com.deepseek.tui.ui.theme.Divider
import com.deepseek.tui.viewmodel.ConnectionUiState
import com.deepseek.tui.viewmodel.DashboardUiState

/**
 * Assembles all dashboard components into a scrollable column.
 * Intended to fill the top ~40% of the split-pane layout.
 */
@Composable
fun DashboardScreen(
    connectionState: ConnectionUiState,
    dashboardState: DashboardUiState,
    onDisconnect: () -> Unit,
    onInspectAgent: (AgentEntity) -> Unit = {},
    onKillAgent: (AgentEntity) -> Unit = {},
    onRestartAgent: (AgentEntity) -> Unit = {},
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .background(MaterialTheme.colorScheme.background)
    ) {
        // Connection bar
        ConnectionBar(
            state = connectionState.state,
            host = connectionState.host,
            latencyMs = connectionState.latencyMs,
            onDisconnect = onDisconnect
        )

        HorizontalDivider(color = Divider)

        // Stats row
        StatsRow(
            activeTaskCount = dashboardState.activeTaskCount,
            connectedClientCount = dashboardState.connectedClientCount,
            daemonUptime = dashboardState.daemonUptime
        )

        HorizontalDivider(color = Divider)

        // Agent chips
        AgentChips(
            agents = dashboardState.agents,
            onInspect = onInspectAgent,
            onKill = onKillAgent,
            onRestart = onRestartAgent
        )

        HorizontalDivider(color = Divider)

        // Progress log
        ProgressLog(
            lines = dashboardState.progressLines,
            modifier = Modifier.heightIn(min = 120.dp, max = 300.dp)
        )

        // Error state
        if (connectionState.errorMessage != null) {
            Text(
                text = connectionState.errorMessage,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(12.dp)
            )
        }
    }
}
