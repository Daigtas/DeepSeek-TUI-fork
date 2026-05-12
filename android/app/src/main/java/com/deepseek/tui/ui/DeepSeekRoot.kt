package com.deepseek.tui.ui

import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.compose.viewModel
import com.deepseek.tui.DeepSeekApp
import com.deepseek.tui.connection.SshTunnelManager
import com.deepseek.tui.ui.chat.ChatScreen
import com.deepseek.tui.ui.dashboard.DashboardScreen
import com.deepseek.tui.ui.navigation.NavigationDrawer
import com.deepseek.tui.ui.hive.HiveScreen
import com.deepseek.tui.ui.sessions.SessionsScreen
import com.deepseek.tui.ui.settings.SettingsScreen
import com.deepseek.tui.ui.swarm.SwarmScreen
import com.deepseek.tui.ui.theme.*
import com.deepseek.tui.viewmodel.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DeepSeekRoot() {
    val context = LocalContext.current
    val app = context.applicationContext as DeepSeekApp
    val coroutineScope = rememberCoroutineScope()
    val snackbarHostState = remember { SnackbarHostState() }

    class ViewModelFactory(val creator: () -> ViewModel) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T = creator() as T
    }

    val connectionViewModel: ConnectionViewModel = viewModel(factory = ViewModelFactory { ConnectionViewModel(app) })
    val dashboardViewModel: DashboardViewModel = viewModel(factory = ViewModelFactory { DashboardViewModel(app) })
    val chatViewModel: ChatViewModel = viewModel(factory = ViewModelFactory { ChatViewModel(app) })
    val sessionsViewModel: SessionsViewModel = viewModel(factory = ViewModelFactory { SessionsViewModel(app) })
    val swarmViewModel: SwarmViewModel = viewModel(factory = ViewModelFactory { SwarmViewModel(app) })
    val hiveViewModel: HiveViewModel = viewModel(factory = ViewModelFactory { HiveViewModel(app) })

    val connectionState by connectionViewModel.uiState.collectAsState()
    val dashboardState by dashboardViewModel.uiState.collectAsState()
    val chatState by chatViewModel.uiState.collectAsState()

    var drawerOpen by remember { mutableStateOf(false) }
    var selectedScreen by remember { mutableStateOf("chat") }
    var fontSize by remember { mutableFloatStateOf(14f) }
    var paneRatio by remember { mutableFloatStateOf(0.4f) }

    // AI settings (persisted locally; also sent to server when connected)
    var appModel by remember { mutableStateOf("deepseek-v4-pro") }
    var appProvider by remember { mutableStateOf("deepseek") }
    var appThinking by remember { mutableStateOf("high") }
    var appAutoMode by remember { mutableStateOf(false) }

    val sessionsState by sessionsViewModel.uiState.collectAsState()
    val swarmState by swarmViewModel.uiState.collectAsState()
    val hiveState by hiveViewModel.uiState.collectAsState()

    // SSH key import launcher
    // File attachment picker for chat
    val chatAttachmentLauncher = rememberLauncherForActivityResult(contract = ActivityResultContracts.OpenDocument()) { uri ->
        uri?.let {
            coroutineScope.launch {
                val path = it.path ?: it.lastPathSegment ?: "file"
                chatViewModel.onInputChanged(chatState.inputText + " @$path")
            }
        }
    }

    val keyImportLauncher = rememberLauncherForActivityResult(contract = ActivityResultContracts.OpenDocument()) { uri ->
        uri?.let {
            coroutineScope.launch {
                try {
                    val bytes = withContext(Dispatchers.IO) { context.contentResolver.openInputStream(it)?.readBytes() }
                    if (bytes != null) {
                        app.appContainer.keyStoreManager.importSshKey(bytes)
                        connectionViewModel.refreshKeyStatus()
                        snackbarHostState.showSnackbar("SSH key imported")
                    }
                } catch (e: Exception) {
                    snackbarHostState.showSnackbar("Import failed: ${e.message}")
                }
            }
        }
    }

    // Clear data dialog
    var showClearDialog by remember { mutableStateOf(false) }
    if (showClearDialog) {
        AlertDialog(
            onDismissRequest = { showClearDialog = false },
            title = { Text("Clear All Data?") },
            text = { Text("This will remove all messages, cached data, and settings.") },
            confirmButton = {
                TextButton(onClick = {
                    showClearDialog = false
                    coroutineScope.launch {
                        app.appContainer.clearAllData()
                        connectionViewModel.refreshKeyStatus()
                        chatViewModel.newConversation()
                        snackbarHostState.showSnackbar("Data cleared")
                    }
                }) { Text("Clear", color = StatusRed) }
            },
            dismissButton = { TextButton(onClick = { showClearDialog = false }) { Text("Cancel") } }
        )
    }

    // Host key dialog
    val pendingHostKey = connectionState.pendingHostKey
    if (pendingHostKey != null) {
        AlertDialog(
            onDismissRequest = { connectionViewModel.rejectHostKey() },
            title = { Text("Unknown Host Key") },
            text = {
                Column {
                    Text("The authenticity of host '${pendingHostKey.host}' can't be established.")
                    Spacer(Modifier.height(8.dp))
                    Text("Key type: ${pendingHostKey.keyType}", fontWeight = FontWeight.Bold)
                    Text("Fingerprint: ${pendingHostKey.fingerprint}", fontFamily = FontFamily.Monospace, fontSize = 11.sp)
                    Spacer(Modifier.height(8.dp))
                    Text("Accept and continue?", color = StatusYellow)
                }
            },
            confirmButton = { TextButton(onClick = { connectionViewModel.acceptHostKey() }) { Text("Accept", color = StatusGreen) } },
            dismissButton = { TextButton(onClick = { connectionViewModel.rejectHostKey() }) { Text("Reject", color = StatusRed) } }
        )
    }

    LaunchedEffect(connectionState.state) {
        if (connectionState.state == SshTunnelManager.TunnelState.CONNECTED) {
            dashboardViewModel.startPolling()
            chatViewModel.connectWebSocket()
        } else {
            dashboardViewModel.stopPolling()
            chatViewModel.disconnectWebSocket()
        }
    }

    ModalNavigationDrawer(
        drawerState = rememberDrawerState(if (drawerOpen) DrawerValue.Open else DrawerValue.Closed),
        gesturesEnabled = drawerOpen,
        drawerContent = {
            NavigationDrawer(
                selectedItemId = selectedScreen,
                hasSshKey = connectionState.hasSshKey,
                keyFingerprint = connectionState.keyFingerprint,
                onItemSelected = { selectedScreen = it },
                onImportKey = { keyImportLauncher.launch(arrayOf("*/*")) },
                onClose = { drawerOpen = false }
            )
        }
    ) {
        Scaffold(
            snackbarHost = { SnackbarHost(snackbarHostState) },
            containerColor = Background,
            bottomBar = {
                NavigationBar(containerColor = Surface, contentColor = OnSurface) {
                    NavigationBarItem(selected = selectedScreen == "chat", onClick = { selectedScreen = "chat" },
                        icon = { Icon(Icons.Filled.Chat, "Chat") }, label = { Text("Chat") },
                        colors = navColors())
                    NavigationBarItem(selected = selectedScreen == "dashboard", onClick = { selectedScreen = "dashboard" },
                        icon = { Icon(Icons.Filled.Dashboard, "Dashboard") }, label = { Text("Dashboard") },
                        colors = navColors())
                    NavigationBarItem(selected = selectedScreen == "swarm", onClick = { selectedScreen = "swarm" },
                        icon = { Icon(Icons.Filled.Group, "Swarm") }, label = { Text("Swarm") },
                        colors = navColors())
                    NavigationBarItem(selected = selectedScreen == "hive", onClick = { selectedScreen = "hive" },
                        icon = { Icon(Icons.Filled.Storage, "Hive") }, label = { Text("Hive") },
                        colors = navColors())
                    NavigationBarItem(selected = selectedScreen == "sessions", onClick = { selectedScreen = "sessions" },
                        icon = { Icon(Icons.Filled.History, "Sessions") }, label = { Text("Sessions") },
                        colors = navColors())
                    NavigationBarItem(selected = selectedScreen == "settings", onClick = { selectedScreen = "settings" },
                        icon = { Icon(Icons.Filled.Settings, "Settings") }, label = { Text("Settings") },
                        colors = navColors())
                }
            }
        ) { padding ->
            Box(Modifier.fillMaxSize().padding(padding).pointerInput(Unit) {
                detectHorizontalDragGestures { _, dragAmount -> if (dragAmount > 50f) drawerOpen = true }
            }) {
                when (selectedScreen) {
                    "dashboard" -> DashboardScreen(connectionState, dashboardState, { connectionViewModel.disconnect() }, {}, {}, {}, Modifier.fillMaxSize())
                    "swarm" -> SwarmScreen(swarmState, { swarmViewModel.refreshAgents() }, { r, n, p -> swarmViewModel.spawnAgent(r, n, p) }, { swarmViewModel.clearMessage() })
                    "hive" -> HiveScreen(hiveState, { hiveViewModel.refreshSnapshot() }, { hiveViewModel.queryByKey(it) }, { k, v -> hiveViewModel.injectEntry(k, v) }, { _ -> }, { hiveViewModel.clearMessages() })
                    "sessions" -> SessionsScreen(sessionsState, { sessionsViewModel.loadSessions() }, { sessionsViewModel.deleteSession(it) }, { sessionsViewModel.exportSession(it) })
                    "settings" -> SettingsScreen(fontSize, paneRatio, connectionState.config,
                        onFontSizeChanged = { fontSize = it }, onPaneRatioChanged = { paneRatio = it },
                        onConnectionConfigChanged = { connectionViewModel.saveConfig(it) },
                        onImportKey = { keyImportLauncher.launch(arrayOf("*/*")) },
                        onClearData = { showClearDialog = true },
                        onModelChanged = { connectionViewModel.setModel(it) },
                        onProviderChanged = { connectionViewModel.setProvider(it) },
                        onThinkingEffortChanged = { connectionViewModel.setThinkingEffort(it) },
                        onAutoModeChanged = { connectionViewModel.setAutoMode(it) },
                        daemonConnected = connectionState.state == SshTunnelManager.TunnelState.CONNECTED,
                        onDetach = { connectionViewModel.detachDaemon() },
                        onAttach = { connectionViewModel.attachDaemon() },
                        onCheckpoint = { connectionViewModel.saveCheckpoint() })
                    else -> {
                        if (connectionState.state == SshTunnelManager.TunnelState.CONNECTED) {
                            Column(Modifier.fillMaxSize()) {
                                DashboardScreen(connectionState, dashboardState, { connectionViewModel.disconnect() }, {}, {}, {}, Modifier.fillMaxWidth().fillMaxHeight(paneRatio))
                                HorizontalDivider(color = Divider, thickness = 2.dp)
                                ChatScreen(chatState, { chatViewModel.onInputChanged(it) }, { chatViewModel.sendMessage() }, { chatViewModel.newConversation() }, onAttachFile = { chatAttachmentLauncher.launch(arrayOf("*/*")) }, modifier = Modifier.fillMaxWidth().weight(1f))
                            }
                        } else {
                            Column(Modifier.fillMaxSize().padding(16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                Spacer(Modifier.height(32.dp))
                                Text("DeepSeek TUI", style = MaterialTheme.typography.headlineLarge, color = Primary, fontWeight = FontWeight.Bold)
                                Text("${connectionState.config.host}:${connectionState.config.port}", style = MaterialTheme.typography.bodyMedium, color = OnSurface.copy(alpha = 0.6f))
                                Spacer(Modifier.height(24.dp))
                                if (!connectionState.hasSshKey && connectionState.config.password.isNullOrBlank()) {
                                    Text("Import SSH key or set password to connect", style = MaterialTheme.typography.bodySmall, color = StatusRed)
                                    Spacer(Modifier.height(12.dp))
                                }
                                Button(onClick = { connectionViewModel.connect() },
                                    enabled = connectionState.state != SshTunnelManager.TunnelState.CONNECTING && connectionState.state != SshTunnelManager.TunnelState.HOST_KEY_UNKNOWN) {
                                    Text(when (connectionState.state) { SshTunnelManager.TunnelState.CONNECTING -> "Connecting…"; SshTunnelManager.TunnelState.HOST_KEY_UNKNOWN -> "Awaiting host key…"; else -> "Connect" })
                                }
                                if (connectionState.errorMessage != null) {
                                    Spacer(Modifier.height(12.dp))
                                    Text(connectionState.errorMessage!!, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
                                }
                                if (connectionState.logMessages.isNotEmpty()) {
                                    Spacer(Modifier.height(16.dp))
                                    Card(Modifier.fillMaxWidth().weight(1f), colors = CardDefaults.cardColors(containerColor = SurfaceVariant)) {
                                        Column(Modifier.padding(12.dp)) {
                                            Text("Connection Log", style = MaterialTheme.typography.labelLarge, color = Primary, fontWeight = FontWeight.Bold)
                                            Spacer(Modifier.height(4.dp))
                                            LazyColumn { items(connectionState.logMessages) { msg -> Text(msg, fontFamily = FontFamily.Monospace, fontSize = 11.sp, color = OnSurface.copy(alpha = 0.8f), modifier = Modifier.padding(vertical = 1.dp)) } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun navColors() = NavigationBarItemDefaults.colors(selectedIconColor = Primary, selectedTextColor = Primary, indicatorColor = SurfaceVariant)
