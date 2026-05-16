use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub enum DiffLine {
    Header(String),
    Hunk(String),
    Added(String),
    Removed(String),
    Context(String),
    Empty,
}

pub struct DiffPane {
    pub lines: Vec<DiffLine>,
    pub scroll_offset: usize,
    pub file_count: usize,
    pub added_lines: usize,
    pub removed_lines: usize,
}

impl DiffPane {
    pub fn new() -> Self {
        Self { lines: Vec::new(), scroll_offset: 0, file_count: 0, added_lines: 0, removed_lines: 0 }
    }

    pub fn load_diff(&mut self, raw_diff: &str) {
        self.lines.clear();
        self.file_count = 0;
        self.added_lines = 0;
        self.removed_lines = 0;
        for line in raw_diff.lines() {
            if line.starts_with("diff --git") {
                self.file_count += 1;
                self.lines.push(DiffLine::Header(line.to_string()));
            } else if line.starts_with("@@") {
                self.lines.push(DiffLine::Hunk(line.to_string()));
            } else if line.starts_with('+') && !line.starts_with("+++") {
                self.added_lines += 1;
                self.lines.push(DiffLine::Added(line.to_string()));
            } else if line.starts_with('-') && !line.starts_with("---") {
                self.removed_lines += 1;
                self.lines.push(DiffLine::Removed(line.to_string()));
            } else if line.starts_with("+++") || line.starts_with("---") {
                self.lines.push(DiffLine::Header(line.to_string()));
            } else if line.is_empty() {
                self.lines.push(DiffLine::Empty);
            } else {
                self.lines.push(DiffLine::Context(line.to_string()));
            }
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let stats = format!(" {} files · +{} -{} ", self.file_count, self.added_lines, self.removed_lines);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title_top(Line::from(" Diff ").left_aligned())
            .title_bottom(Line::from(stats).right_aligned().fg(Color::DarkGray));
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.height < 2 || self.lines.is_empty() {
            f.render_widget(Paragraph::new(Span::styled("No changes", Style::default().fg(Color::DarkGray).italic())), inner);
            return;
        }
        let max_lines = inner.height as usize;
        let start = self.lines.len().saturating_sub(max_lines + self.scroll_offset).min(self.lines.len().saturating_sub(1));
        let end = (start + max_lines).min(self.lines.len());
        let styled: Vec<Line> = self.lines[start..end].iter().map(|dl| match dl {
            DiffLine::Header(t) => Line::from(Span::styled(t.as_str(), Style::default().fg(Color::Yellow).bold())),
            DiffLine::Hunk(t) => Line::from(Span::styled(t.as_str(), Style::default().fg(Color::Cyan))),
            DiffLine::Added(t) => Line::from(Span::styled(t.as_str(), Style::default().fg(Color::Green))),
            DiffLine::Removed(t) => Line::from(Span::styled(t.as_str(), Style::default().fg(Color::Red))),
            DiffLine::Context(t) => Line::from(Span::raw(t.as_str())),
            DiffLine::Empty => Line::from(""),
        }).collect();
        f.render_widget(Paragraph::new(Text::from(styled)), inner);
    }
}

impl Default for DiffPane {
    fn default() -> Self { Self::new() }
}
