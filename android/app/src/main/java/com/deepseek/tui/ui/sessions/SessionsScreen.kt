package com.deepseek.tui.ui.sessions

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.deepseek.tui.ui.components.SwipeableCard
import com.deepseek.tui.ui.theme.*
import com.deepseek.tui.viewmodel.SessionSummary
import com.deepseek.tui.viewmodel.SessionsUiState

@Composable
fun SessionsScreen(
    state: SessionsUiState,
    onRefresh: () -> Unit,
    onDelete: (String) -> Unit,
    onExport: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    var deleteTarget by remember { mutableStateOf<SessionSummary?>(null) }

    if (deleteTarget != null) {
        AlertDialog(
            onDismissRequest = { deleteTarget = null },
            title = { Text("Delete Session?") },
            text = { Text("Delete \"${deleteTarget!!.name}\"? This cannot be undone.") },
            confirmButton = {
                TextButton(onClick = {
                    onDelete(deleteTarget!!.id)
                    deleteTarget = null
                }) { Text("Delete", color = StatusRed) }
            },
            dismissButton = { TextButton(onClick = { deleteTarget = null }) { Text("Cancel") } }
        )
    }

    Column(modifier = modifier.fillMaxSize().padding(12.dp)) {
        Text("Sessions", style = MaterialTheme.typography.headlineMedium, color = Primary, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(4.dp))
        Text("${state.sessions.size} saved", style = MaterialTheme.typography.bodySmall, color = OnSurface.copy(alpha = 0.5f))

        Spacer(Modifier.height(12.dp))

        if (state.errorMessage != null) {
            Text(state.errorMessage!!, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            Spacer(Modifier.height(8.dp))
        }

        if (state.isLoading) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                CircularProgressIndicator(color = Primary)
            }
        } else if (state.sessions.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(Icons.Filled.FolderOff, null, tint = OnSurface.copy(alpha = 0.3f), modifier = Modifier.size(48.dp))
                    Spacer(Modifier.height(8.dp))
                    Text("No saved sessions", style = MaterialTheme.typography.bodyLarge, color = OnSurface.copy(alpha = 0.4f))
                    Spacer(Modifier.height(4.dp))
                    TextButton(onClick = onRefresh) { Text("Refresh") }
                }
            }
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(state.sessions, key = { it.id }) { session ->
                    SwipeableCard(
                        onSwipeRight = { onExport(session.id) },
                        onSwipeLeft = { deleteTarget = session },
                        rightActionLabel = "Export",
                        leftActionLabel = "Delete"
                    ) {
                        SessionCard(session)
                    }
                }
                item { Spacer(Modifier.height(8.dp)) }
            }
        }
    }
}

@Composable
private fun SessionCard(session: SessionSummary) {
    Card(
        colors = CardDefaults.cardColors(containerColor = Surface),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Filled.History, null, tint = Primary, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(8.dp))
                Text(session.name, style = MaterialTheme.typography.titleSmall, color = OnSurface, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                Text("${session.turnCount} turns", style = MaterialTheme.typography.labelSmall, color = Primary)
            }
            if (session.workspace.isNotBlank()) {
                Spacer(Modifier.height(4.dp))
                Text(session.workspace, style = MaterialTheme.typography.bodySmall, color = OnSurface.copy(alpha = 0.5f), maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            Spacer(Modifier.height(6.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                if (session.model.isNotBlank()) {
                    AssistChip(
                        onClick = {}, label = { Text(session.model, style = MaterialTheme.typography.labelSmall) },
                        colors = AssistChipDefaults.assistChipColors(containerColor = SurfaceVariant)
                    )
                }
                Text(session.createdAt.take(10), style = MaterialTheme.typography.labelSmall, color = OnSurface.copy(alpha = 0.4f))
            }
        }
    }
}
