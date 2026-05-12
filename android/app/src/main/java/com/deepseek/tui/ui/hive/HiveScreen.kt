package com.deepseek.tui.ui.hive

import androidx.compose.animation.AnimatedVisibility
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
import com.deepseek.tui.viewmodel.HiveEntry
import com.deepseek.tui.viewmodel.HiveUiState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HiveScreen(
    state: HiveUiState,
    onRefresh: () -> Unit,
    onQuery: (String) -> Unit,
    onInject: (key: String, value: String) -> Unit,
    onFilterChange: (String) -> Unit,
    onClearMessages: () -> Unit,
    modifier: Modifier = Modifier
) {
    var showInjectDialog by remember { mutableStateOf(false) }
    var injectKey by remember { mutableStateOf("") }
    var injectValue by remember { mutableStateOf("") }
    var searchQuery by remember { mutableStateOf("") }
    var expandedEntryKey by remember { mutableStateOf<String?>(null) }

    // Snackbar for inject/query results
    val snackbarHostState = remember { SnackbarHostState() }
    LaunchedEffect(state.injectMessage) {
        state.injectMessage?.let {
            snackbarHostState.showSnackbar(it)
            onClearMessages()
        }
    }

    Scaffold(
        snackbarHost = { SnackbarHost(snackbarHostState) },
        containerColor = Background
    ) { padding ->
        Column(
            modifier = modifier
                .fillMaxSize()
                .padding(padding)
                .background(Background)
        ) {
            // Header
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Hive",
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
                        injectKey = ""
                        injectValue = ""
                        showInjectDialog = true
                    }) {
                        Icon(
                            Icons.Filled.Add,
                            contentDescription = "Inject entry",
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

            // Search/Filter bar
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 12.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                OutlinedTextField(
                    value = searchQuery,
                    onValueChange = {
                        searchQuery = it
                        onFilterChange(it)
                    },
                    placeholder = { Text("Filter by key prefix…", color = OnSurface.copy(alpha = 0.4f)) },
                    singleLine = true,
                    leadingIcon = {
                        Icon(
                            Icons.Filled.Search,
                            contentDescription = null,
                            tint = OnSurface.copy(alpha = 0.5f)
                        )
                    },
                    trailingIcon = {
                        if (searchQuery.isNotBlank()) {
                            IconButton(onClick = {
                                searchQuery = ""
                                onFilterChange("")
                            }) {
                                Icon(Icons.Filled.Clear, contentDescription = "Clear", tint = OnSurface.copy(alpha = 0.5f))
                            }
                        }
                    },
                    modifier = Modifier.weight(1f),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = OnSurface,
                        unfocusedTextColor = OnSurface,
                        focusedBorderColor = Primary,
                        unfocusedBorderColor = Divider
                    )
                )

                Spacer(modifier = Modifier.width(8.dp))

                // Query button
                IconButton(onClick = {
                    if (searchQuery.isNotBlank()) {
                        onQuery(searchQuery)
                    }
                }) {
                    Icon(
                        Icons.Filled.PlayArrow,
                        contentDescription = "Query key",
                        tint = Primary
                    )
                }
            }

            HorizontalDivider(color = Divider)

            // Query result
            if (state.queryResult != null) {
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(12.dp),
                    colors = CardDefaults.cardColors(containerColor = SurfaceVariant),
                    shape = RoundedCornerShape(8.dp)
                ) {
                    Column(modifier = Modifier.padding(12.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                text = "Query: ${state.queryKey}",
                                style = MaterialTheme.typography.labelMedium,
                                color = Primary,
                                fontWeight = FontWeight.Bold
                            )
                        }
                        Spacer(modifier = Modifier.height(6.dp))
                        Text(
                            text = state.queryResult!!,
                            fontFamily = FontFamily.Monospace,
                            fontSize = 12.sp,
                            color = OnSurface.copy(alpha = 0.8f)
                        )
                    }
                }
                HorizontalDivider(color = Divider)
            }

            // Entry list
            val filteredEntries = if (state.filterPrefix.isBlank()) {
                state.entries
            } else {
                state.entries.filter { it.key.startsWith(state.filterPrefix) }
            }

            if (filteredEntries.isEmpty() && state.entries.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(
                            Icons.Filled.Storage,
                            contentDescription = null,
                            tint = OnSurface.copy(alpha = 0.2f),
                            modifier = Modifier.size(64.dp)
                        )
                        Spacer(modifier = Modifier.height(16.dp))
                        Text(
                            text = "Hive is empty.",
                            style = MaterialTheme.typography.bodyLarge,
                            color = OnSurface.copy(alpha = 0.5f)
                        )
                        Text(
                            text = "Tap + to inject an entry.",
                            style = MaterialTheme.typography.bodySmall,
                            color = OnSurface.copy(alpha = 0.3f)
                        )
                    }
                }
            } else if (filteredEntries.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "No entries matching \"${state.filterPrefix}\"",
                        style = MaterialTheme.typography.bodyMedium,
                        color = OnSurface.copy(alpha = 0.4f)
                    )
                }
            } else {
                LazyColumn(
                    contentPadding = PaddingValues(12.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    items(filteredEntries, key = { it.key + it.version }) { entry ->
                        val isExpanded = expandedEntryKey == entry.key
                        HiveEntryCard(
                            entry = entry,
                            isExpanded = isExpanded,
                            onClick = {
                                expandedEntryKey = if (isExpanded) null else entry.key
                            }
                        )
                    }
                }
            }
        }
    }

    // Inject dialog
    if (showInjectDialog) {
        AlertDialog(
            onDismissRequest = { showInjectDialog = false },
            title = { Text("Inject into Hive", color = Primary, fontWeight = FontWeight.Bold) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = injectKey,
                        onValueChange = { injectKey = it },
                        label = { Text("Key") },
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
                    OutlinedTextField(
                        value = injectValue,
                        onValueChange = { injectValue = it },
                        label = { Text("Value (JSON)") },
                        modifier = Modifier.fillMaxWidth().heightIn(min = 80.dp),
                        maxLines = 5,
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
                        onInject(injectKey, injectValue)
                        showInjectDialog = false
                    },
                    enabled = injectKey.isNotBlank() && injectValue.isNotBlank(),
                    colors = ButtonDefaults.buttonColors(containerColor = Primary, contentColor = OnPrimary)
                ) {
                    Text("Inject")
                }
            },
            dismissButton = {
                TextButton(onClick = { showInjectDialog = false }) {
                    Text("Cancel", color = OnSurface.copy(alpha = 0.6f))
                }
            },
            containerColor = Surface,
            titleContentColor = OnSurface,
            textContentColor = OnSurface
        )
    }
}

@Composable
private fun HiveEntryCard(
    entry: HiveEntry,
    isExpanded: Boolean,
    onClick: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = Surface)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            // Key row
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    Icons.Filled.Key,
                    contentDescription = null,
                    tint = Primary,
                    modifier = Modifier.size(18.dp)
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = entry.key,
                    style = MaterialTheme.typography.titleSmall,
                    color = Primary,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.weight(1f)
                )
                Text(
                    text = "v${entry.version}",
                    style = MaterialTheme.typography.labelSmall,
                    color = OnSurface.copy(alpha = 0.35f)
                )
            }

            Spacer(modifier = Modifier.height(4.dp))

            // Value preview
            val preview = if (isExpanded) entry.value else entry.value.take(120) + if (entry.value.length > 120) "…" else ""
            Text(
                text = preview,
                fontFamily = FontFamily.Monospace,
                fontSize = 11.sp,
                color = if (isExpanded) OnSurface.copy(alpha = 0.9f) else OnSurface.copy(alpha = 0.6f),
                maxLines = if (isExpanded) 20 else 3
            )

            Spacer(modifier = Modifier.height(4.dp))

            // Meta row
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (entry.author.isNotBlank()) {
                    Text(
                        text = entry.author,
                        style = MaterialTheme.typography.labelSmall,
                        color = OnSurface.copy(alpha = 0.35f)
                    )
                    Text(
                        text = " · ",
                        style = MaterialTheme.typography.labelSmall,
                        color = OnSurface.copy(alpha = 0.25f)
                    )
                }
                if (entry.timestamp.isNotBlank()) {
                    Text(
                        text = entry.timestamp,
                        style = MaterialTheme.typography.labelSmall,
                        color = OnSurface.copy(alpha = 0.35f)
                    )
                }
            }

            // Expand indicator
            AnimatedVisibility(visible = !isExpanded && entry.value.length > 120) {
                Text(
                    text = "Tap to expand",
                    style = MaterialTheme.typography.labelSmall,
                    color = Primary.copy(alpha = 0.5f)
                )
            }
        }
    }
}
