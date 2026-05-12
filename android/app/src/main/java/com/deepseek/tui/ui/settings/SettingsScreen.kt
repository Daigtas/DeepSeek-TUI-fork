package com.deepseek.tui.ui.settings

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.deepseek.tui.data.prefs.ConnectionConfig
import com.deepseek.tui.ui.theme.*

@Composable
fun SettingsScreen(
    fontSize: Float,
    paneRatio: Float,
    connectionConfig: ConnectionConfig,
    onFontSizeChanged: (Float) -> Unit,
    onPaneRatioChanged: (Float) -> Unit,
    onConnectionConfigChanged: (ConnectionConfig) -> Unit,
    onImportKey: () -> Unit,
    onClearData: () -> Unit,
    onModelChanged: (String) -> Unit = {},
    onProviderChanged: (String) -> Unit = {},
    onThinkingEffortChanged: (String) -> Unit = {},
    onAutoModeChanged: (Boolean) -> Unit = {},
    onApiKeyChanged: (String) -> Unit = {},
    onBaseUrlChanged: (String) -> Unit = {},
    onDetach: () -> Unit = {},
    onAttach: () -> Unit = {},
    onCheckpoint: () -> Unit = {},
    daemonConnected: Boolean = false,
    modifier: Modifier = Modifier
) {
    val models = listOf("auto", "deepseek-v4-pro", "deepseek-v4-flash")
    val providers = listOf("deepseek", "nvidia-nim", "fireworks", "sglang", "vllm")
    val thinkingLevels = listOf("off", "high", "max")

    var editHost by remember { mutableStateOf(connectionConfig.host) }
    var editPort by remember { mutableStateOf(connectionConfig.port.toString()) }
    var editUser by remember { mutableStateOf(connectionConfig.user ?: "") }
    var editPassword by remember { mutableStateOf(connectionConfig.password ?: "") }

    var selectedModel by remember { mutableStateOf("auto") }
    var selectedProvider by remember { mutableStateOf("deepseek") }
    var selectedThinking by remember { mutableStateOf("high") }
    var autoMode by remember { mutableStateOf(true) }
    var apiKey by remember { mutableStateOf("") }
    var baseUrl by remember { mutableStateOf("") }

    Column(
        modifier = modifier.verticalScroll(rememberScrollState()).padding(16.dp)
    ) {
        Text("Settings", style = MaterialTheme.typography.headlineMedium, color = Primary, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(16.dp))

        // ── AI Model ──
        sectionHeader("AI Model")
        labeledSelect("Model", models, selectedModel) { selectedModel = it; onModelChanged(it) }
        labeledSelect("Provider", providers, selectedProvider) { selectedProvider = it; onProviderChanged(it) }
        labeledSelect("Thinking", thinkingLevels, selectedThinking) { selectedThinking = it; onThinkingEffortChanged(it) }
        labeledSwitch("Auto Mode", autoMode) { autoMode = it; onAutoModeChanged(it) }

        // ── API Config ──
        sectionHeader("API Config")
        labeledField("API Key", apiKey, isPassword = true) { apiKey = it; onApiKeyChanged(it) }
        labeledField("Base URL", baseUrl) { baseUrl = it; onBaseUrlChanged(it) }

        // ── Appearance ──
        sectionHeader("Appearance")
        labeledSlider("Font Size", fontSize, 10f..24f) { onFontSizeChanged(it) }
        labeledSlider("Pane Ratio", paneRatio, 0.2f..0.8f) { onPaneRatioChanged(it) }

        // ── SSH Connection ──
        sectionHeader("SSH Connection")
        labeledField("Hostname", editHost) {
            editHost = it
            onConnectionConfigChanged(connectionConfig.copy(host = it))
        }
        labeledField("Port", editPort) { port ->
            editPort = port
            onConnectionConfigChanged(connectionConfig.copy(port = port.toIntOrNull() ?: 8484))
        }
        labeledField("Username", editUser) {
            editUser = it
            onConnectionConfigChanged(connectionConfig.copy(user = it))
        }
        labeledField("Password", editPassword, isPassword = true) {
            editPassword = it
            onConnectionConfigChanged(connectionConfig.copy(password = it))
        }

        // ── Security ──
        sectionHeader("Security")
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Button(onClick = onImportKey) { Text("Import Key") }
            OutlinedButton(onClick = onClearData) { Text("Clear Data", color = MaterialTheme.colorScheme.error) }
        }

        // ── Daemon Control ──
        sectionHeader("Daemon Control")
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = onDetach, enabled = daemonConnected) { Text("Detach") }
            OutlinedButton(onClick = onAttach, enabled = daemonConnected) { Text("Attach") }
            OutlinedButton(onClick = onCheckpoint) { Text("Checkpoint") }
        }

        // ── About ──
        sectionHeader("About")
        Text("DeepSeek TUI v0.8.26", style = MaterialTheme.typography.bodyMedium, color = OnSurface)
        Text("Android wrapper for DeepSeek TUI daemon", style = MaterialTheme.typography.bodySmall, color = OnSurface.copy(alpha = 0.5f))

        Spacer(Modifier.height(32.dp))
    }
}

// ── Reusable components ──

@Composable
private fun sectionHeader(title: String) {
    Spacer(Modifier.height(20.dp))
    HorizontalDivider(color = Divider, thickness = 1.dp)
    Text(title, style = MaterialTheme.typography.titleSmall, color = Primary, fontWeight = FontWeight.Bold, modifier = Modifier.padding(vertical = 8.dp))
}

@Composable
private fun labeledSelect(
    label: String, items: List<String>, selected: String, onChange: (String) -> Unit
) {
    var expanded by remember { mutableStateOf(false) }
    Column(Modifier.padding(vertical = 4.dp)) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = OnSurface.copy(alpha = 0.6f))
        Box {
            OutlinedButton(onClick = { expanded = true }, modifier = Modifier.fillMaxWidth()) {
                Text(selected, Modifier.weight(1f), color = OnSurface)
            }
            DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                items.forEach { item ->
                    DropdownMenuItem(text = { Text(item) }, onClick = { onChange(item); expanded = false })
                }
            }
        }
    }
}

@Composable
private fun labeledField(
    label: String, value: String, isPassword: Boolean = false, onChange: (String) -> Unit
) {
    Column(Modifier.padding(vertical = 4.dp)) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = OnSurface.copy(alpha = 0.6f))
        OutlinedTextField(
            value = value, onValueChange = onChange, modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            visualTransformation = if (isPassword) androidx.compose.ui.text.input.PasswordVisualTransformation() else androidx.compose.ui.text.input.VisualTransformation.None,
            colors = fieldColors()
        )
    }
}

@Composable
private fun labeledSlider(
    label: String, value: Float, range: ClosedFloatingPointRange<Float>, onChange: (Float) -> Unit
) {
    Column(Modifier.padding(vertical = 4.dp)) {
        Text("$label: ${"%.2f".format(value)}", style = MaterialTheme.typography.labelMedium, color = OnSurface.copy(alpha = 0.6f))
        Slider(value = value, onValueChange = onChange, valueRange = range, colors = SliderDefaults.colors(thumbColor = Primary, activeTrackColor = Primary))
    }
}

@Composable
private fun labeledSwitch(
    label: String, checked: Boolean, onChange: (Boolean) -> Unit
) {
    Row(Modifier.fillMaxWidth().padding(vertical = 4.dp), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = OnSurface.copy(alpha = 0.6f))
        Switch(checked = checked, onCheckedChange = onChange, colors = SwitchDefaults.colors(checkedThumbColor = Primary, checkedTrackColor = PrimaryVariant))
    }
}

@Composable
private fun fieldColors() = OutlinedTextFieldDefaults.colors(
    focusedTextColor = OnSurface, unfocusedTextColor = OnSurface,
    focusedBorderColor = Primary, unfocusedBorderColor = SurfaceVariant,
    cursorColor = Primary
)
