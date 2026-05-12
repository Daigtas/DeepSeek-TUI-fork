package com.deepseek.tui.ui.dashboard

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.deepseek.tui.connection.SshTunnelManager
import com.deepseek.tui.ui.theme.*

@Composable
fun ConnectionBar(
    state: SshTunnelManager.TunnelState,
    host: String,
    latencyMs: Long?,
    onDisconnect: () -> Unit,
    modifier: Modifier = Modifier
) {
    val dotColor by animateColorAsState(
        when (state) {
            SshTunnelManager.TunnelState.CONNECTED -> StatusGreen
            SshTunnelManager.TunnelState.CONNECTING -> StatusYellow
            SshTunnelManager.TunnelState.ERROR -> StatusRed
            SshTunnelManager.TunnelState.DISCONNECTED -> StatusRed
            SshTunnelManager.TunnelState.HOST_KEY_UNKNOWN -> StatusYellow
        },
        label = "dotColor"
    )

    val statusText = when (state) {
        SshTunnelManager.TunnelState.CONNECTED -> "Connected"
        SshTunnelManager.TunnelState.CONNECTING -> "Connecting…"
        SshTunnelManager.TunnelState.ERROR -> "Error"
        SshTunnelManager.TunnelState.DISCONNECTED -> "Disconnected"
        SshTunnelManager.TunnelState.HOST_KEY_UNKNOWN -> "Verify Host Key"
    }

    Row(
        modifier = modifier
            .fillMaxWidth()
            .background(SurfaceVariant)
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        // Status dot
        Box(
            modifier = Modifier
                .size(10.dp)
                .clip(CircleShape)
                .background(dotColor)
        )

        Spacer(modifier = Modifier.width(8.dp))

        // Host + status
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = host,
                style = MaterialTheme.typography.labelLarge,
                color = OnSurface,
                fontWeight = FontWeight.SemiBold
            )
            Text(
                text = buildString {
                    append(statusText)
                    if (latencyMs != null) {
                        append(" · ${latencyMs}ms")
                    }
                },
                style = MaterialTheme.typography.bodySmall,
                color = if (state == SshTunnelManager.TunnelState.CONNECTED) StatusGreen
                       else OnSurface.copy(alpha = 0.6f)
            )
        }

        // Disconnect button (visible only when connected)
        if (state == SshTunnelManager.TunnelState.CONNECTED || state == SshTunnelManager.TunnelState.ERROR) {
            IconButton(onClick = onDisconnect) {
                Icon(
                    imageVector = Icons.Default.Close,
                    contentDescription = "Disconnect",
                    tint = OnSurface.copy(alpha = 0.6f),
                    modifier = Modifier.size(20.dp)
                )
            }
        }
    }
}
