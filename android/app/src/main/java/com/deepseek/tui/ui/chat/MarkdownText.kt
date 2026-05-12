package com.deepseek.tui.ui.chat

import android.widget.TextView
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import com.deepseek.tui.ui.theme.*
import io.noties.markwon.Markwon
import io.noties.markwon.SoftBreakAddsNewLinePlugin
import io.noties.markwon.ext.strikethrough.StrikethroughPlugin
import io.noties.markwon.ext.tables.TablePlugin
import io.noties.markwon.linkify.LinkifyPlugin

/**
 * Renders markdown text using Markwon inside an AndroidView.
 *
 * Styles code blocks, links, lists, and blockquotes to match the
 * app's dark theme color palette.
 */
@Composable
fun MarkdownText(
    markdown: String,
    modifier: Modifier = Modifier,
    isUser: Boolean = false,
    maxWidth: Int = 320
) {
    val context = LocalContext.current

    // User messages: render as plain text (no markdown parsing)
    if (isUser) {
        AndroidView(
            factory = { ctx ->
                TextView(ctx).apply {
                    text = markdown
                    setTextColor(OnPrimary.toArgb())
                    textSize = 14f
                    setLineSpacing(4f, 1f)
                    setPadding(0, 0, 0, 0)
                }
            },
            modifier = modifier,
            update = { it.text = markdown }
        )
        return
    }

    // AI messages: full markdown rendering
    val markwon = remember {
        Markwon.builder(context)
            .usePlugin(SoftBreakAddsNewLinePlugin.create())
            .usePlugin(LinkifyPlugin.create())
            .usePlugin(StrikethroughPlugin.create())
            .usePlugin(TablePlugin.create(context))
            .build()
    }

    AndroidView(
        factory = { ctx ->
            TextView(ctx).apply {
                // Base text styling
                setTextColor(OnSurface.toArgb())
                textSize = 14f
                setLineSpacing(6f, 1.2f)

                // Link handling
                autoLinkMask = 0 // let Markwon handle links
                linksClickable = true

                // Set the markdown
                markwon.setMarkdown(this, markdown)
            }
        },
        modifier = modifier,
        update = { textView ->
            markwon.setMarkdown(textView, markdown)
        }
    )
}
