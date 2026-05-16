use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use deepseek_tui_core::Pane;

use super::widgets::agents::AgentsPane;
use super::widgets::chat::{ChatPane, Message, MessageRole};
use super::widgets::diff::DiffPane;
use super::widgets::pane_bar::PaneBar;
use super::widgets::status;
use super::widgets::tasks::TasksPane;

/// Main TUI application state with multi-pane support.
pub struct TuiApp {
    pub agents: AgentsPane,
    pub chat: ChatPane,
    pub diff: DiffPane,
    pub tasks: TasksPane,
    pub active_pane: Pane,
    pub mode: deepseek_tui_core::AgentMode,
    pub model: String,
    pub budget: deepseek_tui_core::ContextBudget,
    pub round_count: usize,
    pub streaming: bool,
    pub running: bool,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            agents: AgentsPane::new(),
            chat: ChatPane::new(),
            diff: DiffPane::new(),
            tasks: TasksPane::new(),
            active_pane: Pane::Chat,
            mode: deepseek_tui_core::AgentMode::default(),
            model: String::from("deepseek-v4-pro"),
            budget: deepseek_tui_core::ContextBudget::default(),
            round_count: 0,
            streaming: false,
            running: true,
        }
    }

    /// Switch to a different pane.
    pub fn switch_pane(&mut self, pane: Pane) {
        self.active_pane = pane;
    }

    /// Get mutable reference to the scroll offset for the current pane.
    pub fn active_scroll_offset_mut(&mut self) -> &mut usize {
        match self.active_pane {
            Pane::Chat => &mut self.chat.scroll_offset,
            Pane::Diff => &mut self.diff.scroll_offset,
            Pane::Tasks => &mut self.tasks.scroll_offset,
            Pane::Agents => &mut self.agents.scroll_offset,
            _ => &mut self.chat.scroll_offset, // fallback
        }
    }

    pub fn render(&self, f: &mut Frame) {
        let size = f.area();

        // Layout: tab bar + content area + status bar
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Pane bar (tabs)
                Constraint::Min(3),     // Content area
                Constraint::Length(1),  // Status bar
            ])
            .split(size);

        // Tab bar
        PaneBar::render(f, chunks[0], self.active_pane);

        // Content — render active pane
        match self.active_pane {
            Pane::Chat => self.chat.render(f, chunks[1], self.streaming),
            Pane::Diff => self.diff.render(f, chunks[1]),
            Pane::Tasks => self.tasks.render(f, chunks[1]),
            Pane::Agents => self.agents.render(f, chunks[1]),
            _ => self.chat.render(f, chunks[1], self.streaming),
        }

        // Status bar
        status::render(
            f,
            chunks[2],
            self.mode,
            &self.model,
            &self.budget,
            self.round_count,
            self.streaming,
        );
    }

    /// Add a message to the chat history.
    pub fn add_message(&mut self, role: MessageRole, content: String) {
        self.chat.messages.push(Message { role, content });
    }

    /// Reset chat scroll offset to bottom (auto-scroll on new message).
    pub fn scroll_to_bottom(&mut self) {
        self.chat.scroll_offset = 0;
    }
}
