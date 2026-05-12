package com.deepseek.tui.ui.dashboard

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import com.deepseek.tui.ui.theme.CodeStyle
import com.deepseek.tui.ui.theme.OnSurface
import com.deepseek.tui.ui.theme.SurfaceVariant
import com.deepseek.tui.ui.theme.StatusRed

@Composable
fun ProgressLog(
    lines: List<String>,
    modifier: Modifier = Modifier
) {
    val clipboardManager = LocalClipboardManager.current
    val listState = rememberLazyListState()

    Column(modifier = modifier.fillMaxWidth()) {
        Text(
            text = "Progress Log",
            style = MaterialTheme.typography.labelLarge,
            color = OnSurface,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp)
        )

        if (lines.isEmpty()) {
            Text(
                text = "No log entries yet",
                style = MaterialTheme.typography.bodySmall,
                color = OnSurface.copy(alpha = 0.4f),
                modifier = Modifier.padding(horizontal = 12.dp)
            )
        } else {
            LazyColumn(
                state = listState,
                modifier = Modifier
                    .fillMaxWidth()
                    .background(SurfaceVariant)
                    .padding(vertical = 4.dp),
                contentPadding = PaddingValues(horizontal = 12.dp)
            ) {
                itemsIndexed(lines) { _, line ->
                    var offsetX by remember { mutableFloatStateOf(0f) }

                    Text(
                        text = line,
                        style = CodeStyle,
                        color = OnSurface.copy(alpha = 0.8f),
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 2.dp)
                            .pointerInput(Unit) {
                                detectHorizontalDragGestures(
                                    onDragEnd = {
                                        if (offsetX > 80f) {
                                            clipboardManager.setText(AnnotatedString(line))
                                        }
                                        offsetX = 0f
                                    },
                                    onHorizontalDrag = { _, dragAmount ->
                                        offsetX = (offsetX + dragAmount).coerceIn(-200f, 200f)
                                    }
                                )
                            }
                    )
                }
            }
        }
    }
}
