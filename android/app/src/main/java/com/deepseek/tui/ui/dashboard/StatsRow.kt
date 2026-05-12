package com.deepseek.tui.ui.dashboard

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.deepseek.tui.ui.theme.*

@Composable
fun StatsRow(
    activeTaskCount: Long,
    connectedClientCount: Int,
    daemonUptime: String,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        StatCard(
            label = "Tasks",
            value = activeTaskCount.toString(),
            modifier = Modifier.weight(1f)
        )
        StatCard(
            label = "Clients",
            value = connectedClientCount.toString(),
            modifier = Modifier.weight(1f)
        )
        StatCard(
            label = "Uptime",
            value = formatUptime(daemonUptime),
            modifier = Modifier.weight(1f)
        )
    }
}

@Composable
private fun StatCard(
    label: String,
    value: String,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .clip(RoundedCornerShape(8.dp))
            .background(SurfaceVariant)
            .padding(horizontal = 10.dp, vertical = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(
            text = value,
            style = MaterialTheme.typography.titleLarge,
            color = Primary,
            fontWeight = FontWeight.Bold
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = OnSurface.copy(alpha = 0.5f)
        )
    }
}

private fun formatUptime(rfc3339: String): String {
    if (rfc3339.isBlank()) return "—"
    return try {
        val instant = java.time.Instant.parse(rfc3339)
        val duration = java.time.Duration.between(instant, java.time.Instant.now())
        val hours = duration.toHours()
        val minutes = duration.toMinutes() % 60
        if (hours > 0) "${hours}h ${minutes}m"
        else "${minutes}m"
    } catch (_: Exception) {
        "—"
    }
}
