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
/// `bands` must be a slice of 0.0–1.0 normalised band amplitudes.
/// Each bar is 1 column wide with a 1-column gap, so up to `area.width / 2`
/// bars are drawn, clamped to 8..=32.
///
/// Does nothing if all bands are zero (startup / turned off).
pub fn render_visualizer(f: &mut Frame, area: Rect, bands: &[f32], accent: Color) {
    if area.width == 0 || area.height == 0 || bands.is_empty() {
        return;
    }

    // Skip rendering when the visualizer was just toggled off and reset.
    if bands.iter().all(|&b| b == 0.0) {
        return;
    }

    // Drop the last 2 bands: the highest-frequency bars are sparse single-bin
    // regions that spike independently and look like noise.  Rendering 30 of
    // the 32 computed bands cuts the noisy tail without touching the FFT logic.
    let visible_bands = bands.len().saturating_sub(2);

    // area.width is the inner pane width after block-border removal.
    // Layout path: center → 50% right col → 25% bottom row → block inner.
    // Examples: 80-col terminal → area.width ≈ 38; 140-col → area.width ≈ 68.
    // At 2 cols per bar (1 bar + 1 gap): area.width / 2 is the natural count.
    let num_bars = ((area.width / 2) as usize)
        .min(30)
        .max(28)  // always attempt at least 28 bars
        .min(visible_bands);

    // Floating-point step distributes all num_bars evenly across area.width.
    // When num_bars > area.width / 2 (narrow terminal), bars pack tighter than
    // 2 cols but still span the full width without the break-guard cutting them.
    let bar_step_f = area.width as f32 / num_bars as f32;

    for i in 0..num_bars {
        // Map bar index to the corresponding visible band (first 30 of 32).
        let band_idx = i * visible_bands / num_bars;
        let band_val = bands[band_idx].clamp(0.0, 1.0);

        // Total height in units of 1/8 of a row.
        let total_units = (band_val * area.height as f32 * 8.0) as usize;
        let full_rows = (total_units / 8).min(area.height as usize);
        let partial_idx = total_units % 8;

        let col_x = area.x + (i as f32 * bar_step_f) as u16;
        if col_x >= area.x + area.width {
            break;
        }
        let bar_rect = Rect::new(col_x, area.y, 1, area.height);

        // Build lines from top to bottom.
        // Layout (top→bottom): empty rows, optional partial block, full-block rows.
        let has_partial = partial_idx > 0 && full_rows < area.height as usize;
        let top_empty = area.height as usize - full_rows - if has_partial { 1 } else { 0 };

        let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);

        for _ in 0..top_empty {
            lines.push(Line::from(" "));
        }
        if has_partial {
            let ch = BLOCKS[partial_idx - 1].to_string();
            lines.push(Line::from(Span::styled(ch, Style::default().fg(accent))));
        }
        for _ in 0..full_rows {
            lines.push(Line::from(Span::styled("█", Style::default().fg(accent))));
        }

        f.render_widget(Paragraph::new(lines), bar_rect);
    }
}
