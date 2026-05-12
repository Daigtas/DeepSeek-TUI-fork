package com.deepseek.tui.ui.chat

import androidx.compose.foundation.layout.*
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.deepseek.tui.data.db.MessageEntity
import com.deepseek.tui.ui.theme.OnSurface
import com.deepseek.tui.viewmodel.ChatUiState

@Composable
fun ChatScreen(
    chatState: ChatUiState,
    onInputChanged: (String) -> Unit,
    onSend: () -> Unit,
    onNewConversation: () -> Unit,
    onAttachFile: () -> Unit = {},
    onRetryMessage: (MessageEntity) -> Unit = {},
    onEditMessage: (MessageEntity) -> Unit = {},
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxSize()) {
        // Error banner
        if (chatState.errorMessage != null) {
            Text(
                text = chatState.errorMessage,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp)
            )
        }

        // Message list
        MessageList(
            messages = chatState.messages,
            onRetryMessage = onRetryMessage,
            onEditMessage = onEditMessage,
            modifier = Modifier.weight(1f)
        )

        // Input bar
        ChatInputBar(
            inputText = chatState.inputText,
            isSending = chatState.isSending,
            onInputChanged = onInputChanged,
            onSend = onSend,
            onNewConversation = onNewConversation,
            onAttachFile = onAttachFile
        )
    }
}
