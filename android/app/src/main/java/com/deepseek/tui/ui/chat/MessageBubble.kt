package com.deepseek.tui.ui.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.deepseek.tui.data.db.MessageEntity
import com.deepseek.tui.ui.theme.*

@Composable
fun MessageBubble(
    message: MessageEntity,
    modifier: Modifier = Modifier
) {
    val isUser = message.role == "user"
    val bubbleColor = if (isUser) UserBubble else AiBubble
    val alignment = if (isUser) Alignment.End else Alignment.Start
    val shape = RoundedCornerShape(
        topStart = 16.dp,
        topEnd = 16.dp,
        bottomStart = if (isUser) 16.dp else 4.dp,
        bottomEnd = if (isUser) 4.dp else 16.dp
    )

    val hasContent = message.content.isNotBlank()
    val hasStreamingContent = message.isStreaming && message.content.isNotBlank()

    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 4.dp),
        horizontalAlignment = alignment
    ) {
        // Role label
        Text(
            text = if (isUser) "You" else (message.agentRole ?: "DeepSeek"),
            style = MaterialTheme.typography.labelSmall,
            color = OnSurface.copy(alpha = 0.5f),
            modifier = Modifier.padding(horizontal = 4.dp, vertical = 2.dp)
        )

        // Bubble
        Column(
            modifier = Modifier
                .widthIn(max = 320.dp)
                .clip(shape)
                .background(bubbleColor)
                .padding(12.dp)
        ) {
            // Thinking tokens (if any)
            if (!message.thinkingTokens.isNullOrBlank()) {
                Text(
                    text = message.thinkingTokens,
                    style = MaterialTheme.typography.bodySmall,
                    color = OnSurface.copy(alpha = 0.5f),
                    fontStyle = androidx.compose.ui.text.font.FontStyle.Italic
                )
                Spacer(modifier = Modifier.height(6.dp))
            }

            // Main content — markdown for AI, plain text for user
            if (hasContent || hasStreamingContent) {
                MarkdownText(
                    markdown = message.content,
                    isUser = isUser,
                    modifier = Modifier.fillMaxWidth()
                )
            }

            if (message.isStreaming) {
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "▌",
                    style = MaterialTheme.typography.bodyMedium,
                    color = Primary
                )
            }
        }
    }
}
