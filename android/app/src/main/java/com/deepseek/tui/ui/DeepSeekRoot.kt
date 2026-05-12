package com.deepseek.tui.ui

import android.net.Uri
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Dashboard
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.compose.viewModel
import com.deepseek.tui.DeepSeekApp
import com.deepseek.tui.ui.chat.ChatScreen
import com.deepseek.tui.ui.dashboard.DashboardScreen
import com.deepseek.tui.ui.navigation.NavigationDrawer
import com.deepseek.tui.ui.settings.SettingsScreen
import com.deepseek.tui.ui.theme.*
import com.deepseek.tui.viewmodel.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Root composable — hosts the full split-pane layout with
 * navigation drawer, dashboard, chat, and settings.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DeepSeekRoot() {
    val context = LocalContext.current
    val app = context.applicationContext as DeepSeekApp

    // ViewModel factory
    class ViewModelFactory(val creator: () -> ViewModel) : ViewModelProvider.Factory {
        @Suppress("UNCHECKED_CAST")
        override fun <T : ViewModel> create(modelClass: Class<T>): T = creator() as T
    }

    // ViewModels
    val connectionViewModel: ConnectionViewModel = viewModel(
        factory = ViewModelFactory { ConnectionViewModel(app) }
    )
    val dashboardViewModel: DashboardViewModel = viewModel(
        factory = ViewModelFactory { DashboardViewModel(app) }
    )
    val chatViewModel: ChatViewModel = viewModel(
        factory = ViewModelFactory { ChatViewModel(app) }
    )

    val connectionState by connectionViewModel.uiState.collectAsState()
    val dashboardState by dashboardViewModel.uiState.collectAsState()
    val chatState by chatViewModel.uiState.collectAsState()

    // UI state
    var drawerOpen by remember { mutableStateOf(false) }
    var selectedScreen by remember { mutableStateOf("chat") } // "chat", "dashboard", "settings"
    var fontSize by remember { mutableFloatStateOf(14f) }
    var paneRatio by remember { mutableFloatStateOf(0.4f) } // 40% dashboard / 60% chat
    var showClearDataDialog by remember { mutableStateOf(false) }

    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()

    // SSH key import file picker
    val keyImportLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument()
    ) { uri: Uri? ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            try {
                val bytes = withContext(Dispatchers.IO) {
                    context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
                }
                if (bytes == null || bytes.isEmpty()) {
                    Toast.makeText(context, "Failed to read key file", Toast.LENGTH_SHORT).show()
                    return@launch
                }
                app.appContainer.keyStoreManager.importSshKey(bytes)
                connectionViewModel.refreshKeyStatus()
                Toast.makeText(context, "SSH key imported successfully", Toast.LENGTH_SHORT).show()
            } catch (e: Exception) {
                Toast.makeText(
                    context,
                    "Import failed: ${e.message}",
                    Toast.LENGTH_SHORT
                ).show()
            }
        }
    }

    // Clear data confirmation dialog
    if (showClearDataDialog) {
        AlertDialog(
            onDismissRequest = { showClearDataDialog = false },
            title = { Text("Clear All Local Data") },
            text = {
                Text(
                    "This will delete all messages, agents, SSH keys, and connection settings. " +
                    "This action cannot be undone. Continue?"
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        showClearDataDialog = false
                        scope.launch {
                            try {
                                app.appContainer.clearAllData()
                                connectionViewModel.refreshKeyStatus()
                                chatViewModel.newConversation()
                                snackbarHostState.showSnackbar("Data cleared")
                            } catch (e: Exception) {
                                Toast.makeText(
                                    context,
                                    "Clear failed: ${e.message}",
                                    Toast.LENGTH_SHORT
                                ).show()
                            }
                        }
                    },
                    colors = ButtonDefaults.textButtonColors(contentColor = StatusRed)
                ) {
                    Text("Clear All Data")
                }
            },
            dismissButton = {
                TextButton(onClick = { showClearDataDialog = false }) {
                    Text("Cancel")
                }
            }
        )
    }

    // Start polling when connected
    LaunchedEffect(connectionState.state) {
        if (connectionState.state == com.deepseek.tui.connection.SshTunnelManager.TunnelState.CONNECTED) {
            dashboardViewModel.startPolling()
        } else {
            dashboardViewModel.stopPolling()
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
                // Bottom nav bar
                NavigationBar(
                    containerColor = Surface,
                    contentColor = OnSurface
                ) {
                    NavigationBarItem(
                        selected = selectedScreen == "chat",
                        onClick = { selectedScreen = "chat" },
                        icon = { Icon(Icons.Filled.Chat, "Chat") },
                        label = { Text("Chat") },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = Primary,
                            selectedTextColor = Primary,
                            indicatorColor = SurfaceVariant
                        )
                    )
                    NavigationBarItem(
                        selected = selectedScreen == "dashboard",
                        onClick = { selectedScreen = "dashboard" },
                        icon = { Icon(Icons.Filled.Dashboard, "Dashboard") },
                        label = { Text("Dashboard") },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = Primary,
                            selectedTextColor = Primary,
                            indicatorColor = SurfaceVariant
                        )
                    )
                    NavigationBarItem(
                        selected = selectedScreen == "settings",
                        onClick = { selectedScreen = "settings" },
                        icon = { Icon(Icons.Filled.Settings, "Settings") },
                        label = { Text("Settings") },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = Primary,
                            selectedTextColor = Primary,
                            indicatorColor = SurfaceVariant
                        )
                    )
                }
            }
        ) { padding ->
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .pointerInput(Unit) {
                        detectHorizontalDragGestures { _, dragAmount ->
                            if (dragAmount > 50f) drawerOpen = true
                        }
                    }
            ) {
                when (selectedScreen) {
                    "dashboard" -> {
                        // Full-screen dashboard
                        DashboardScreen(
                            connectionState = connectionState,
                            dashboardState = dashboardState,
                            onDisconnect = { connectionViewModel.disconnect() },
                            modifier = Modifier.fillMaxSize()
                        )
                    }
                    "settings" -> {
                        SettingsScreen(
                            fontSize = fontSize,
                            paneRatio = paneRatio,
                            onFontSizeChanged = { fontSize = it },
                            onPaneRatioChanged = { paneRatio = it },
                            onImportKey = { keyImportLauncher.launch(arrayOf("*/*")) },
                            onClearData = { showClearDataDialog = true },
                            modifier = Modifier.fillMaxSize()
                        )
                    }
                    else -> {
                        // Split-pane layout (chat + optional dashboard)
                        if (connectionState.state == com.deepseek.tui.connection.SshTunnelManager.TunnelState.CONNECTED) {
                            // Connected: show split
                            Column(modifier = Modifier.fillMaxSize()) {
                                // Dashboard (top pane)
                                DashboardScreen(
                                    connectionState = connectionState,
                                    dashboardState = dashboardState,
                                    onDisconnect = { connectionViewModel.disconnect() },
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .fillMaxHeight(paneRatio)
                                )

                                HorizontalDivider(color = Divider, thickness = 2.dp)

                                // Chat (bottom pane)
                                ChatScreen(
                                    chatState = chatState,
                                    onInputChanged = { chatViewModel.onInputChanged(it) },
                                    onSend = { chatViewModel.sendMessage() },
                                    onNewConversation = { chatViewModel.newConversation() },
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .weight(1f)
                                )
                            }
                        } else {
                            // Disconnected: show connect button + status
                            Box(
                                modifier = Modifier.fillMaxSize(),
                                contentAlignment = androidx.compose.ui.Alignment.Center
                            ) {
                                Column(
                                    horizontalAlignment = androidx.compose.ui.Alignment.CenterHorizontally
                                ) {
                                    Text(
                                        text = "DeepSeek TUI",
                                        style = MaterialTheme.typography.headlineMedium,
                                        color = Primary
                                    )
                                    Spacer(modifier = Modifier.height(8.dp))
                                    Text(
                                        text = connectionState.host,
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = OnSurface.copy(alpha = 0.6f)
                                    )
                                    Spacer(modifier = Modifier.height(24.dp))

                                    if (!connectionState.hasSshKey) {
                                        Text(
                                            text = "Import SSH key to connect",
                                            style = MaterialTheme.typography.bodySmall,
                                            color = StatusRed
                                        )
                                        Spacer(modifier = Modifier.height(12.dp))
                                    }

                                    Button(
                                        onClick = { connectionViewModel.connect() },
                                        enabled = connectionState.state != com.deepseek.tui.connection.SshTunnelManager.TunnelState.CONNECTING
                                    ) {
                                        Text(
                                            when (connectionState.state) {
                                                com.deepseek.tui.connection.SshTunnelManager.TunnelState.CONNECTING -> "Connecting…"
                                                else -> "Connect"
                                            }
                                        )
                                    }

                                    if (connectionState.errorMessage != null) {
                                        Spacer(modifier = Modifier.height(12.dp))
                                        Text(
                                            text = connectionState.errorMessage!!,
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.error
                                        )
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
