package com.deepseek.tui.ui.navigation

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.ui.unit.dp
import com.deepseek.tui.ui.theme.*

data class DrawerItem(
    val id: String,
    val label: String,
    val icon: @Composable () -> Unit
)

@Composable
fun NavigationDrawer(
    selectedItemId: String,
    hasSshKey: Boolean,
    keyFingerprint: String?,
    onItemSelected: (String) -> Unit,
    onImportKey: () -> Unit,
    onClose: () -> Unit,
    modifier: Modifier = Modifier
) {
    val items = listOf(
        DrawerItem("chat", "Chat") { Icon(Icons.Filled.Chat, null, modifier = Modifier.size(24.dp)) },
        DrawerItem("dashboard", "Dashboard") { Icon(Icons.Filled.Dashboard, null, modifier = Modifier.size(24.dp)) },
        DrawerItem("swarm", "Swarm") { Icon(Icons.Filled.Group, null, modifier = Modifier.size(24.dp)) },
        DrawerItem("hive", "Hive") { Icon(Icons.Filled.Storage, null, modifier = Modifier.size(24.dp)) },
        DrawerItem("sessions", "Sessions") { Icon(Icons.Filled.History, null, modifier = Modifier.size(24.dp)) },
        DrawerItem("settings", "Settings") { Icon(Icons.Filled.Settings, null, modifier = Modifier.size(24.dp)) },
    )

    ModalDrawerSheet(
        modifier = modifier
            .width(280.dp)
            .background(Background),
        drawerContainerColor = Background,
        drawerContentColor = OnSurface
    ) {
        // Header
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
        ) {
            Text(
                text = "DeepSeek TUI",
                style = MaterialTheme.typography.headlineMedium,
                color = Primary,
                fontWeight = FontWeight.Bold
            )
            Text(
                text = "v0.8.26 · Android",
                style = MaterialTheme.typography.bodySmall,
                color = OnSurface.copy(alpha = 0.5f)
            )
        }

        HorizontalDivider(color = Divider)

        // SSH Key status
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = if (hasSshKey) Icons.Filled.VpnKey else Icons.Filled.VpnKeyOff,
                    contentDescription = null,
                    tint = if (hasSshKey) StatusGreen else StatusRed,
                    modifier = Modifier.size(20.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = if (hasSshKey) "SSH Key loaded" else "No SSH Key",
                    style = MaterialTheme.typography.labelLarge,
                    color = if (hasSshKey) StatusGreen else StatusRed
                )
            }

            if (keyFingerprint != null) {
                Text(
                    text = keyFingerprint,
                    style = MaterialTheme.typography.bodySmall,
                    color = OnSurface.copy(alpha = 0.4f),
                    modifier = Modifier.padding(start = 28.dp, top = 2.dp)
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            TextButton(
                onClick = onImportKey,
                modifier = Modifier.fillMaxWidth()
            ) {
                Icon(Icons.Filled.FileUpload, null, modifier = Modifier.size(18.dp))
                Spacer(modifier = Modifier.width(6.dp))
                Text(if (hasSshKey) "Change Key" else "Import SSH Key")
            }
        }

        HorizontalDivider(color = Divider)

        // Navigation items
        LazyColumn(modifier = Modifier.padding(vertical = 8.dp)) {
            items(items) { item ->
                val selected = item.id == selectedItemId

                NavigationDrawerItem(
                    icon = item.icon,
                    label = { Text(item.label) },
                    selected = selected,
                    onClick = {
                        onItemSelected(item.id)
                        onClose()
                    },
                    colors = NavigationDrawerItemDefaults.colors(
                        selectedContainerColor = SurfaceVariant,
                        selectedIconColor = Primary,
                        selectedTextColor = Primary,
                        unselectedIconColor = OnSurface.copy(alpha = 0.6f),
                        unselectedTextColor = OnSurface
                    ),
                    modifier = Modifier.padding(horizontal = 12.dp)
                )
            }
        }

        Spacer(modifier = Modifier.weight(1f))

        // Footer
        Text(
            text = "boottify.com · root",
            style = MaterialTheme.typography.bodySmall,
            color = OnSurface.copy(alpha = 0.3f),
            modifier = Modifier.padding(16.dp)
        )
    }
}
