package com.deepseek.tui.ui.settings

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
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
    modifier: Modifier = Modifier
) {
    var showPassword by remember { mutableStateOf(false) }

    Column(modifier = modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
        Text("Settings", style = MaterialTheme.typography.headlineMedium, color = Primary, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(24.dp))

        SectionHeader("SSH Connection")
        Spacer(Modifier.height(12.dp))

        OutlinedTextField(value = connectionConfig.host, onValueChange = { onConnectionConfigChanged(connectionConfig.copy(host = it)) },
            label = { Text("Hostname") }, singleLine = true, modifier = Modifier.fillMaxWidth(), colors = fieldColors())
        Spacer(Modifier.height(8.dp))

        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(value = connectionConfig.port.toString(),
                onValueChange = { val p = it.toIntOrNull() ?: connectionConfig.port; onConnectionConfigChanged(connectionConfig.copy(port = p)) },
                label = { Text("Port") }, singleLine = true, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.width(100.dp), colors = fieldColors())
            OutlinedTextField(value = connectionConfig.user, onValueChange = { onConnectionConfigChanged(connectionConfig.copy(user = it)) },
                label = { Text("Username") }, singleLine = true, modifier = Modifier.weight(1f), colors = fieldColors())
        }
        Spacer(Modifier.height(8.dp))

        OutlinedTextField(value = connectionConfig.password ?: "", onValueChange = { onConnectionConfigChanged(connectionConfig.copy(password = it.ifBlank { null })) },
            label = { Text("Password (optional)") }, singleLine = true,
            visualTransformation = if (showPassword) VisualTransformation.None else PasswordVisualTransformation(),
            trailingIcon = { IconButton(onClick = { showPassword = !showPassword }) { Icon(if (showPassword) Icons.Filled.VisibilityOff else Icons.Filled.Visibility, null, tint = OnSurface.copy(alpha = 0.6f)) } },
            modifier = Modifier.fillMaxWidth(), colors = fieldColors())

        DividerSection()

        SectionHeader("Appearance")
        Spacer(Modifier.height(12.dp))
        Text("Font Size: ${fontSize.toInt()}sp", style = MaterialTheme.typography.bodyMedium, color = OnSurface)
        Slider(fontSize, onFontSizeChanged, valueRange = 12f..20f, steps = 7, modifier = Modifier.fillMaxWidth(),
            colors = SliderDefaults.colors(thumbColor = Primary, activeTrackColor = Primary))
        Spacer(Modifier.height(16.dp))
        Text("Split: ${(paneRatio * 100).toInt()}/${(100 - paneRatio * 100).toInt()}", style = MaterialTheme.typography.bodyMedium, color = OnSurface)
        Slider(paneRatio, onPaneRatioChanged, valueRange = 0.2f..0.6f, steps = 7, modifier = Modifier.fillMaxWidth(),
            colors = SliderDefaults.colors(thumbColor = Primary, activeTrackColor = Primary))

        DividerSection()

        SectionHeader("Security")
        Spacer(Modifier.height(12.dp))
        OutlinedButton(onClick = onImportKey, modifier = Modifier.fillMaxWidth()) { Text("Import SSH Private Key") }

        DividerSection()

        SectionHeader("Data")
        Spacer(Modifier.height(12.dp))
        OutlinedButton(onClick = onClearData, modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.outlinedButtonColors(contentColor = StatusRed)) { Text("Clear All Local Data") }
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
