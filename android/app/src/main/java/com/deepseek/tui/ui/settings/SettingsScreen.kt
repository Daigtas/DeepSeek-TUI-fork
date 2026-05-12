package com.deepseek.tui.ui.settings

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import com.deepseek.tui.data.prefs.ConnectionConfig
import com.deepseek.tui.ui.theme.*

data class AppSettings(
    val model: String = "deepseek-v4-pro",
    val provider: String = "deepseek",
    val thinkingEffort: String = "high",
    val autoMode: Boolean = false,
    val apiKey: String = "",
    val baseUrl: String = "",
    val isDarkTheme: Boolean = true
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    fontSize: Float,
    paneRatio: Float,
    connectionConfig: ConnectionConfig,
    appSettings: AppSettings,
    daemonConnected: Boolean,
    onFontSizeChanged: (Float) -> Unit,
    onPaneRatioChanged: (Float) -> Unit,
    onConnectionConfigChanged: (ConnectionConfig) -> Unit,
    onAppSettingsChanged: (AppSettings) -> Unit,
    onImportKey: () -> Unit,
    onClearData: () -> Unit,
    onDetach: () -> Unit = {},
    onAttach: () -> Unit = {},
    onCheckpoint: () -> Unit = {},
    modifier: Modifier = Modifier
) {
    var showPassword by remember { mutableStateOf(false) }
    var showApiKey by remember { mutableStateOf(false) }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp)
    ) {
        Text("Settings", style = MaterialTheme.typography.headlineMedium, color = Primary, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(24.dp))

        // ── AI Model ──────────────────────────────────────────────────
        SectionHeader("AI Model")
        Spacer(Modifier.height(8.dp))

        // Model picker
        val models = listOf("deepseek-v4-pro", "deepseek-v4-flash", "auto")
        var modelExpanded by remember { mutableStateOf(false) }
        ExposedDropdownMenuBox(expanded = modelExpanded, onExpandedChange = { modelExpanded = it }) {
            OutlinedTextField(
                value = appSettings.model, onValueChange = {},
                readOnly = true, label = { Text("Model") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = modelExpanded) },
                modifier = Modifier.fillMaxWidth().menuAnchor(),
                colors = fieldColors()
            )
            ExposedDropdownMenu(expanded = modelExpanded, onDismissRequest = { modelExpanded = false }) {
                models.forEach { m ->
                    DropdownMenuItem(text = { Text(m) }, onClick = {
                        onAppSettingsChanged(appSettings.copy(model = m))
                        modelExpanded = false
                    })
                }
            }
        }

        Spacer(Modifier.height(8.dp))

        // Provider picker
        val providers = listOf("deepseek", "nvidia-nim", "fireworks", "sglang", "vllm")
        var provExpanded by remember { mutableStateOf(false) }
        ExposedDropdownMenuBox(expanded = provExpanded, onExpandedChange = { provExpanded = it }) {
            OutlinedTextField(
                value = appSettings.provider, onValueChange = {},
                readOnly = true, label = { Text("Provider") },
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = provExpanded) },
                modifier = Modifier.fillMaxWidth().menuAnchor(),
                colors = fieldColors()
            )
            ExposedDropdownMenu(expanded = provExpanded, onDismissRequest = { provExpanded = false }) {
                providers.forEach { p ->
                    DropdownMenuItem(text = { Text(p) }, onClick = {
                        onAppSettingsChanged(appSettings.copy(provider = p))
                        provExpanded = false
                    })
                }
            }
        }

        Spacer(Modifier.height(12.dp))

        // Thinking effort
        Text("Thinking Effort", style = MaterialTheme.typography.labelLarge, color = OnSurface)
        Spacer(Modifier.height(4.dp))
        val efforts = listOf("off", "high", "max")
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            efforts.forEach { e ->
                FilterChip(
                    selected = appSettings.thinkingEffort == e,
                    onClick = { onAppSettingsChanged(appSettings.copy(thinkingEffort = e)) },
                    label = { Text(e.replaceFirstChar { it.uppercase() }) },
                    colors = FilterChipDefaults.filterChipColors(
                        selectedContainerColor = Primary,
                        selectedLabelColor = OnPrimary
                    )
                )
            }
        }

        Spacer(Modifier.height(8.dp))

        // Auto mode
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Auto Mode", style = MaterialTheme.typography.bodyMedium, color = OnSurface, modifier = Modifier.weight(1f))
            Switch(
                checked = appSettings.autoMode,
                onCheckedChange = { onAppSettingsChanged(appSettings.copy(autoMode = it)) },
                colors = SwitchDefaults.colors(checkedThumbColor = Primary, checkedTrackColor = PrimaryVariant)
            )
        }

        DividerSection()

        // ── API Configuration ──────────────────────────────────────────
        SectionHeader("API Configuration")
        Spacer(Modifier.height(8.dp))

        OutlinedTextField(
            value = appSettings.apiKey, onValueChange = { onAppSettingsChanged(appSettings.copy(apiKey = it)) },
            label = { Text("API Key") }, singleLine = true,
            visualTransformation = if (showApiKey) VisualTransformation.None else PasswordVisualTransformation(),
            trailingIcon = {
                IconButton(onClick = { showApiKey = !showApiKey }) {
                    Icon(if (showApiKey) Icons.Filled.VisibilityOff else Icons.Filled.Visibility, null, tint = OnSurface.copy(alpha = 0.6f))
                }
            },
            modifier = Modifier.fillMaxWidth(), colors = fieldColors()
        )

        Spacer(Modifier.height(8.dp))

        OutlinedTextField(
            value = appSettings.baseUrl, onValueChange = { onAppSettingsChanged(appSettings.copy(baseUrl = it)) },
            label = { Text("Base URL (optional)") }, placeholder = { Text("https://api.deepseek.com") },
            singleLine = true, modifier = Modifier.fillMaxWidth(), colors = fieldColors()
        )

        DividerSection()

        // ── Appearance ─────────────────────────────────────────────────
        SectionHeader("Appearance")
        Spacer(Modifier.height(8.dp))

        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(if (appSettings.isDarkTheme) "Dark Theme" else "Light Theme", style = MaterialTheme.typography.bodyMedium, color = OnSurface, modifier = Modifier.weight(1f))
            Switch(
                checked = appSettings.isDarkTheme,
                onCheckedChange = { onAppSettingsChanged(appSettings.copy(isDarkTheme = it)) },
                colors = SwitchDefaults.colors(checkedThumbColor = Primary, checkedTrackColor = PrimaryVariant)
            )
        }

        Spacer(Modifier.height(12.dp))

        Text("Font Size: ${fontSize.toInt()}sp", style = MaterialTheme.typography.bodyMedium, color = OnSurface)
        Slider(value = fontSize, onValueChange = onFontSizeChanged, valueRange = 12f..20f, steps = 7,
            modifier = Modifier.fillMaxWidth(),
            colors = SliderDefaults.colors(thumbColor = Primary, activeTrackColor = Primary))

        Spacer(Modifier.height(8.dp))

        Text("Split: ${(paneRatio * 100).toInt()}/${(100 - paneRatio * 100).toInt()}", style = MaterialTheme.typography.bodyMedium, color = OnSurface)
        Slider(value = paneRatio, onValueChange = onPaneRatioChanged, valueRange = 0.2f..0.6f, steps = 7,
            modifier = Modifier.fillMaxWidth(),
            colors = SliderDefaults.colors(thumbColor = Primary, activeTrackColor = Primary))

        DividerSection()

        // ── SSH Connection ─────────────────────────────────────────────
        SectionHeader("SSH Connection")
        Spacer(Modifier.height(8.dp))

        OutlinedTextField(
            value = connectionConfig.host, onValueChange = { onConnectionConfigChanged(connectionConfig.copy(host = it)) },
            label = { Text("Hostname") }, singleLine = true, modifier = Modifier.fillMaxWidth(), colors = fieldColors())

        Spacer(Modifier.height(8.dp))

        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(
                value = connectionConfig.port.toString(),
                onValueChange = { val p = it.toIntOrNull() ?: connectionConfig.port; onConnectionConfigChanged(connectionConfig.copy(port = p)) },
                label = { Text("Port") }, singleLine = true, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.width(100.dp), colors = fieldColors())
            OutlinedTextField(
                value = connectionConfig.user, onValueChange = { onConnectionConfigChanged(connectionConfig.copy(user = it)) },
                label = { Text("Username") }, singleLine = true, modifier = Modifier.weight(1f), colors = fieldColors())
        }

        Spacer(Modifier.height(8.dp))

        OutlinedTextField(
            value = connectionConfig.password ?: "", onValueChange = { onConnectionConfigChanged(connectionConfig.copy(password = it.ifBlank { null })) },
            label = { Text("Password (optional)") }, singleLine = true,
            visualTransformation = if (showPassword) VisualTransformation.None else PasswordVisualTransformation(),
            trailingIcon = { IconButton(onClick = { showPassword = !showPassword }) { Icon(if (showPassword) Icons.Filled.VisibilityOff else Icons.Filled.Visibility, null, tint = OnSurface.copy(alpha = 0.6f)) } },
            modifier = Modifier.fillMaxWidth(), colors = fieldColors())

        DividerSection()

        // ── Daemon Control ─────────────────────────────────────────────
        SectionHeader("Daemon Control")
        Spacer(Modifier.height(8.dp))

        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = onDetach, modifier = Modifier.weight(1f), enabled = daemonConnected) { Text("Detach") }
            OutlinedButton(onClick = onAttach, modifier = Modifier.weight(1f), enabled = daemonConnected) { Text("Attach") }
        }
        Spacer(Modifier.height(8.dp))
        OutlinedButton(onClick = onCheckpoint, modifier = Modifier.fillMaxWidth(), enabled = daemonConnected) {
            Icon(Icons.Filled.Save, null, modifier = Modifier.size(18.dp))
            Spacer(Modifier.width(8.dp))
            Text("Save Checkpoint")
        }

        DividerSection()

        // ── Security ───────────────────────────────────────────────────
        SectionHeader("Security")
        Spacer(Modifier.height(8.dp))
        OutlinedButton(onClick = onImportKey, modifier = Modifier.fillMaxWidth()) { Text("Import SSH Private Key") }

        DividerSection()

        // ── Data ───────────────────────────────────────────────────────
        SectionHeader("Data")
        Spacer(Modifier.height(8.dp))
        OutlinedButton(onClick = onClearData, modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.outlinedButtonColors(contentColor = StatusRed)) {
            Text("Clear All Local Data")
        }

        DividerSection()

        // ── About ──────────────────────────────────────────────────────
        SectionHeader("About")
        Spacer(Modifier.height(4.dp))
        Text("DeepSeek TUI v0.8.26", style = MaterialTheme.typography.bodyMedium, color = OnSurface)
        Text("Android Wrapper · com.deepseek.tui", style = MaterialTheme.typography.bodySmall, color = OnSurface.copy(alpha = 0.5f))
        Text("Built 2026-05-12", style = MaterialTheme.typography.bodySmall, color = OnSurface.copy(alpha = 0.4f))
    }
}

@Composable
private fun SectionHeader(title: String) {
    Text(title, style = MaterialTheme.typography.titleMedium, color = OnSurface, fontWeight = FontWeight.SemiBold)
}

@Composable
private fun DividerSection() {
    Spacer(Modifier.height(20.dp))
    HorizontalDivider(color = Divider)
    Spacer(Modifier.height(20.dp))
}

@Composable
private fun fieldColors() = OutlinedTextFieldDefaults.colors(
    focusedBorderColor = Primary, unfocusedBorderColor = Divider,
    focusedContainerColor = SurfaceVariant, unfocusedContainerColor = SurfaceVariant,
    cursorColor = Primary, focusedLabelColor = Primary
)
