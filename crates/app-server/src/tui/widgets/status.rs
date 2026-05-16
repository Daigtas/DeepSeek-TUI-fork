use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};
use deepseek_tui_core::{AgentMode, BudgetZone, ContextBudget};

/// Render the status bar at the bottom of the screen.
pub fn render(
    f: &mut Frame,
    area: Rect,
    mode: AgentMode,
    model: &str,
    budget: &ContextBudget,
    round_count: usize,
    streaming: bool,
) {
    let zone_color = match budget.zone() {
        BudgetZone::Ok => Color::Green,
        BudgetZone::Warning => Color::Yellow,
        BudgetZone::Critical => Color::Red,
    };

    let used_k = budget.used_tokens as f64 / 1000.0;
    let remaining_pct = budget.pct_remaining() * 100.0;

    let streaming_indicator = if streaming {
        Span::styled(" ● STREAMING ", Style::default().fg(Color::Green).bold())
    } else {
        Span::styled(" ○ idle ", Style::default().fg(Color::DarkGray))
    };

    let mode_text = format!(" {} {} ", mode.emoji(), mode.label());
    let mode_span = Span::styled(mode_text, Style::default().fg(Color::Yellow).bold());

    let model_span = Span::styled(
        format!(" {} ", model),
        Style::default().fg(Color::Cyan),
    );

    let ctx_text = format!(
        " ctx {:.0}% ({:.1}K used) ",
        remaining_pct, used_k
    );
    let ctx_span = Span::styled(ctx_text, Style::default().fg(zone_color));

    let rounds_text = format!(" {}r ", round_count);
    let rounds_span = Span::styled(rounds_text, Style::default().fg(Color::DarkGray));

    let gauge_pct = (1.0 - budget.pct_remaining()).min(1.0).max(0.0);
    let gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(zone_color).bg(Color::DarkGray))
        .percent((gauge_pct * 100.0) as u16);

    let line = Line::from(vec![
        streaming_indicator,
        mode_span,
        model_span,
        ctx_span,
        rounds_span,
    ]);

    let text_area = Rect {
        width: area.width.saturating_sub(20).min(area.width),
        ..area
    };
    let gauge_area = Rect {
        x: area.x + area.width.saturating_sub(20),
        width: 20.min(area.width),
        ..area
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));

    f.render_widget(
        Paragraph::new(line).block(block).alignment(Alignment::Left),
        text_area,
    );
    f.render_widget(gauge, gauge_area);
}
