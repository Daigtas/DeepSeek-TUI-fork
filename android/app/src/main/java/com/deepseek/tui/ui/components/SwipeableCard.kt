package com.deepseek.tui.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.deepseek.tui.ui.theme.StatusGreen
import com.deepseek.tui.ui.theme.StatusRed
import kotlin.math.roundToInt

/**
 * A swipeable card that reveals action labels when dragged.
 *
 * Swipe right → shows "Copy" (green background)
 * Swipe left → shows "Delete" or "Retry" (red background)
 */
@Composable
fun SwipeableCard(
    onSwipeRight: (() -> Unit)? = null,
    onSwipeLeft: (() -> Unit)? = null,
    rightActionLabel: String = "Copy",
    leftActionLabel: String = "Delete",
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    var offsetX by remember { mutableFloatStateOf(0f) }
    val swipeThreshold = 100f

    val bgColor by animateColorAsState(
        when {
            offsetX > swipeThreshold -> StatusGreen.copy(alpha = 0.15f)
            offsetX < -swipeThreshold -> StatusRed.copy(alpha = 0.15f)
            else -> Color.Transparent
        },
        label = "swipeBg"
    )

    Box(modifier = modifier.fillMaxWidth()) {
        // Action labels behind the content
        if (offsetX > 30f && onSwipeRight != null) {
            Text(
                text = rightActionLabel,
                style = MaterialTheme.typography.labelLarge,
                color = StatusGreen,
                modifier = Modifier
                    .align(Alignment.CenterStart)
                    .offset(x = 16.dp)
            )
        }
        if (offsetX < -30f && onSwipeLeft != null) {
            Text(
                text = leftActionLabel,
                style = MaterialTheme.typography.labelLarge,
                color = StatusRed,
                modifier = Modifier
                    .align(Alignment.CenterEnd)
                    .offset(x = (-16).dp)
            )
        }

        // Content (slides with drag)
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .background(bgColor)
                .offset { IntOffset(offsetX.roundToInt(), 0) }
                .pointerInput(Unit) {
                    detectHorizontalDragGestures(
                        onDragEnd = {
                            if (offsetX > swipeThreshold) onSwipeRight?.invoke()
                            else if (offsetX < -swipeThreshold) onSwipeLeft?.invoke()
                            offsetX = 0f
                        },
                        onHorizontalDrag = { _, dragAmount ->
                            offsetX = (offsetX + dragAmount).coerceIn(-200f, 200f)
                        }
                    )
                }
        ) {
            content()
        }
    }
}
