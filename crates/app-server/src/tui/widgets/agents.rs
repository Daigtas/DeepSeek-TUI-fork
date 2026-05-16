use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

#[derive(Debug, Clone)]
pub struct AgentDisplay {
    pub id: String,
    pub role: String,
    pub name: String,
    pub status: AgentDisplayStatus,
    pub task: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentDisplayStatus {
    Idle,
    Working,
    Completed,
    Failed,
}

impl AgentDisplayStatus {
    fn icon(&self) -> &'static str {
        match self { Self::Idle => "○", Self::Working => "◉", Self::Completed => "✓", Self::Failed => "✗" }
    }
    fn color(&self) -> Color {
        match self { Self::Idle => Color::DarkGray, Self::Working => Color::Yellow, Self::Completed => Color::Green, Self::Failed => Color::Red }
    }
    fn label(&self) -> &'static str {
        match self { Self::Idle => "idle", Self::Working => "working", Self::Completed => "done", Self::Failed => "failed" }
    }
}

pub struct AgentsPane {
    pub agents: Vec<AgentDisplay>,
    pub scroll_offset: usize,
}

impl AgentsPane {
    pub fn new() -> Self { Self { agents: Vec::new(), scroll_offset: 0 } }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let active = self.agents.iter().filter(|a| a.status == AgentDisplayStatus::Working).count();
        let done = self.agents.iter().filter(|a| a.status == AgentDisplayStatus::Completed).count();
        let failed = self.agents.iter().filter(|a| a.status == AgentDisplayStatus::Failed).count();
        let stats = format!(" {} agents · {} active · {} done · {} failed ", self.agents.len(), active, done, failed);

        let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray))
            .title_top(Line::from(" Agents ").left_aligned())
            .title_bottom(Line::from(stats).right_aligned().fg(Color::DarkGray));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 2 || self.agents.is_empty() {
            f.render_widget(Paragraph::new(Span::styled("No active agents — use /swarm to spawn", Style::default().fg(Color::DarkGray).italic())), inner);
            return;
        }

        let max = inner.height as usize;
        let start = self.agents.len().saturating_sub(max + self.scroll_offset).min(self.agents.len().saturating_sub(1));
        let end = (start + max).min(self.agents.len());

        let items: Vec<ListItem> = self.agents[start..end].iter().map(|a| {
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} {} ", a.status.icon(), a.status.label()), Style::default().fg(a.status.color())),
                Span::styled(format!(" {}", a.role), Style::default().fg(Color::Cyan)),
                Span::styled(format!(" {}", a.name), Style::default().fg(Color::White).bold()),
                a.task.as_ref().map(|t| Span::styled(format!(" — {}", t), Style::default().fg(Color::DarkGray).italic())).unwrap_or(Span::raw("")),
                a.model.as_ref().map(|m| Span::styled(format!(" [{}]", m), Style::default().fg(Color::DarkGray))).unwrap_or(Span::raw("")),
                Span::styled(format!("  {}", &a.id[..a.id.len().min(8)]), Style::default().fg(Color::DarkGray)),
            ]))
        }).collect();
        f.render_widget(List::new(items), inner);
    }
}

impl Default for AgentsPane {
    fn default() -> Self { Self::new() }
}
