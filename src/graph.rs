//! Pluggable graph rendering for every chart in the app.
//!
//! Mirrors the theme module: a `GraphStyle` enum with a small `by_name` lookup,
//! plus a `render` entry point (and `render_with_max` for shared-axis overlays)
//! that dispatches to a per-style implementation. Every sparkline in the UI —
//! aggregated RX/TX, in-row top-connection lines, RTT history, timeline
//! severity layers, etc. — routes through here so a single setting toggles
//! them all.

use ratatui::buffer::Buffer;
use ratatui::prelude::*;
use ratatui::widgets::{Sparkline, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphStyle {
    /// Solid-color stacked blocks (ratatui `Sparkline`, the existing look).
    Bars,
    /// btop-style braille area plot: each column is filled with pixels
    /// from the bottom up to the sample's value height, giving 4× vertical
    /// resolution over `Bars` while keeping the filled-area look.
    Dots,
}

pub const GRAPH_STYLE_NAMES: &[&str] = &["bars", "dots"];

pub fn by_name(name: &str) -> GraphStyle {
    match name.to_lowercase().as_str() {
        "dots" => GraphStyle::Dots,
        _ => GraphStyle::Bars,
    }
}

impl GraphStyle {
    pub fn name(self) -> &'static str {
        match self {
            GraphStyle::Bars => "bars",
            GraphStyle::Dots => "dots",
        }
    }
}

/// Render `data` into `area` using the chosen style.
///
/// `base` is the primary series color (e.g. `theme.rx_rate`). `accent` is
/// reserved for future gradient styles; current styles ignore it but call
/// sites pass it for forward compatibility. Auto-derives the y-axis max
/// from `data` — use [`render_with_max`] when overlaying multiple series
/// that need a shared scale.
pub fn render(
    f: &mut Frame,
    area: Rect,
    data: &[u64],
    style: GraphStyle,
    base: Color,
    accent: Color,
) {
    let max = data.iter().copied().max().unwrap_or(0);
    render_with_max(f, area, data, max, style, base, accent);
}

/// Like [`render`], but with an explicit y-axis max — required when
/// multiple layers must share a scale (e.g. the timeline's three-color
/// severity overlay).
pub fn render_with_max(
    f: &mut Frame,
    area: Rect,
    data: &[u64],
    max: u64,
    style: GraphStyle,
    base: Color,
    _accent: Color,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    match style {
        GraphStyle::Bars => render_bars(f, area, data, max, base),
        GraphStyle::Dots => render_dots(f.buffer_mut(), area, data, max, base),
    }
}

fn render_bars(f: &mut Frame, area: Rect, data: &[u64], max: u64, base: Color) {
    let mut spark = Sparkline::default()
        .data(data)
        .style(Style::default().fg(base));
    if max > 0 {
        spark = spark.max(max);
    }
    spark.render(area, f.buffer_mut());
}

// ── braille pixel-dot line plot ─────────────────────────────────────────────

/// Bit position in a braille cell mask for each (sub_col, sub_row).
/// Braille pattern dots numbered 1–8 map to bits 0–7; the 4th row uses dots
/// 7 and 8 (bits 6 and 7) which is why it's not a straight `row + col*4`.
const BRAILLE_BIT: [[u8; 4]; 2] = [
    [0, 1, 2, 6], // sub_col 0: rows 0..=3 → dots 1, 2, 3, 7
    [3, 4, 5, 7], // sub_col 1: rows 0..=3 → dots 4, 5, 6, 8
];

const BRAILLE_BASE: u32 = 0x2800;

fn render_dots(buf: &mut Buffer, area: Rect, data: &[u64], max: u64, color: Color) {
    if max == 0 || data.is_empty() {
        return;
    }

    let cell_w = area.width as usize;
    let cell_h = area.height as usize;
    if cell_w == 0 || cell_h == 0 {
        return;
    }
    let pix_h = cell_h * 4;

    // Right-align samples: one sample per cell column.
    let start = data.len().saturating_sub(cell_w);
    let samples = &data[start..];

    let mut masks = vec![vec![0u8; cell_w]; cell_h];

    for (i, &v) in samples.iter().enumerate() {
        if v == 0 {
            // Skip empty samples so flat-zero spans render nothing instead
            // of a baseline floor. This matters for stacked overlays (e.g.
            // the timeline's green/yellow/red layers, which carry zeros on
            // columns owned by other layers).
            continue;
        }
        // Highest pixel row (counted from the bottom) that this sample
        // reaches; fill every pixel from the bottom up to and including
        // it so the area below the peak is shaded. Only the LEFT
        // sub-column is lit per cell — half the horizontal density of
        // the original area plot, giving the classic btop comb look
        // where adjacent samples sit slightly apart.
        let v = v.min(max);
        let top_pixel_from_bottom = ((v as u128 * (pix_h as u128 - 1)) / max as u128) as usize;
        for fill in 0..=top_pixel_from_bottom {
            let pix_y_from_top = (pix_h - 1) - fill;
            let cell_y = pix_y_from_top / 4;
            let row_in_cell = pix_y_from_top % 4;
            masks[cell_y][i] |= 1 << BRAILLE_BIT[0][row_in_cell];
        }
    }

    for (y, row_masks) in masks.iter().enumerate() {
        for (x, &mask) in row_masks.iter().enumerate() {
            if mask == 0 {
                continue;
            }
            let ch = char::from_u32(BRAILLE_BASE | mask as u32).unwrap_or(' ');
            let cell = buf.get_mut(area.x + x as u16, area.y + y as u16);
            cell.set_char(ch);
            cell.set_style(Style::default().fg(color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_name_falls_back_to_bars() {
        assert_eq!(by_name("nonsense"), GraphStyle::Bars);
        assert_eq!(by_name(""), GraphStyle::Bars);
    }

    #[test]
    fn by_name_recognises_known_styles() {
        assert_eq!(by_name("bars"), GraphStyle::Bars);
        assert_eq!(by_name("DOTS"), GraphStyle::Dots);
    }

    #[test]
    fn name_roundtrips_through_by_name() {
        for name in GRAPH_STYLE_NAMES {
            assert_eq!(by_name(name).name(), *name);
        }
    }
}
