package com.deepseek.tui.ui.swarm

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.deepseek.tui.ui.theme.*
import com.deepseek.tui.viewmodel.SwarmAgent
import com.deepseek.tui.viewmodel.SwarmUiState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SwarmScreen(
    state: SwarmUiState,
    onRefresh: () -> Unit,
    onSpawn: (role: String, name: String, prompt: String) -> Unit,
    onClearMessage: () -> Unit,
    modifier: Modifier = Modifier
) {
    var showSpawnDialog by remember { mutableStateOf(false) }
    var selectedAgent by remember { mutableStateOf<SwarmAgent?>(null) }
    var showDetailDialog by remember { mutableStateOf(false) }

    // Spawn dialog fields
    var spawnRole by remember { mutableStateOf("general") }
    var spawnName by remember { mutableStateOf("") }
    var spawnPrompt by remember { mutableStateOf("") }
    var roleExpanded by remember { mutableStateOf(false) }

    val roles = listOf("explorer", "implementer", "reviewer", "tester", "planner", "coordinator", "general")

    // Message snackbar
    val snackbarHostState = remember { SnackbarHostState() }
    LaunchedEffect(state.spawnMessage) {
        state.spawnMessage?.let {
            snackbarHostState.showSnackbar(it)
            onClearMessage()
        }
    }

    Column(modifier = modifier.fillMaxSize().background(Background)) {
        // Header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Swarm",
                style = MaterialTheme.typography.headlineSmall,
                color = Primary,
                fontWeight = FontWeight.Bold
            )
            Row {
                IconButton(onClick = onRefresh) {
                    Icon(
                        Icons.Filled.Refresh,
                        contentDescription = "Refresh",
                        tint = OnSurface.copy(alpha = 0.7f)
                    )
                }
                IconButton(onClick = {
                    spawnName = ""
                    spawnPrompt = ""
                    spawnRole = "general"
                    showSpawnDialog = true
                }) {
                    Icon(
                        Icons.Filled.Add,
                        contentDescription = "Spawn agent",
                        tint = Primary
                    )
                }
            }
        }

        if (state.isRefreshing) {
            LinearProgressIndicator(
                modifier = Modifier.fillMaxWidth(),
                color = Primary,
                trackColor = SurfaceVariant
            )
        }

        HorizontalDivider(color = Divider)

        if (state.agents.isEmpty()) {
            // Empty state
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(
                        Icons.Filled.Group,
                        contentDescription = null,
                        tint = OnSurface.copy(alpha = 0.2f),
                        modifier = Modifier.size(64.dp)
                    )
                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        text = "No agents running.",
                        style = MaterialTheme.typography.bodyLarge,
                        color = OnSurface.copy(alpha = 0.5f)
                    )
                    Text(
                        text = "Tap + to spawn one.",
                        style = MaterialTheme.typography.bodySmall,
                        color = OnSurface.copy(alpha = 0.3f)
                    )
                }
            }
        } else {
            LazyColumn(
                contentPadding = PaddingValues(12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(state.agents, key = { it.agentId }) { agent ->
                    AgentCard(
                        agent = agent,
                        onClick = {
                            selectedAgent = agent
                            showDetailDialog = true
                        }
                    )
                }
            }
        }
    }

    // Spawn dialog
    if (showSpawnDialog) {
        AlertDialog(
            onDismissRequest = { showSpawnDialog = false },
            title = { Text("Spawn Agent", color = Primary, fontWeight = FontWeight.Bold) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    // Role picker
                    Text("Role", style = MaterialTheme.typography.labelMedium, color = OnSurface)
                    ExposedDropdownMenuBox(
                        expanded = roleExpanded,
                        onExpandedChange = { roleExpanded = it }
                    ) {
                        OutlinedTextField(
                            value = spawnRole,
                            onValueChange = {},
                            readOnly = true,
                            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = roleExpanded) },
                            modifier = Modifier.menuAnchor().fillMaxWidth(),
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedTextColor = OnSurface,
                                unfocusedTextColor = OnSurface,
                                focusedBorderColor = Primary,
                                unfocusedBorderColor = Divider
                            )
                        )
                        ExposedDropdownMenu(
                            expanded = roleExpanded,
                            onDismissRequest = { roleExpanded = false }
                        ) {
                            roles.forEach { role ->
                                DropdownMenuItem(
                                    text = { Text(role) },
                                    onClick = {
                                        spawnRole = role
                                        roleExpanded = false
                                    }
                                )
                            }
                        }
                    }

                    // Name field
                    OutlinedTextField(
                        value = spawnName,
                        onValueChange = { spawnName = it },
                        label = { Text("Name") },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = OnSurface,
                            unfocusedTextColor = OnSurface,
                            focusedBorderColor = Primary,
                            unfocusedBorderColor = Divider,
                            focusedLabelColor = Primary,
                            unfocusedLabelColor = OnSurface.copy(alpha = 0.6f)
                        )
                    )

                    // Prompt field
                    OutlinedTextField(
                        value = spawnPrompt,
                        onValueChange = { spawnPrompt = it },
                        label = { Text("Prompt") },
                        modifier = Modifier.fillMaxWidth().heightIn(min = 80.dp),
                        maxLines = 4,
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = OnSurface,
                            unfocusedTextColor = OnSurface,
                            focusedBorderColor = Primary,
                            unfocusedBorderColor = Divider,
                            focusedLabelColor = Primary,
                            unfocusedLabelColor = OnSurface.copy(alpha = 0.6f)
                        )
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onSpawn(spawnRole, spawnName, spawnPrompt)
                        showSpawnDialog = false
                    },
                    enabled = spawnName.isNotBlank(),
                    colors = ButtonDefaults.buttonColors(containerColor = Primary, contentColor = OnPrimary)
                ) {
                    Text("Spawn")
                }
            },
            dismissButton = {
                TextButton(onClick = { showSpawnDialog = false }) {
                    Text("Cancel", color = OnSurface.copy(alpha = 0.6f))
                }
            },
            containerColor = Surface,
            titleContentColor = OnSurface,
            textContentColor = OnSurface
        )
    }

    // Agent detail dialog
    if (showDetailDialog && selectedAgent != null) {
        val agent = selectedAgent!!
        AlertDialog(
            onDismissRequest = { showDetailDialog = false },
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = roleIcon(agent.role),
                        fontSize = 24.sp
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = agent.name.ifBlank { agent.agentId },
                        color = Primary,
                        fontWeight = FontWeight.Bold
                    )
                }
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    DetailRow("ID", agent.agentId)
                    DetailRow("Role", agent.role)
                    DetailRow("Status", agent.status)
                    DetailRow("Uptime", agent.uptime)
                    if (agent.taskDescription != null) {
                        DetailRow("Task", agent.taskDescription)
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = { showDetailDialog = false }) {
                    Text("Close", color = Primary)
                }
            },
            containerColor = Surface,
            titleContentColor = OnSurface,
            textContentColor = OnSurface
        )
    }
}

@Composable
private fun AgentCard(
    agent: SwarmAgent,
    onClick: () -> Unit
) {
    val statusColor = when (agent.status) {
        "working", "active" -> StatusGreen
        "error", "failed" -> StatusRed
        else -> StatusYellow
    }

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = Surface)
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Status dot
            Box(
                modifier = Modifier
                    .size(10.dp)
                    .background(statusColor, RoundedCornerShape(50))
            )

            Spacer(modifier = Modifier.width(12.dp))

            // Role icon
            Text(
                text = roleIcon(agent.role),
                fontSize = 22.sp
            )

            Spacer(modifier = Modifier.width(12.dp))

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = agent.name.ifBlank { agent.role.replaceFirstChar { it.uppercase() } },
                    style = MaterialTheme.typography.titleMedium,
                    color = OnSurface,
                    fontWeight = FontWeight.SemiBold
                )
                if (agent.taskDescription != null) {
                    Text(
                        text = agent.taskDescription.take(60) + if (agent.taskDescription.length > 60) "…" else "",
                        style = MaterialTheme.typography.bodySmall,
                        color = OnSurface.copy(alpha = 0.6f),
                        maxLines = 2
                    )
                }
                if (agent.uptime.isNotBlank()) {
                    Text(
                        text = agent.uptime,
                        style = MaterialTheme.typography.bodySmall,
                        color = OnSurface.copy(alpha = 0.35f)
                    )
                }
            }

            Icon(
                Icons.Filled.ChevronRight,
                contentDescription = "Details",
                tint = OnSurface.copy(alpha = 0.3f)
            )
        }
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Row {
        Text(
            text = "$label: ",
            style = MaterialTheme.typography.bodyMedium,
            color = OnSurface.copy(alpha = 0.5f),
            fontWeight = FontWeight.Medium
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            color = OnSurface
        )
    }
}

private fun roleIcon(role: String): String = when (role) {
    "explorer" -> "\uD83D\uDD0D"
    "planner" -> "\uD83D\uDCCB"
    "implementer" -> "\uD83D\uDD27"
    "verifier", "tester" -> "\u2705"
    "reviewer", "review" -> "\uD83D\uDC40"
    "coordinator" -> "\uD83E\uDD1D"
    else -> "\uD83E\uDD16"
}
