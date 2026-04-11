use std::time::Instant;

use crate::app::App;
use crate::collectors::connections::TrackedConnection;
use crate::ui::widgets;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(6),    // timeline chart
            Constraint::Length(3), // legend
            Constraint::Length(3), // summary
            Constraint::Length(3), // footer
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_chart(f, app, chunks[1]);
    render_legend(f, app, chunks[2]);
    render_summary(f, app, chunks[3]);
    render_footer(f, app, chunks[4]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let window_label = app.timeline_window.label();
    let extra = vec![
        Span::raw("  "),
        Span::styled(
            format!("last {}", window_label),
            Style::default().fg(app.theme.status_good),
        ),
    ];
    widgets::render_header_with_extra(f, app, area, extra);
}

fn render_chart(f: &mut Frame, app: &App, area: Rect) {
    let window_secs = app.timeline_window.seconds();
    let now = Instant::now();
    let window_start = now - std::time::Duration::from_secs(window_secs);

    // Sort: active first, then by first_seen (oldest at top)
    let mut sorted: Vec<&TrackedConnection> = app
        .connection_timeline
        .tracked
        .iter()
        .filter(|t| t.last_seen >= window_start)
        .collect();
    sorted.sort_by(|a, b| {
        b.is_active
            .cmp(&a.is_active)
            .then_with(|| a.first_seen.cmp(&b.first_seen))
    });

    let block = Block::default()
        .title(format!(
            " Timeline ({}) — now ← {:>30} → {} ago ",
            sorted.len(),
            "",
            app.timeline_window.label(),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 || inner.width < 30 {
        return;
    }

    let label_width = 24u16; // "process    remote_ip   "
    let bar_width = inner.width.saturating_sub(label_width + 1) as usize;

    if bar_width < 5 {
        return;
    }

    // Build time axis
    let time_axis = build_time_axis(
        app.timeline_window.seconds(),
        label_width as usize,
        bar_width,
        &app.theme,
    );

    let visible_rows = inner.height.saturating_sub(1) as usize;
    let scroll = app
        .scroll.timeline_scroll
        .min(sorted.len().saturating_sub(visible_rows.max(1)));
    let visible: Vec<&TrackedConnection> = sorted
        .iter()
        .skip(scroll)
        .take(visible_rows)
        .copied()
        .collect();

    let lines: Vec<Line> = visible
        .iter()
        .enumerate()
        .map(|(i, tracked)| {
            let is_selected = i + scroll == app.scroll.timeline_scroll;

            // Build label: "process    remote_ip"
            let proc_name = tracked.process_name.as_deref().unwrap_or("—");
            let remote = extract_ip(&tracked.key.remote_addr);
            let label = format!(
                " {:<10} {:<12}",
                truncate(proc_name, 10),
                truncate(&remote, 12)
            );

            let label_style = if is_selected {
                Style::default().fg(app.theme.active_tab).bold()
            } else if tracked.is_active {
                Style::default().fg(app.theme.text_primary)
            } else {
                Style::default().fg(app.theme.text_muted)
            };

            // Build the bar
            let bar = render_bar(tracked, now, window_start, bar_width, &app.theme);

            let mut spans = vec![Span::styled(label, label_style), Span::raw(" ")];
            spans.extend(bar);

            Line::from(spans)
        })
        .collect();

    let mut all_lines = vec![time_axis];
    all_lines.extend(lines);
    let content = Paragraph::new(all_lines);
    f.render_widget(content, inner);
}

fn render_bar(
    tracked: &TrackedConnection,
    now: Instant,
    window_start: Instant,
    width: usize,
    theme: &crate::theme::Theme,
) -> Vec<Span<'static>> {
    let window_duration = now.duration_since(window_start).as_secs_f64();
    if window_duration <= 0.0 || width == 0 {
        return vec![Span::raw(" ".repeat(width))];
    }

    let first = tracked.first_seen.max(window_start);
    let last = tracked.last_seen.min(now);

    if first > last {
        return vec![Span::raw(" ".repeat(width))];
    }

    // Flipped axis: left = now (col 0), right = window_start (col width)
    // A point at time T maps to col: (now - T) / window_duration * width
    let start_frac = now.duration_since(last).as_secs_f64() / window_duration;
    let end_frac = now.duration_since(first).as_secs_f64() / window_duration;

    let start_col = (start_frac * width as f64).floor() as usize;
    let end_col = (end_frac * width as f64).ceil() as usize;
    let start_col = start_col.min(width);
    let end_col = end_col.max(start_col + 1).min(width);

    let (bar_char, color) = bar_style(tracked, theme);

    let mut spans = Vec::new();

    // If active, mark the leftmost cell (now edge) with ▓
    if tracked.is_active && start_col == 0 {
        spans.push(Span::styled("▓".to_string(), Style::default().fg(color)));
        let bar_len = (end_col - start_col).saturating_sub(1);
        if bar_len > 0 {
            spans.push(Span::styled(
                bar_char.to_string().repeat(bar_len),
                Style::default().fg(color),
            ));
        }
    } else {
        if start_col > 0 {
            spans.push(Span::raw(" ".repeat(start_col)));
        }
        let bar_len = end_col - start_col;
        spans.push(Span::styled(
            bar_char.to_string().repeat(bar_len),
            Style::default().fg(color),
        ));
    }

    let used = end_col;
    if used < width {
        spans.push(Span::raw(" ".repeat(width - used)));
    }

    spans
}

fn bar_style(tracked: &TrackedConnection, theme: &crate::theme::Theme) -> (char, Color) {
    if !tracked.is_active {
        return ('░', theme.text_muted);
    }
    match tracked.state.as_str() {
        "ESTABLISHED" => ('█', theme.status_good),
        "LISTEN" => ('█', theme.status_warn),
        "SYN_SENT" | "SYN_RECV" | "SYN_RECEIVED" => ('░', theme.status_info),
        "CLOSE_WAIT" | "TIME_WAIT" | "FIN_WAIT_1" | "FIN_WAIT_2" | "FIN_WAIT1" | "FIN_WAIT2" => {
            ('░', theme.status_error)
        }
        _ => ('█', theme.status_good),
    }
}

fn render_legend(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let legend = Paragraph::new(Line::from(vec![
        Span::styled(" ██", Style::default().fg(t.status_good)),
        Span::raw(" Established  "),
        Span::styled("██", Style::default().fg(t.status_warn)),
        Span::raw(" Listen  "),
        Span::styled("░░", Style::default().fg(t.status_info)),
        Span::raw(" Connecting  "),
        Span::styled("░░", Style::default().fg(t.status_error)),
        Span::raw(" Closing  "),
        Span::styled("░░", Style::default().fg(t.text_muted)),
        Span::raw(" Closed  "),
        Span::styled("▓", Style::default().fg(t.status_good)),
        Span::raw(" Active edge"),
    ]))
    .block(
        Block::default()
            .title(" Legend ")
            .title_style(Style::default().fg(t.brand))
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(t.brand)),
    );
    f.render_widget(legend, area);
}

fn render_summary(f: &mut Frame, app: &App, area: Rect) {
    let active = app
        .connection_timeline
        .tracked
        .iter()
        .filter(|t| t.is_active)
        .count();
    let closed = app
        .connection_timeline
        .tracked
        .iter()
        .filter(|t| !t.is_active)
        .count();
    let total = app.connection_timeline.tracked.len();

    let t = &app.theme;
    let summary = Paragraph::new(Line::from(vec![
        Span::styled(" Active: ", Style::default().fg(t.brand).bold()),
        Span::styled(format!("{}", active), Style::default().fg(t.status_good)),
        Span::raw("  │  "),
        Span::styled("Closed: ", Style::default().fg(t.brand).bold()),
        Span::styled(format!("{}", closed), Style::default().fg(t.text_muted)),
        Span::raw("  │  "),
        Span::styled("Total seen: ", Style::default().fg(t.brand).bold()),
        Span::raw(format!("{}", total)),
    ]))
    .block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(t.brand)),
    );
    f.render_widget(summary, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let hints = vec![
        Span::styled("Enter", Style::default().fg(app.theme.key_hint).bold()),
        Span::raw(":→Connections  "),
        Span::styled("t", Style::default().fg(app.theme.key_hint).bold()),
        Span::raw(":Timespan  "),
        Span::styled("p", Style::default().fg(app.theme.key_hint).bold()),
        Span::raw(":Pause"),
    ];
    widgets::render_footer(f, app, area, hints);
}

fn build_time_axis(
    window_secs: u64,
    label_width: usize,
    bar_width: usize,
    theme: &crate::theme::Theme,
) -> Line<'static> {
    // Choose tick interval based on window size
    let (tick_count, tick_label_fn): (usize, Box<dyn Fn(usize) -> String>) = match window_secs {
        0..=60 => (6, Box::new(|i| format!("{}s", i * 10))),
        61..=300 => (5, Box::new(|i| format!("{}m", i))),
        301..=900 => (5, Box::new(|i| format!("{}m", i * 3))),
        901..=1800 => (6, Box::new(|i| format!("{}m", i * 5))),
        _ => (6, Box::new(|i| format!("{}m", i * 10))),
    };

    let muted = theme.text_muted;

    // Build the axis string
    // Left side: label padding + "now"
    let mut spans: Vec<Span<'static>> = vec![Span::styled(
        format!("{:>width$} ", "", width = label_width),
        Style::default().fg(muted),
    )];

    // The axis goes left=now, right=oldest
    // Track character count (not byte count) since '─' is multi-byte
    let segment_width = if tick_count > 0 {
        bar_width / tick_count
    } else {
        bar_width
    };

    let mut axis_chars: Vec<char> = Vec::with_capacity(bar_width);

    // First label: "now"
    for c in "now".chars() {
        axis_chars.push(c);
    }

    for i in 1..=tick_count {
        let target = segment_width * i;
        let label = tick_label_fn(i);
        let label_char_count = label.chars().count();
        let fill = target
            .saturating_sub(axis_chars.len())
            .saturating_sub(label_char_count);
        for _ in 0..fill {
            axis_chars.push('─');
        }
        for c in label.chars() {
            axis_chars.push(c);
        }
    }

    // Pad or truncate to bar_width (by character count)
    while axis_chars.len() < bar_width {
        axis_chars.push(' ');
    }
    axis_chars.truncate(bar_width);

    let axis: String = axis_chars.into_iter().collect();
    spans.push(Span::styled(axis, Style::default().fg(muted)));
    Line::from(spans)
}

fn extract_ip(addr: &str) -> String {
    if addr == "*:*" || addr.is_empty() {
        return "—".to_string();
    }
    if let Some(bracket_end) = addr.rfind("]:") {
        addr[1..bracket_end].to_string()
    } else if let Some(colon) = addr.rfind(':') {
        let ip = &addr[..colon];
        if ip == "*" {
            "—".to_string()
        } else {
            ip.to_string()
        }
    } else {
        addr.to_string()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{}…", truncated)
    }
}
