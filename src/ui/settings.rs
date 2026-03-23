use crate::app::App;
use crate::config::NetwatchConfig;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub const SETTINGS_COUNT: usize = 13;

struct SettingRow {
    label: &'static str,
    value: String,
}

fn build_rows(cfg: &NetwatchConfig) -> Vec<SettingRow> {
    vec![
        SettingRow {
            label: "Theme",
            value: cfg.theme.clone(),
        },
        SettingRow {
            label: "Default Tab",
            value: cfg.default_tab.clone(),
        },
        SettingRow {
            label: "Refresh Rate (ms)",
            value: cfg.refresh_rate_ms.to_string(),
        },
        SettingRow {
            label: "Capture Interface",
            value: if cfg.capture_interface.is_empty() {
                "(auto)".into()
            } else {
                cfg.capture_interface.clone()
            },
        },
        SettingRow {
            label: "Show GeoIP",
            value: if cfg.show_geo { "on" } else { "off" }.into(),
        },
        SettingRow {
            label: "Timeline Window",
            value: cfg.timeline_window.clone(),
        },
        SettingRow {
            label: "Packet Follow",
            value: if cfg.packet_follow { "on" } else { "off" }.into(),
        },
        SettingRow {
            label: "BPF Filter",
            value: if cfg.bpf_filter.is_empty() {
                "(none)".into()
            } else {
                cfg.bpf_filter.clone()
            },
        },
        SettingRow {
            label: "GeoIP DB Path",
            value: if cfg.geoip_db.is_empty() {
                "(none)".into()
            } else {
                cfg.geoip_db.clone()
            },
        },
        SettingRow {
            label: "GeoIP ASN DB Path",
            value: if cfg.geoip_asn_db.is_empty() {
                "(none)".into()
            } else {
                cfg.geoip_asn_db.clone()
            },
        },
        SettingRow {
            label: "Insights Model",
            value: cfg.insights_model.clone(),
        },
        SettingRow {
            label: "Bandwidth Threshold",
            value: format_bandwidth(cfg.alerts.bandwidth_threshold),
        },
        SettingRow {
            label: "Port Scan Threshold",
            value: cfg.alerts.port_scan_threshold.to_string(),
        },
    ]
}

fn format_bandwidth(bytes: u64) -> String {
    if bytes == 0 {
        "disabled".into()
    } else if bytes >= 1_000_000_000 {
        format!("{} GB/s", bytes / 1_000_000_000)
    } else if bytes >= 1_000_000 {
        format!("{} MB/s", bytes / 1_000_000)
    } else if bytes >= 1_000 {
        format!("{} KB/s", bytes / 1_000)
    } else {
        format!("{} B/s", bytes)
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = (area.width * 60 / 100)
        .max(50)
        .min(area.width.saturating_sub(4));
    let popup_height = (SETTINGS_COUNT as u16 + 7).min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup);

    let title = if let Some(ref path) = NetwatchConfig::path() {
        format!(" Settings — {} ", path.display())
    } else {
        " Settings ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.brand));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = build_rows(&app.user_config);
    let label_width = 22;

    let mut lines: Vec<Line> = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let is_selected = i == app.settings_cursor;
        let is_editing = is_selected && app.settings_editing;

        let indicator = if is_selected { "▸ " } else { "  " };
        let label_style = if is_selected {
            Style::default().fg(app.theme.active_tab).bold()
        } else {
            Style::default().fg(app.theme.brand)
        };

        let value_display = if is_editing {
            format!("{}▏", app.settings_edit_buf)
        } else if i == 0 && is_selected {
            format!("◀ {} ▶", row.value)
        } else {
            row.value.clone()
        };

        let value_style = if is_editing {
            Style::default()
                .fg(app.theme.text_primary)
                .bg(app.theme.selection_bg)
        } else if is_selected {
            Style::default().fg(app.theme.text_primary)
        } else {
            Style::default().fg(app.theme.text_muted)
        };

        lines.push(Line::from(vec![
            Span::styled(indicator.to_string(), label_style),
            Span::styled(
                format!("{:<width$}", row.label, width = label_width),
                label_style,
            ),
            Span::styled(value_display, value_style),
        ]));
    }

    // Status message
    lines.push(Line::raw(""));
    if let Some(ref status) = app.settings_status {
        lines.push(Line::from(Span::styled(
            format!("  {}", status),
            Style::default().fg(app.theme.status_good),
        )));
    } else {
        lines.push(Line::raw(""));
    }

    let content_height = inner.height.saturating_sub(1);
    let content = Paragraph::new(lines);
    f.render_widget(
        content,
        Rect::new(inner.x, inner.y, inner.width, content_height),
    );

    // Footer
    let footer_spans = if app.settings_editing {
        vec![
            Span::styled("Enter", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Apply  "),
            Span::styled("Esc", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Cancel"),
        ]
    } else if app.settings_cursor == 0 {
        vec![
            Span::styled("←→", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Theme  "),
            Span::styled("↑↓", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Navigate  "),
            Span::styled("S", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Save  "),
            Span::styled("Esc", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Close"),
        ]
    } else {
        vec![
            Span::styled("↑↓", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Navigate  "),
            Span::styled("Enter", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Edit  "),
            Span::styled("S", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Save  "),
            Span::styled("Esc", Style::default().fg(app.theme.key_hint).bold()),
            Span::raw(":Close"),
        ]
    };
    let footer = Paragraph::new(Line::from(footer_spans)).alignment(Alignment::Center);
    let footer_area = Rect::new(
        inner.x,
        inner.y + inner.height.saturating_sub(1),
        inner.width,
        1,
    );
    f.render_widget(footer, footer_area);
}

/// Returns the raw config value for the setting at `cursor` position,
/// suitable for pre-filling the edit buffer.
pub fn get_edit_value(cfg: &NetwatchConfig, cursor: usize) -> String {
    match cursor {
        0 => cfg.theme.clone(),
        1 => cfg.default_tab.clone(),
        2 => cfg.refresh_rate_ms.to_string(),
        3 => cfg.capture_interface.clone(),
        4 => if cfg.show_geo { "on" } else { "off" }.into(),
        5 => cfg.timeline_window.clone(),
        6 => if cfg.packet_follow { "on" } else { "off" }.into(),
        7 => cfg.bpf_filter.clone(),
        8 => cfg.geoip_db.clone(),
        9 => cfg.geoip_asn_db.clone(),
        10 => cfg.insights_model.clone(),
        11 => cfg.alerts.bandwidth_threshold.to_string(),
        12 => cfg.alerts.port_scan_threshold.to_string(),
        _ => String::new(),
    }
}

/// Apply the edited value back to the config. Returns an error message if invalid.
pub fn apply_edit(cfg: &mut NetwatchConfig, cursor: usize, value: &str) -> Result<(), String> {
    match cursor {
        0 => {
            let valid = crate::theme::THEME_NAMES;
            let v = value.to_lowercase();
            if valid.contains(&v.as_str()) {
                cfg.theme = v;
                Ok(())
            } else {
                Err(format!("Invalid theme. Use: {}", valid.join(", ")))
            }
        }
        1 => {
            let valid = [
                "dashboard",
                "connections",
                "interfaces",
                "packets",
                "stats",
                "topology",
                "timeline",
                "insights",
            ];
            let v = value.to_lowercase();
            if valid.contains(&v.as_str()) {
                cfg.default_tab = v;
                Ok(())
            } else {
                Err(format!("Invalid tab. Use: {}", valid.join(", ")))
            }
        }
        2 => {
            let ms: u64 = value.parse().map_err(|_| "Must be a number".to_string())?;
            if !(100..=5000).contains(&ms) {
                return Err("Must be 100–5000".into());
            }
            cfg.refresh_rate_ms = ms;
            Ok(())
        }
        3 => {
            cfg.capture_interface = value.to_string();
            Ok(())
        }
        4 => {
            match value.to_lowercase().as_str() {
                "on" | "true" | "yes" | "1" => cfg.show_geo = true,
                "off" | "false" | "no" | "0" => cfg.show_geo = false,
                _ => return Err("Use on/off".into()),
            }
            Ok(())
        }
        5 => {
            let valid = ["1m", "5m", "15m", "30m", "1h"];
            if valid.contains(&value) {
                cfg.timeline_window = value.to_string();
                Ok(())
            } else {
                Err(format!("Use: {}", valid.join(", ")))
            }
        }
        6 => {
            match value.to_lowercase().as_str() {
                "on" | "true" | "yes" | "1" => cfg.packet_follow = true,
                "off" | "false" | "no" | "0" => cfg.packet_follow = false,
                _ => return Err("Use on/off".into()),
            }
            Ok(())
        }
        7 => {
            cfg.bpf_filter = value.to_string();
            Ok(())
        }
        8 => {
            cfg.geoip_db = value.to_string();
            Ok(())
        }
        9 => {
            cfg.geoip_asn_db = value.to_string();
            Ok(())
        }
        10 => {
            cfg.insights_model = value.to_string();
            Ok(())
        }
        11 => {
            let v: u64 = value
                .parse()
                .map_err(|_| "Must be a number (bytes/sec)".to_string())?;
            cfg.alerts.bandwidth_threshold = v;
            Ok(())
        }
        12 => {
            let v: usize = value.parse().map_err(|_| "Must be a number".to_string())?;
            cfg.alerts.port_scan_threshold = v;
            Ok(())
        }
        _ => Err("Unknown setting".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_rows_count() {
        let cfg = NetwatchConfig::default();
        let rows = build_rows(&cfg);
        assert_eq!(rows.len(), SETTINGS_COUNT);
    }

    #[test]
    fn get_edit_value_roundtrip() {
        let cfg = NetwatchConfig::default();
        assert_eq!(get_edit_value(&cfg, 0), "dark");
        assert_eq!(get_edit_value(&cfg, 1), "dashboard");
        assert_eq!(get_edit_value(&cfg, 2), "1000");
        assert_eq!(get_edit_value(&cfg, 4), "on");
        assert_eq!(get_edit_value(&cfg, 6), "on");
    }

    #[test]
    fn apply_valid_tab() {
        let mut cfg = NetwatchConfig::default();
        assert!(apply_edit(&mut cfg, 1, "packets").is_ok());
        assert_eq!(cfg.default_tab, "packets");
    }

    #[test]
    fn apply_invalid_tab() {
        let mut cfg = NetwatchConfig::default();
        assert!(apply_edit(&mut cfg, 1, "nonsense").is_err());
    }

    #[test]
    fn apply_refresh_rate_bounds() {
        let mut cfg = NetwatchConfig::default();
        assert!(apply_edit(&mut cfg, 2, "500").is_ok());
        assert_eq!(cfg.refresh_rate_ms, 500);
        assert!(apply_edit(&mut cfg, 2, "50").is_err());
        assert!(apply_edit(&mut cfg, 2, "10000").is_err());
        assert!(apply_edit(&mut cfg, 2, "abc").is_err());
    }

    #[test]
    fn apply_bool_toggle() {
        let mut cfg = NetwatchConfig::default();
        assert!(apply_edit(&mut cfg, 4, "off").is_ok());
        assert!(!cfg.show_geo);
        assert!(apply_edit(&mut cfg, 4, "on").is_ok());
        assert!(cfg.show_geo);
        assert!(apply_edit(&mut cfg, 4, "maybe").is_err());
    }

    #[test]
    fn apply_timeline_window() {
        let mut cfg = NetwatchConfig::default();
        assert!(apply_edit(&mut cfg, 5, "1h").is_ok());
        assert_eq!(cfg.timeline_window, "1h");
        assert!(apply_edit(&mut cfg, 5, "2h").is_err());
    }

    #[test]
    fn apply_bandwidth_threshold() {
        let mut cfg = NetwatchConfig::default();
        assert!(apply_edit(&mut cfg, 11, "50000000").is_ok());
        assert_eq!(cfg.alerts.bandwidth_threshold, 50_000_000);
        assert!(apply_edit(&mut cfg, 11, "not_a_number").is_err());
    }

    #[test]
    fn apply_string_fields() {
        let mut cfg = NetwatchConfig::default();
        assert!(apply_edit(&mut cfg, 3, "en1").is_ok());
        assert_eq!(cfg.capture_interface, "en1");
        assert!(apply_edit(&mut cfg, 7, "tcp port 80").is_ok());
        assert_eq!(cfg.bpf_filter, "tcp port 80");
        assert!(apply_edit(&mut cfg, 10, "llama3:8b").is_ok());
        assert_eq!(cfg.insights_model, "llama3:8b");
    }

    #[test]
    fn apply_theme() {
        let mut cfg = NetwatchConfig::default();
        assert!(apply_edit(&mut cfg, 0, "dracula").is_ok());
        assert_eq!(cfg.theme, "dracula");
        assert!(apply_edit(&mut cfg, 0, "invalid").is_err());
    }

    #[test]
    fn format_bandwidth_values() {
        assert_eq!(format_bandwidth(0), "disabled");
        assert_eq!(format_bandwidth(500), "500 B/s");
        assert_eq!(format_bandwidth(50_000), "50 KB/s");
        assert_eq!(format_bandwidth(100_000_000), "100 MB/s");
        assert_eq!(format_bandwidth(2_000_000_000), "2 GB/s");
    }
}
