use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use super::{ACCENT, TEXT_MUTED, BG, TEXT};

// ── Key-legend helpers ────────────────────────────────────────────────────────

/// One keybind entry: the key label and the action description.
struct Bind<'a> {
    key: &'a str,
    action: &'a str,
}

impl<'a> Bind<'a> {
    fn new(key: &'a str, action: &'a str) -> Self {
        Self { key, action }
    }
}

/// Build the spans for a row of keybind groups, truncating to `max_width`.
/// Format: " key Action │ key Action │ …"
fn build_legend<'a>(binds: &[Bind<'a>], max_width: u16) -> Vec<Span<'a>> {
    let key_style    = Style::default().fg(TEXT_MUTED);
    let action_style = Style::default().fg(ACCENT);
    let sep_style    = Style::default().fg(TEXT_MUTED);

    // Pre-build (key, action, sep?) tuples with character widths so we can
    // truncate before committing any Spans to the output.
    let sep = " │ ";
    let n = binds.len();
    let mut spans: Vec<Span<'a>> = Vec::new();
    let mut used: u16 = 1; // leading space

    spans.push(Span::raw(" "));

    for (i, bind) in binds.iter().enumerate() {
        let is_last = i + 1 == n;
        let chunk_w = (bind.key.len() + 1 + bind.action.len()) as u16
            + if is_last { 0 } else { sep.len() as u16 };

        if used + chunk_w > max_width {
            break;
        }

        spans.push(Span::styled(bind.key, key_style));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(bind.action, action_style));

        if !is_last {
            used += chunk_w;
            spans.push(Span::styled(sep, sep_style));
        }
    }

    spans
}

// ── Public render ─────────────────────────────────────────────────────────────

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let line = if app.search_mode.active {
        Line::from(vec![
            Span::styled("/ ", Style::default().fg(ACCENT)),
            Span::styled(app.search_mode.query.as_str(), Style::default().fg(TEXT)),
            Span::styled("_", Style::default().fg(ACCENT)),
            Span::raw("   "),
            Span::styled("Enter", Style::default().fg(TEXT_MUTED)),
            Span::raw(" "),
            Span::styled("Confirm", Style::default().fg(ACCENT)),
            Span::styled("  │  ", Style::default().fg(TEXT_MUTED)),
            Span::styled("Esc", Style::default().fg(TEXT_MUTED)),
            Span::raw(" "),
            Span::styled("Cancel", Style::default().fg(ACCENT)),
        ])
    } else {
        let host = app.config.subsonic_url
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        let binds = [
            Bind::new("h/l", "Columns"),
            Bind::new("j/k", "Scroll"),
            Bind::new("Tab", "Switch"),
            Bind::new("/",   "Search"),
            Bind::new("a",   "Add"),
            Bind::new("A",   "Add All"),
            Bind::new("D",   "Clear"),
            Bind::new("p",   "Play"),
            Bind::new("n",   "Next"),
            Bind::new("N",   "Prev"),
            Bind::new("q",   "Quit"),
        ];

        let host_span_w = (2 + host.len()) as u16; // "● " + host
        let legend_w = area.width.saturating_sub(host_span_w);

        let mut spans: Vec<Span> = vec![
            Span::styled("● ", Style::default().fg(ACCENT)),
            Span::styled(host.to_string(), Style::default().fg(TEXT_MUTED)),
        ];
        spans.extend(build_legend(&binds, legend_w));

        Line::from(spans)
    };

    let para = Paragraph::new(line).style(Style::default().bg(BG));
    frame.render_widget(para, area);
}
