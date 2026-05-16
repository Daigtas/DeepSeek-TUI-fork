use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// A single message in the chat pane.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
    Error,
}

/// The chat pane: scrollable message history with input bar at the bottom.
pub struct ChatPane {
    pub messages: Vec<Message>,
    pub input: String,
    pub streaming_text: String,
    /// Scroll offset from the bottom (0 = bottom, higher = scrolled up).
    pub scroll_offset: usize,
}

impl ChatPane {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            streaming_text: String::new(),
            scroll_offset: 0,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, streaming: bool) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title_top(Line::from(" Chat ").left_aligned());

        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 3 {
            return;
        }

        // Calculate how many lines we can show
        let available_lines = inner.height as usize;
        let input_lines = std::cmp::max(1, (self.input.len() + inner.width as usize - 1) / inner.width as usize);
        let msg_area_height = available_lines.saturating_sub(input_lines + 2);

        // Build text from messages
        let mut lines: Vec<Line> = Vec::new();
        let mut line_count = 0;

        for msg in self.messages.iter().rev() {
            let (role_prefix, color) = match msg.role {
                MessageRole::User => ("You", Color::Cyan),
                MessageRole::Assistant => ("DS", Color::Green),
                MessageRole::Tool => ("🔧", Color::Yellow),
                MessageRole::System => ("⚙", Color::DarkGray),
                MessageRole::Error => ("✗", Color::Red),
            };

            let prefix = Span::styled(
                format!("{} ", role_prefix),
                Style::default().fg(color).bold(),
            );

            // Word-wrap the message content
            for content_line in msg.content.lines() {
                if content_line.is_empty() {
                    lines.push(Line::from(""));
                    line_count += 1;
                    continue;
                }
                // Simple character-based wrapping
                let chars_per_line = inner.width.saturating_sub(4) as usize;
                let mut start = 0;
                while start < content_line.len() {
                    let end = (start + chars_per_line).min(content_line.len());
                    let slice = &content_line[start..end];
                    lines.push(Line::from(vec![prefix.clone(), Span::raw(slice)]));
                    line_count += 1;
                    start = end;
                }
            }
            lines.push(Line::from(""));
            line_count += 1;

            if line_count >= msg_area_height + self.scroll_offset {
                break;
            }
        }

        // Truncate from top based on scroll offset
        let visible_start = if lines.len() > msg_area_height {
            lines.len().saturating_sub(msg_area_height + self.scroll_offset)
        } else {
            0
        };
        let visible_lines: Vec<Line> = lines.into_iter().skip(visible_start).take(msg_area_height).collect();

        // Streaming text indicator
        let mut display_lines = visible_lines;
        if streaming && !self.streaming_text.is_empty() {
            let stream_line = Line::from(vec![
                Span::styled("DS ", Style::default().fg(Color::Green).bold()),
                Span::styled(&self.streaming_text, Style::default().italic()),
                Span::styled(" ▌", Style::default().fg(Color::Green)),
            ]);
            display_lines.push(stream_line);
        }

        let text = Text::from(display_lines);
        f.render_widget(
            Paragraph::new(text),
            Rect {
                height: msg_area_height as u16,
                ..inner
            },
        );

        // Input bar
        let input_area = Rect {
            y: inner.y + msg_area_height as u16 + 1,
            height: input_lines as u16 + 1,
            ..inner
        };

        let prompt = if streaming {
            Span::styled("⏳ ", Style::default().fg(Color::Yellow))
        } else {
            Span::styled("❯ ", Style::default().fg(Color::Green))
        };

        let input_text = if self.input.is_empty() {
            Span::styled("type a message…", Style::default().fg(Color::DarkGray).italic())
        } else {
            Span::raw(&self.input)
        };

        let input_line = Line::from(vec![prompt, input_text]);
        let input_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));

        f.render_widget(
            Paragraph::new(input_line).block(input_block),
            input_area,
        );
    }
}

impl Default for ChatPane {
    fn default() -> Self {
        Self::new()
    }
}
