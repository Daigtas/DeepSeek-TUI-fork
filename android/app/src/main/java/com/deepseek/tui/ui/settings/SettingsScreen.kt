package com.deepseek.tui.ui.settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.deepseek.tui.ui.theme.*

@Composable
fun SettingsScreen(
    fontSize: Float,
    paneRatio: Float,
    onFontSizeChanged: (Float) -> Unit,
    onPaneRatioChanged: (Float) -> Unit,
    onImportKey: () -> Unit,
    onClearData: () -> Unit,
    modifier: Modifier = Modifier
) {
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

        // Appearance section
        Text(
            text = "Appearance",
            style = MaterialTheme.typography.titleMedium,
            color = OnSurface,
            fontWeight = FontWeight.SemiBold
        )

        Spacer(modifier = Modifier.height(12.dp))

        // Font size slider
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

        // Pane ratio slider
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

        // Security section
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

        // Data section
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
