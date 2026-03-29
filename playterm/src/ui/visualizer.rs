//! Spectrum visualizer bar chart widget.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Unicode block characters for sub-row precision: ▁▂▃▄▅▆▇█
const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render a spectrum visualizer into `area`.
///
/// Each bar is 2 columns wide with no gap between bars, so
/// `num_bars = (area.width / 2).min(32)` and bar `i` occupies
/// columns `i*2` and `i*2+1`, filling the full pane width.
///
/// Does nothing if all bands are zero (startup / toggled off).
pub fn render_visualizer(f: &mut Frame, area: Rect, bands: &[f32], accent: Color) {
    if area.width == 0 || area.height == 0 || bands.is_empty() {
        return;
    }

    // Skip rendering when the visualizer was just toggled off and reset.
    if bands.iter().all(|&b| b == 0.0) {
        return;
    }

    // Drop the last 2 bands: the highest-frequency bands map to only 1-2 FFT
    // bins and spike independently.  Render the first 30 of 32 computed bands.
    let visible_bands = bands.len().saturating_sub(2);

    // Each bar is 2 cols wide, no gap → num_bars = area.width / 2, capped at 32.
    let num_bars = ((area.width / 2) as usize).min(32).min(visible_bands);
    if num_bars == 0 {
        return;
    }

    for i in 0..num_bars {
        // Map bar index evenly across the 30 visible bands.
        let band_idx = i * visible_bands / num_bars;
        let band_val = bands[band_idx].clamp(0.0, 1.0);

        // Total height in units of 1/8 of a row for sub-row precision.
        let total_units = (band_val * area.height as f32 * 8.0) as usize;
        let full_rows = (total_units / 8).min(area.height as usize);
        let partial_idx = total_units % 8;

        // Bar occupies columns i*2 and i*2+1.
        let col_x = area.x + (i as u16) * 2;
        if col_x + 1 >= area.x + area.width {
            break;
        }
        let bar_rect = Rect::new(col_x, area.y, 2, area.height);

        // Build lines top→bottom: empty rows, optional partial block, full-block rows.
        // Each line is 2 chars wide to match the bar rect width.
        let has_partial = partial_idx > 0 && full_rows < area.height as usize;
        let top_empty =
            area.height as usize - full_rows - if has_partial { 1 } else { 0 };

        let partial_str: String = BLOCKS[partial_idx.saturating_sub(1)].to_string().repeat(2);

        let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
        for _ in 0..top_empty {
            lines.push(Line::from("  "));
        }
        if has_partial {
            lines.push(Line::from(Span::styled(partial_str, Style::default().fg(accent))));
        }
        for _ in 0..full_rows {
            lines.push(Line::from(Span::styled("██", Style::default().fg(accent))));
        }

        f.render_widget(Paragraph::new(lines), bar_rect);
    }
}
