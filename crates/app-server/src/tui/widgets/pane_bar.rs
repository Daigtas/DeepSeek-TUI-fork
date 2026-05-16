use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
    Frame,
};
use deepseek_tui_core::Pane;

/// A tab bar showing available panes with keyboard shortcuts.
pub struct PaneBar;

impl PaneBar {
    /// Render the pane tab bar at the top of the screen.
    pub fn render(f: &mut Frame, area: Rect, active_pane: Pane) {
        let panes = [
            (Pane::Chat, "1", "Chat"),
            (Pane::Diff, "2", "Diff"),
            (Pane::Tasks, "3", "Tasks"),
            (Pane::Agents, "4", "Agents"),
            (Pane::Jobs, "5", "Jobs"),
        ];

        let titles: Vec<Line> = panes.iter().map(|(pane, key, label)| {
            let base = format!(" {key}:{label} ");
            if *pane == active_pane {
                Line::from(Span::styled(
                    base,
                    Style::default().fg(Color::Black).bg(Color::Yellow).bold(),
                ))
            } else {
                Line::from(Span::styled(
                    base,
                    Style::default().fg(Color::DarkGray),
                ))
            }
        }).collect();

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)))
            .highlight_style(Style::default().fg(Color::Yellow));

        f.render_widget(tabs, area);
    }
}
