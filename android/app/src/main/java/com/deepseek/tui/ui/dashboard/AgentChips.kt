package com.deepseek.tui.ui.dashboard

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.deepseek.tui.data.db.AgentEntity
import com.deepseek.tui.ui.theme.*

@Composable
fun AgentChips(
    agents: List<AgentEntity>,
    onInspect: (AgentEntity) -> Unit,
    onKill: (AgentEntity) -> Unit,
    onRestart: (AgentEntity) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxWidth()) {
        Text(
            text = "Active Agents",
            style = MaterialTheme.typography.labelLarge,
            color = OnSurface,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp)
        )

        if (agents.isEmpty()) {
            Text(
                text = "No agents running",
                style = MaterialTheme.typography.bodySmall,
                color = OnSurface.copy(alpha = 0.4f),
                modifier = Modifier.padding(horizontal = 12.dp)
            )
        } else {
            LazyRow(
                contentPadding = PaddingValues(horizontal = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(agents, key = { it.agentId }) { agent ->
                    AgentChip(
                        agent = agent,
                        onInspect = { onInspect(agent) },
                        onKill = { onKill(agent) },
                        onRestart = { onRestart(agent) }
                    )
                }
            }
        }
    }
}

@Composable
private fun AgentChip(
    agent: AgentEntity,
    onInspect: () -> Unit,
    onKill: () -> Unit,
    onRestart: () -> Unit
) {
    var offsetX by remember { mutableFloatStateOf(0f) }
    val swipeThreshold = 80f

    val bgColor by animateColorAsState(
        when (agent.status) {
            "working" -> AgentWorking
            "error" -> AgentError
            else -> AgentIdle
        },
        label = "chipBg"
    )

    Card(
        modifier = Modifier
            .pointerInput(Unit) {
                detectHorizontalDragGestures(
                    onDragEnd = {
                        if (offsetX < -swipeThreshold) onKill()
                        else if (offsetX > swipeThreshold) onInspect()
                        offsetX = 0f
                    },
                    onHorizontalDrag = { _, dragAmount ->
                        offsetX = (offsetX + dragAmount).coerceIn(-200f, 200f)
                    }
                )
            },
        shape = RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = bgColor)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Role icon
            val icon = when (agent.role) {
                "explorer" -> "🔍"
                "planner" -> "📋"
                "implementer" -> "🔧"
                "verifier" -> "✅"
                "review" -> "👀"
                else -> "🤖"
            }
            Text(text = icon, fontSize = MaterialTheme.typography.bodySmall.fontSize)

            Spacer(modifier = Modifier.width(6.dp))

            Column {
                Text(
                    text = agent.role.replaceFirstChar { it.uppercase() },
                    style = MaterialTheme.typography.labelLarge,
                    color = OnPrimary,
                    fontWeight = FontWeight.SemiBold
                )
                if (agent.taskDescription != null) {
                    Text(
                        text = agent.taskDescription.take(30) + if (agent.taskDescription.length > 30) "…" else "",
                        style = MaterialTheme.typography.bodySmall,
                        color = OnPrimary.copy(alpha = 0.7f),
                        maxLines = 1
                    )
                }
            }
        }
    }
}
