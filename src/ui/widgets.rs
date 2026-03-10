use crate::app::{App, Tab};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn format_bytes_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_000_000_000.0 {
        format!("{:.1} GB/s", bytes_per_sec / 1_000_000_000.0)
    } else if bytes_per_sec >= 1_000_000.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec >= 1_000.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0}  B/s", bytes_per_sec)
    }
}

pub fn format_bytes_total(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}

const ALL_TABS: &[Tab] = &[
    Tab::Dashboard, Tab::Connections, Tab::Interfaces, Tab::Packets,
    Tab::Stats, Tab::Topology, Tab::Timeline, Tab::Insights,
];

fn tab_label(tab: Tab) -> (&'static str, &'static str) {
    match tab {
        Tab::Dashboard => ("1", "Dashboard"),
        Tab::Connections => ("2", "Connections"),
        Tab::Interfaces => ("3", "Interfaces"),
        Tab::Packets => ("4", "Packets"),
        Tab::Stats => ("5", "Stats"),
        Tab::Topology => ("6", "Topology"),
        Tab::Timeline => ("7", "Timeline"),
        Tab::Insights => ("8", "Insights"),
    }
}

fn build_header_spans(app: &App, extra: Option<Vec<Span<'static>>>) -> Line<'static> {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();

    let mut spans: Vec<Span<'static>> = vec![
        Span::styled("◉ NetWatch ", Style::default().fg(Color::Cyan).bold()),
    ];

    for (i, &tab) in ALL_TABS.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
        }
        let (num, name) = tab_label(tab);
        let label = format!("[{}] {}", num, name);
        if tab == app.current_tab {
            spans.push(Span::styled(label, Style::default().fg(Color::Yellow).bold()));
        } else {
            spans.push(Span::styled(label, Style::default().fg(Color::DarkGray)));
        }
    }

    if app.paused {
        spans.push(Span::styled(
            " ⏸ PAUSED ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ));
    }

    let alert_count = app.network_intel.active_alert_count();
    if alert_count > 0 {
        spans.push(Span::styled(
            format!(" ⚠ {} ", alert_count),
            Style::default().fg(Color::Black).bg(Color::Red),
        ));
    }

    if let Some(extra_spans) = extra {
        for s in extra_spans {
            spans.push(s);
        }
    }

    spans.push(Span::raw("  "));
    spans.push(Span::styled(now, Style::default().fg(Color::DarkGray)));

    Line::from(spans)
}

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let line = build_header_spans(app, None);
    let header = Paragraph::new(line)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(header, area);
}

pub fn render_header_with_extra(f: &mut Frame, app: &App, area: Rect, extra: Vec<Span<'static>>) {
    let line = build_header_spans(app, Some(extra));
    let header = Paragraph::new(line)
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(header, area);
}

pub fn render_footer(f: &mut Frame, area: Rect, context_hints: Vec<Span<'static>>) {
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];

    for s in context_hints {
        spans.push(s);
    }

    if spans.len() > 1 {
        spans.push(Span::raw("  "));
    }

    let standard_hints: &[(&str, &str)] = &[
        ("q", "Quit"),
        ("↑↓", "Scroll"),
        ("1-8", "Tab"),
        ("?", "Help"),
    ];
    for (i, (key, desc)) in standard_hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(format!("{}", key), Style::default().fg(Color::Yellow).bold()));
        spans.push(Span::raw(format!(":{}", desc)));
    }

    let footer = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(footer, area);
}
