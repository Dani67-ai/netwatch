use crate::app::App;
use crate::collectors::insights::InsightsStatus;
use crate::ui::widgets;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // status bar
            Constraint::Min(8),    // insights content
            Constraint::Length(3), // footer
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_status(f, app, chunks[1]);
    render_insights(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    widgets::render_header(f, app, area);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let Some(collector) = app.insights_collector.as_ref() else {
        return;
    };
    let status = collector.get_status();
    let model = &collector.model;

    let (status_text, status_style) = match &status {
        InsightsStatus::Idle => (
            "Waiting for packet data...".to_string(),
            Style::default().fg(app.theme.text_muted),
        ),
        InsightsStatus::Analyzing => (
            format!("🔄 Analyzing with {}...", model),
            Style::default().fg(app.theme.status_warn),
        ),
        InsightsStatus::Available => (
            format!("✓ AI analysis via {} (auto-refreshes every 15s)", model),
            Style::default().fg(app.theme.status_good),
        ),
        InsightsStatus::Error(e) => (
            format!("✗ Error: {}", e),
            Style::default().fg(app.theme.status_error),
        ),
        InsightsStatus::OllamaUnavailable => (
            "✗ Ollama not running — start with: ollama serve".to_string(),
            Style::default().fg(app.theme.status_error),
        ),
    };

    let status_bar = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {} ", status_text),
        status_style,
    )]))
    .block(
        Block::default()
            .title(" AI Analysis ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border)),
    );
    f.render_widget(status_bar, area);
}

fn render_insights(f: &mut Frame, app: &App, area: Rect) {
    let Some(collector) = app.insights_collector.as_ref() else {
        return;
    };
    let insights = collector.get_insights();

    let block = Block::default()
        .title(format!(" Network Insights ({}) ", insights.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if insights.is_empty() {
        let status = collector.get_status();
        let msg = match status {
            InsightsStatus::OllamaUnavailable => {
                let endpoint = &app
                    .insights_collector
                    .as_ref()
                    .map(|c| c.endpoint.clone())
                    .unwrap_or_default();
                let is_local = endpoint == "local" || endpoint.is_empty();
                if is_local {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Ollama is not running. To enable AI insights:",
                            Style::default().fg(app.theme.status_warn),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "    1. Install Ollama: https://ollama.com",
                            Style::default().fg(app.theme.text_primary),
                        )),
                        Line::from(Span::styled(
                            "    2. Pull a model:   ollama pull llama3.2",
                            Style::default().fg(app.theme.text_primary),
                        )),
                        Line::from(Span::styled(
                            "    3. Start serving:  ollama serve",
                            Style::default().fg(app.theme.text_primary),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Or set AI Endpoint in Settings (,) to use Ollama cloud.",
                            Style::default().fg(app.theme.text_muted),
                        )),
                    ]
                } else {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Could not reach the configured AI endpoint:",
                            Style::default().fg(app.theme.status_warn),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("    {}", endpoint),
                            Style::default().fg(app.theme.text_primary),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Check the AI Endpoint setting in Settings (,).",
                            Style::default().fg(app.theme.text_muted),
                        )),
                    ]
                }
            }
            _ => vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Start capturing packets on tab [4] to enable AI analysis.",
                    Style::default().fg(app.theme.text_muted),
                )),
                Line::from(Span::styled(
                    "  Insights will appear here automatically every 15 seconds.",
                    Style::default().fg(app.theme.text_muted),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press 'a' to trigger analysis immediately.",
                    Style::default().fg(app.theme.text_muted),
                )),
            ],
        };
        let content = Paragraph::new(msg);
        f.render_widget(content, inner);
        return;
    }

    // Build display lines from insights (most recent first)
    let mut lines: Vec<Line> = Vec::new();
    let visible_height = inner.height as usize;

    for insight in insights.iter().rev() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("─── {} ", insight.timestamp),
                Style::default().fg(app.theme.brand).bold(),
            ),
            Span::styled(
                "─".repeat(inner.width.saturating_sub(16) as usize),
                Style::default().fg(app.theme.border),
            ),
        ]));

        for text_line in insight.text.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", text_line))));
        }

        lines.push(Line::from(""));
    }

    // Apply scroll
    let total_lines = lines.len();
    let scroll = app
        .scroll
        .insights_scroll
        .min(total_lines.saturating_sub(visible_height));
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    let content = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
    f.render_widget(content, inner);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let hints = vec![
        Span::styled("a", Style::default().fg(app.theme.key_hint).bold()),
        Span::raw(":Analyze  "),
        Span::styled("p", Style::default().fg(app.theme.key_hint).bold()),
        Span::raw(":Pause  "),
        Span::styled("r", Style::default().fg(app.theme.key_hint).bold()),
        Span::raw(":Refresh  "),
        Span::styled(",", Style::default().fg(app.theme.key_hint).bold()),
        Span::raw(":AI Settings"),
    ];
    widgets::render_footer(f, app, area, hints);
}
