use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// A task item in the task list.
#[derive(Debug, Clone)]
pub struct TaskItem {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub category: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

impl TaskStatus {
    fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "○",
            TaskStatus::InProgress => "◉",
            TaskStatus::Completed => "✓",
            TaskStatus::Blocked => "⊘",
        }
    }
    fn color(&self) -> Color {
        match self {
            TaskStatus::Pending => Color::DarkGray,
            TaskStatus::InProgress => Color::Yellow,
            TaskStatus::Completed => Color::Green,
            TaskStatus::Blocked => Color::Red,
        }
    }
}

/// Displays a task checklist, integrated with the GSD planning system.
pub struct TasksPane {
    pub tasks: Vec<TaskItem>,
    pub scroll_offset: usize,
    pub selected: Option<usize>,
}

impl TasksPane {
    pub fn new() -> Self {
        Self { tasks: Vec::new(), scroll_offset: 0, selected: None }
    }

    /// Load tasks from a list of (title, status_str) pairs.
    pub fn load_tasks(&mut self, items: Vec<(String, String, String)>) {
        self.tasks = items.into_iter().enumerate().map(|(i, (title, status, cat))| {
            let status = match status.as_str() {
                "in_progress" | "active" | "doing" => TaskStatus::InProgress,
                "completed" | "done" => TaskStatus::Completed,
                "blocked" | "stuck" => TaskStatus::Blocked,
                _ => TaskStatus::Pending,
            };
            TaskItem { id: format!("T-{:03}", i + 1), title, status, category: cat }
        }).collect();
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let pending = self.tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
        let active = self.tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
        let done = self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let stats = format!(" {} total · {} active · {} done · {} pending ", self.tasks.len(), active, done, pending);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title_top(Line::from(" Tasks ").left_aligned())
            .title_bottom(Line::from(stats).right_aligned().fg(Color::DarkGray));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 2 || self.tasks.is_empty() {
            f.render_widget(Paragraph::new(Span::styled("No tasks", Style::default().fg(Color::DarkGray).italic())), inner);
            return;
        }

        let max = inner.height as usize;
        let start = self.tasks.len().saturating_sub(max + self.scroll_offset).min(self.tasks.len().saturating_sub(1));
        let end = (start + max).min(self.tasks.len());

        let items: Vec<ListItem> = self.tasks[start..end].iter().enumerate().map(|(i, task)| {
            let global_i = start + i;
            let is_selected = self.selected == Some(global_i);
            let icon = task.status.icon();
            let color = task.status.color();
            let prefix = if is_selected {
                Span::styled(format!(" {} {} ", icon, task.id), Style::default().fg(color).bg(Color::DarkGray).bold())
            } else {
                Span::styled(format!(" {} {} ", icon, task.id), Style::default().fg(color))
            };
            let title = Span::styled(format!(" {}", task.title), Style::default().fg(Color::White));
            let cat = Span::styled(format!(" [{}]", task.category), Style::default().fg(Color::DarkGray));
            ListItem::new(Line::from(vec![prefix, title, cat]))
        }).collect();

        f.render_widget(List::new(items), inner);
    }
}

impl Default for TasksPane {
    fn default() -> Self { Self::new() }
}
