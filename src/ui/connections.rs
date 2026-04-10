use crate::app::App;
use crate::collectors::traceroute::TracerouteStatus;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let layout = crate::ui::widgets::frame_layout(area);
    render_header(f, app, layout.header);
    render_connection_table(f, app, layout.content);
    render_footer(f, app, layout.footer);

    if app.traceroute_view_open {
        render_traceroute_overlay(f, app, area);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let count = app.connection_collector.connections.lock().unwrap().len();
    let extra = vec![
        Span::raw("  "),
        Span::styled(
            format!("{count} connections"),
            Style::default().fg(app.theme.status_good),
        ),
    ];
    crate::ui::widgets::render_header_with_extra(f, app, area, extra);
}

fn render_connection_table(f: &mut Frame, app: &App, area: Rect) {
    let sort_indicator = |col: usize| -> &str {
        if app.sort_column == col {
            " ▼"
        } else {
            ""
        }
    };

    let mut conns = app.connection_collector.connections.lock().unwrap().clone();
    let has_rtt_data = conns.iter().any(|c| c.kernel_rtt_us.is_some());
    let has_sparkline_data = !app.rtt_history.is_empty();

    let mut header_cells = vec![
        Cell::from(format!("Process{}", sort_indicator(0)))
            .style(Style::default().fg(app.theme.brand).bold()),
        Cell::from(format!("PID{}", sort_indicator(1)))
            .style(Style::default().fg(app.theme.brand).bold()),
        Cell::from(format!("Proto{}", sort_indicator(2)))
            .style(Style::default().fg(app.theme.brand).bold()),
        Cell::from(format!("State{}", sort_indicator(3)))
            .style(Style::default().fg(app.theme.brand).bold()),
        Cell::from(format!("Local Address{}", sort_indicator(4)))
            .style(Style::default().fg(app.theme.brand).bold()),
        Cell::from(format!("Remote Address{}", sort_indicator(5)))
            .style(Style::default().fg(app.theme.brand).bold()),
    ];
    if has_rtt_data {
        header_cells.push(Cell::from("RTT").style(Style::default().fg(app.theme.brand).bold()));
    }
    if has_sparkline_data {
        header_cells
            .push(Cell::from("RTT Trend").style(Style::default().fg(app.theme.brand).bold()));
    }
    if app.show_geo {
        header_cells
            .push(Cell::from("Location").style(Style::default().fg(app.theme.brand).bold()));
    }
    let header = Row::new(header_cells).height(1);

    match app.sort_column {
        0 => conns.sort_by(|a, b| {
            a.process_name
                .as_deref()
                .unwrap_or("")
                .cmp(b.process_name.as_deref().unwrap_or(""))
        }),
        1 => conns.sort_by(|a, b| a.pid.cmp(&b.pid)),
        2 => conns.sort_by(|a, b| a.protocol.cmp(&b.protocol)),
        3 => conns.sort_by(|a, b| a.state.cmp(&b.state)),
        4 => conns.sort_by(|a, b| a.local_addr.cmp(&b.local_addr)),
        5 => conns.sort_by(|a, b| a.remote_addr.cmp(&b.remote_addr)),
        _ => {}
    }

    let visible_rows = area.height.saturating_sub(3) as usize; // borders + header
    let scroll = app
        .connection_scroll
        .min(conns.len().saturating_sub(visible_rows));

    let rows: Vec<Row> = conns
        .iter()
        .skip(scroll)
        .enumerate()
        .map(|(i, conn)| {
            let state_style = match conn.state.as_str() {
                "ESTABLISHED" => Style::default().fg(app.theme.status_good),
                "LISTEN" => Style::default().fg(app.theme.status_warn),
                "CLOSE_WAIT" | "TIME_WAIT" => Style::default().fg(app.theme.status_error),
                _ => Style::default().fg(app.theme.text_muted),
            };

            let row_style = if i + scroll == app.connection_scroll {
                Style::default().bg(app.theme.selection_bg)
            } else {
                Style::default()
            };

            let mut cells = vec![
                Cell::from(conn.process_name.as_deref().unwrap_or("—").to_string()),
                Cell::from(
                    conn.pid
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "—".to_string()),
                ),
                Cell::from(conn.protocol.clone()),
                Cell::from(conn.state.clone()).style(state_style),
                Cell::from(conn.local_addr.clone()),
                Cell::from(conn.remote_addr.clone()),
            ];
            if has_rtt_data {
                let (rtt_text, rtt_style) = match conn.kernel_rtt_us {
                    Some(rtt) if rtt > 100_000.0 => (
                        format!("{:.1}ms", rtt / 1000.0),
                        Style::default().fg(app.theme.status_error),
                    ),
                    Some(rtt) if rtt > 10_000.0 => (
                        format!("{:.1}ms", rtt / 1000.0),
                        Style::default().fg(app.theme.status_warn),
                    ),
                    Some(rtt) => (
                        format!("{:.1}ms", rtt / 1000.0),
                        Style::default().fg(app.theme.status_good),
                    ),
                    None => ("—".to_string(), Style::default().fg(app.theme.text_muted)),
                };
                cells.push(Cell::from(rtt_text).style(rtt_style));
            }
            if has_sparkline_data {
                let remote_ip = extract_ip(&conn.remote_addr);
                let (spark_text, spark_style) = remote_ip
                    .and_then(|ip| app.rtt_history.get(ip))
                    .filter(|h| !h.is_empty())
                    .map(|h| {
                        let latest = h.back().copied().unwrap_or(0.0);
                        let text = format!("{} {:.0}ms", rtt_sparkline(h), latest);
                        let color = rtt_sparkline_color(h, &app.theme);
                        (text, Style::default().fg(color))
                    })
                    .unwrap_or_else(|| {
                        ("—".to_string(), Style::default().fg(app.theme.text_muted))
                    });
                cells.push(Cell::from(spark_text).style(spark_style));
            }
            if app.show_geo {
                let remote_ip = extract_ip(&conn.remote_addr);
                let geo_label = remote_ip
                    .and_then(|ip| app.geo_cache.lookup(ip))
                    .map(|g| {
                        if g.city.is_empty() {
                            format!("{} {}", g.country_code, g.org)
                        } else {
                            format!("{} {}, {}", g.country_code, g.city, g.org)
                        }
                    })
                    .unwrap_or_default();
                cells.push(Cell::from(geo_label).style(Style::default().fg(app.theme.text_muted)));
            }
            Row::new(cells).style(row_style)
        })
        .collect();

    let mut widths: Vec<Constraint> = vec![
        Constraint::Length(16),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(14),
        Constraint::Length(22),
        Constraint::Length(22),
    ];
    if has_rtt_data {
        widths.push(Constraint::Length(10));
    }
    if has_sparkline_data {
        widths.push(Constraint::Length(28));
    }
    if app.show_geo {
        widths.push(Constraint::Min(20));
    }
    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title(format!(
                " Connections [{}-{}] ",
                scroll + 1,
                (scroll + visible_rows).min(conns.len())
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border)),
    );

    f.render_widget(table, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let hints = if app.traceroute_view_open {
        vec![
            Span::styled("Esc", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Close  "),
            Span::styled("q", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Quit"),
        ]
    } else {
        vec![
            Span::styled("s", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Sort  "),
            Span::styled("T", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Traceroute  "),
            Span::styled("Enter", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":→Packets"),
        ]
    };
    crate::ui::widgets::render_footer(f, app, area, hints);
}

fn render_traceroute_overlay(f: &mut Frame, app: &App, area: Rect) {
    let result = app.traceroute_runner.result.lock().unwrap();

    let overlay_width = (area.width * 70 / 100)
        .max(50)
        .min(area.width.saturating_sub(4));
    let overlay_height = (area.height * 70 / 100)
        .max(10)
        .min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(overlay_width)) / 2;
    let y = area.y + (area.height.saturating_sub(overlay_height)) / 2;
    let overlay = Rect::new(x, y, overlay_width, overlay_height);

    f.render_widget(Clear, overlay);

    let title = format!(" Traceroute → {} ", result.target);
    let border_color = match result.status {
        TracerouteStatus::Running => app.theme.status_warn,
        TracerouteStatus::Done => app.theme.brand,
        TracerouteStatus::Error(_) => app.theme.status_error,
        TracerouteStatus::Idle => app.theme.text_muted,
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(overlay);
    f.render_widget(block, overlay);

    let mut lines: Vec<Line> = Vec::new();

    match &result.status {
        TracerouteStatus::Running => {
            lines.push(Line::from(Span::styled(
                " ⏳ Running traceroute...",
                Style::default().fg(app.theme.status_warn),
            )));
        }
        TracerouteStatus::Error(msg) => {
            lines.push(Line::from(Span::styled(
                format!(" ✗ Error: {}", msg),
                Style::default().fg(app.theme.status_error),
            )));
        }
        TracerouteStatus::Done => {
            lines.push(Line::from(vec![
                Span::styled(" Hop", Style::default().fg(app.theme.brand).bold()),
                Span::raw("  "),
                Span::styled(
                    format!("{:<40}", "Host / IP"),
                    Style::default().fg(app.theme.brand).bold(),
                ),
                Span::styled("RTT 1     ", Style::default().fg(app.theme.brand).bold()),
                Span::styled("RTT 2     ", Style::default().fg(app.theme.brand).bold()),
                Span::styled("RTT 3", Style::default().fg(app.theme.brand).bold()),
            ]));
            lines.push(Line::from(Span::styled(
                " ───────────────────────────────────────────────────────────────────",
                Style::default().fg(app.theme.text_muted),
            )));
            for hop in &result.hops {
                lines.push(format_hop_line(hop, &app.theme));
            }
            if result.hops.is_empty() {
                lines.push(Line::from(Span::styled(
                    " No hops received",
                    Style::default().fg(app.theme.text_muted),
                )));
            }
        }
        TracerouteStatus::Idle => {
            lines.push(Line::from(Span::styled(
                " No traceroute data",
                Style::default().fg(app.theme.text_muted),
            )));
        }
    }

    let visible_height = inner.height as usize;
    let max_scroll = lines.len().saturating_sub(visible_height);
    let scroll = app.traceroute_scroll.min(max_scroll);
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    let content = Paragraph::new(visible_lines);
    f.render_widget(content, inner);
}

fn format_hop_line(
    hop: &crate::collectors::traceroute::TracerouteHop,
    theme: &crate::theme::Theme,
) -> Line<'static> {
    let hop_num = format!(" {:>2} ", hop.hop_number);
    let host_ip = match (&hop.host, &hop.ip) {
        (Some(h), Some(ip)) => format!("{} ({})", h, ip),
        (None, Some(ip)) => ip.clone(),
        (Some(h), None) => h.clone(),
        (None, None) => "*".to_string(),
    };

    let rtt_spans: Vec<String> = if hop.rtt_ms.is_empty() && hop.ip.is_none() {
        vec!["*".to_string(); 3]
    } else {
        (0..3)
            .map(|i| match hop.rtt_ms.get(i) {
                Some(Some(ms)) => format!("{:>7.2}ms", ms),
                _ => "      *  ".to_string(),
            })
            .collect()
    };

    let rtt_color = hop
        .rtt_ms
        .iter()
        .filter_map(|r| r.as_ref())
        .next()
        .map(|ms| {
            if *ms < 10.0 {
                theme.status_good
            } else if *ms < 50.0 {
                theme.status_warn
            } else if *ms < 100.0 {
                Color::Rgb(255, 165, 0)
            } else {
                theme.status_error
            }
        })
        .unwrap_or(theme.text_muted);

    Line::from(vec![
        Span::styled(hop_num, Style::default().fg(theme.brand)),
        Span::raw("  "),
        Span::styled(
            format!("{:<40}", host_ip),
            Style::default().fg(if hop.ip.is_some() {
                theme.text_primary
            } else {
                theme.text_muted
            }),
        ),
        Span::styled(rtt_spans[0].clone(), Style::default().fg(rtt_color)),
        Span::raw(" "),
        Span::styled(rtt_spans[1].clone(), Style::default().fg(rtt_color)),
        Span::raw(" "),
        Span::styled(rtt_spans[2].clone(), Style::default().fg(rtt_color)),
    ])
}

const SPARKLINE_BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

fn rtt_sparkline(history: &std::collections::VecDeque<f64>) -> String {
    if history.is_empty() {
        return String::new();
    }
    let min = history.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = history.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    history
        .iter()
        .map(|&v| {
            let idx = if range < 0.001 {
                3 // middle block for flat line
            } else {
                let normalized = (v - min) / range;
                (normalized * 7.0).round() as usize
            };
            SPARKLINE_BLOCKS[idx.min(7)]
        })
        .collect()
}

fn rtt_sparkline_color(
    history: &std::collections::VecDeque<f64>,
    theme: &crate::theme::Theme,
) -> Color {
    match history.back() {
        Some(&rtt) if rtt > 100.0 => theme.status_error,
        Some(&rtt) if rtt > 50.0 => theme.status_warn,
        _ => theme.status_good,
    }
}

fn extract_ip(addr: &str) -> Option<&str> {
    if addr == "*:*" || addr.is_empty() {
        return None;
    }
    if let Some(bracket_end) = addr.rfind("]:") {
        Some(&addr[1..bracket_end])
    } else if let Some(colon) = addr.rfind(':') {
        let ip = &addr[..colon];
        if ip == "*" {
            None
        } else {
            Some(ip)
        }
    } else {
        Some(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn sparkline_empty() {
        let h = VecDeque::new();
        assert_eq!(rtt_sparkline(&h), "");
    }

    #[test]
    fn sparkline_single_sample() {
        let h = VecDeque::from(vec![10.0]);
        let s = rtt_sparkline(&h);
        assert_eq!(s.chars().count(), 1);
        // Single sample → flat line → middle block
        assert_eq!(s, "▄");
    }

    #[test]
    fn sparkline_flat_line() {
        let h = VecDeque::from(vec![5.0, 5.0, 5.0, 5.0]);
        let s = rtt_sparkline(&h);
        assert_eq!(s.chars().count(), 4);
        // All same → all middle blocks
        assert!(s.chars().all(|c| c == '▄'));
    }

    #[test]
    fn sparkline_ascending() {
        let h = VecDeque::from(vec![0.0, 50.0, 100.0]);
        let s = rtt_sparkline(&h);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars.len(), 3);
        assert_eq!(chars[0], '▁'); // min
        assert_eq!(chars[2], '█'); // max
    }

    #[test]
    fn sparkline_descending() {
        let h = VecDeque::from(vec![100.0, 50.0, 0.0]);
        let s = rtt_sparkline(&h);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[0], '█'); // max
        assert_eq!(chars[2], '▁'); // min
    }

    #[test]
    fn sparkline_color_green_low() {
        let h = VecDeque::from(vec![10.0, 15.0, 12.0]);
        let theme = crate::theme::dark();
        assert_eq!(rtt_sparkline_color(&h, &theme), theme.status_good);
    }

    #[test]
    fn sparkline_color_yellow_medium() {
        let h = VecDeque::from(vec![10.0, 60.0]);
        let theme = crate::theme::dark();
        assert_eq!(rtt_sparkline_color(&h, &theme), theme.status_warn);
    }

    #[test]
    fn sparkline_color_red_high() {
        let h = VecDeque::from(vec![10.0, 150.0]);
        let theme = crate::theme::dark();
        assert_eq!(rtt_sparkline_color(&h, &theme), theme.status_error);
    }

    #[test]
    fn sparkline_twenty_samples() {
        let h: VecDeque<f64> = (0..20).map(|i| i as f64 * 5.0).collect();
        let s = rtt_sparkline(&h);
        assert_eq!(s.chars().count(), 20);
    }
}
