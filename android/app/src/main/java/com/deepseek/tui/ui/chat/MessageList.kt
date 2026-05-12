package com.deepseek.tui.ui.chat

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import com.deepseek.tui.data.db.MessageEntity
import com.deepseek.tui.ui.components.SwipeableCard
import kotlinx.coroutines.launch

@Composable
fun MessageList(
    messages: List<MessageEntity>,
    onRetryMessage: (MessageEntity) -> Unit = {},
    onEditMessage: (MessageEntity) -> Unit = {},
    modifier: Modifier = Modifier
) {
    val listState = rememberLazyListState()
    val clipboardManager = LocalClipboardManager.current
    val coroutineScope = rememberCoroutineScope()

    // Auto-scroll to bottom when new messages arrive
    LaunchedEffect(messages.size) {
        if (messages.isNotEmpty()) {
            listState.animateScrollToItem(messages.size - 1)
        }
    }

    LazyColumn(
        state = listState,
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(vertical = 8.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp)
    ) {
        items(messages, key = { it.id }) { message ->
            SwipeableCard(
                onSwipeRight = {
                    clipboardManager.setText(AnnotatedString(message.content))
                },
                onSwipeLeft = {
                    if (message.role == "assistant") {
                        onRetryMessage(message)
                    } else if (message.role == "user") {
                        onEditMessage(message)
                    }
                },
                rightActionLabel = "Copy",
                leftActionLabel = if (message.role == "assistant") "Retry" else "Edit"
            ) {
                MessageBubble(message = message)
            }
        }
    }
}
