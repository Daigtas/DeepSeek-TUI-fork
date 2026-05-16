/// Ratatui-based terminal UI module.
///
/// Provides a rich, multi-pane terminal interface replacing the raw echo-based
/// TUI with proper ratatui rendering. Supports:
/// - Chat pane with scrollable message history
/// - Status bar with context gauge, mode, model, round count
/// - Future: Diff, Tasks, Agents panes
pub mod app;
pub mod widgets;
