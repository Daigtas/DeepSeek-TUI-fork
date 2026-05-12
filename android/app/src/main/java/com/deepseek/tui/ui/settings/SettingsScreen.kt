package com.deepseek.tui.ui.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
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

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp)
    ) {
        Text(
            text = "Settings",
            style = MaterialTheme.typography.headlineMedium,
            color = Primary,
            fontWeight = FontWeight.Bold
        )

        Spacer(modifier = Modifier.height(24.dp))

        // ── SSH Connection section ──────────────────────────────────────
        Text(
            text = "SSH Connection",
            style = MaterialTheme.typography.titleMedium,
            color = OnSurface,
            fontWeight = FontWeight.SemiBold
        )

        Spacer(modifier = Modifier.height(12.dp))

        OutlinedTextField(
            value = connectionConfig.host,
            onValueChange = { onConnectionConfigChanged(connectionConfig.copy(host = it)) },
            label = { Text("Hostname") },
            placeholder = { Text("boottify.com") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = Primary,
                unfocusedBorderColor = Divider,
                focusedContainerColor = SurfaceVariant,
                unfocusedContainerColor = SurfaceVariant,
                cursorColor = Primary
            )
        )

        Spacer(modifier = Modifier.height(8.dp))

        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(
                value = connectionConfig.port.toString(),
                onValueChange = {
                    val port = it.toIntOrNull() ?: connectionConfig.port
                    onConnectionConfigChanged(connectionConfig.copy(port = port))
                },
                label = { Text("Port") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.width(100.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = Primary,
                    unfocusedBorderColor = Divider,
                    focusedContainerColor = SurfaceVariant,
                    unfocusedContainerColor = SurfaceVariant,
                    cursorColor = Primary
                )
            )

            OutlinedTextField(
                value = connectionConfig.user,
                onValueChange = { onConnectionConfigChanged(connectionConfig.copy(user = it)) },
                label = { Text("Username") },
                placeholder = { Text("root") },
                singleLine = true,
                modifier = Modifier.weight(1f),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = Primary,
                    unfocusedBorderColor = Divider,
                    focusedContainerColor = SurfaceVariant,
                    unfocusedContainerColor = SurfaceVariant,
                    cursorColor = Primary
                )
            )
        }

        Spacer(modifier = Modifier.height(8.dp))

        OutlinedTextField(
            value = connectionConfig.password ?: "",
            onValueChange = {
                onConnectionConfigChanged(connectionConfig.copy(password = it.ifBlank { null }))
            },
            label = { Text("Password (optional)") },
            singleLine = true,
            visualTransformation = if (showPassword) VisualTransformation.None else PasswordVisualTransformation(),
            trailingIcon = {
                IconButton(onClick = { showPassword = !showPassword }) {
                    Icon(
                        imageVector = if (showPassword) Icons.Filled.VisibilityOff else Icons.Filled.Visibility,
                        contentDescription = if (showPassword) "Hide password" else "Show password",
                        tint = OnSurface.copy(alpha = 0.6f)
                    )
                }
            },
            modifier = Modifier.fillMaxWidth(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = Primary,
                unfocusedBorderColor = Divider,
                focusedContainerColor = SurfaceVariant,
                unfocusedContainerColor = SurfaceVariant,
                cursorColor = Primary
            )
        )

        Spacer(modifier = Modifier.height(24.dp))

        HorizontalDivider(color = Divider)

        Spacer(modifier = Modifier.height(24.dp))

        // ── Appearance section ──────────────────────────────────────────
        Text(
            text = "Appearance",
            style = MaterialTheme.typography.titleMedium,
            color = OnSurface,
            fontWeight = FontWeight.SemiBold
        )

        Spacer(modifier = Modifier.height(12.dp))

        Text(
            text = "Font Size: ${fontSize.toInt()}sp",
            style = MaterialTheme.typography.bodyMedium,
            color = OnSurface
        )
        Slider(
            value = fontSize,
            onValueChange = onFontSizeChanged,
            valueRange = 12f..20f,
            steps = 7,
            modifier = Modifier.fillMaxWidth(),
            colors = SliderDefaults.colors(
                thumbColor = Primary,
                activeTrackColor = Primary
            )
        )

        Spacer(modifier = Modifier.height(16.dp))

        Text(
            text = "Dashboard/Chat Split: ${(paneRatio * 100).toInt()}/${(100 - paneRatio * 100).toInt()}",
            style = MaterialTheme.typography.bodyMedium,
            color = OnSurface
        )
        Slider(
            value = paneRatio,
            onValueChange = onPaneRatioChanged,
            valueRange = 0.2f..0.6f,
            steps = 7,
            modifier = Modifier.fillMaxWidth(),
            colors = SliderDefaults.colors(
                thumbColor = Primary,
                activeTrackColor = Primary
            )
        )

        Spacer(modifier = Modifier.height(24.dp))

        HorizontalDivider(color = Divider)

        Spacer(modifier = Modifier.height(24.dp))

        // ── Security section ────────────────────────────────────────────
        Text(
            text = "Security",
            style = MaterialTheme.typography.titleMedium,
            color = OnSurface,
            fontWeight = FontWeight.SemiBold
        )

        Spacer(modifier = Modifier.height(12.dp))

        OutlinedButton(
            onClick = onImportKey,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Import SSH Private Key")
        }

        Spacer(modifier = Modifier.height(24.dp))

        HorizontalDivider(color = Divider)

        Spacer(modifier = Modifier.height(24.dp))

        // ── Data section ────────────────────────────────────────────────
        Text(
            text = "Data",
            style = MaterialTheme.typography.titleMedium,
            color = OnSurface,
            fontWeight = FontWeight.SemiBold
        )

        Spacer(modifier = Modifier.height(12.dp))

        OutlinedButton(
            onClick = onClearData,
            modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.outlinedButtonColors(contentColor = StatusRed)
        ) {
            Text("Clear All Local Data")
        }
    }
}
