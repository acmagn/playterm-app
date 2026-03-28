use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use super::{ACCENT, SURFACE, TEXT_MUTED};

// ── Top-level: 3-column Spotify-style bar ────────────────────────────────────

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // track info
            Constraint::Percentage(40), // transport controls
            Constraint::Percentage(30), // inline progress
        ])
        .split(area);

    render_track_info(app, frame, cols[0]);
    render_controls(app, frame, cols[1]);
    render_progress(app, frame, cols[2]);
}

// ── Left 30%: track title (accent/bold) + artist (muted) ─────────────────────

fn render_track_info(app: &App, frame: &mut Frame, area: Rect) {
    let lines: Vec<Line> = if let Some(song) = &app.playback.current_song {
        let artist = song.artist.as_deref().unwrap_or("Unknown Artist");
        vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    song.title.as_str(),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(artist, Style::default().fg(TEXT_MUTED)),
            ]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("Not playing", Style::default().fg(TEXT_MUTED)),
            ]),
            Line::from(""),
            Line::from(""),
        ]
    };
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(SURFACE)),
        area,
    );
}

// ── Center 40%: transport controls, centered ─────────────────────────────────
//
// Target: ⇄      ⏮      ( ⏸ )      ⏭      ↻
//         4-6 spaces between each symbol; play/pause bracketed + accent.

fn render_controls(app: &App, frame: &mut Frame, area: Rect) {
    let (play_label, play_style) = if app.playback.current_song.is_none() {
        ("▶", Style::default().fg(TEXT_MUTED))
    } else if app.playback.paused {
        ("( ▶ )", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
    } else {
        ("( ⏸ )", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
    };

    let sep = Style::default().fg(TEXT_MUTED);
    let controls = Line::from(vec![
        Span::styled("⇄", sep),
        Span::raw("      "),
        Span::styled("⏮", sep),
        Span::raw("      "),
        Span::styled(play_label, play_style),
        Span::raw("      "),
        Span::styled("⏭", sep),
        Span::raw("      "),
        Span::styled("↻", sep),
    ]);

    // Place controls on row 1 of 4 (1 blank row above for visual centering).
    let lines: Vec<Line> = vec![
        Line::from(""),
        controls,
        Line::from(""),
        Line::from(""),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().bg(SURFACE)),
        area,
    );
}

// ── Right 30%: inline progress "elapsed  ████░░░░  total" ────────────────────
//
// No Gauge widget — bar is built as a string of █ (ACCENT) and ░ (TEXT_MUTED)
// sized to fit the column width.  Placed on row 2 of 4.

fn render_progress(app: &App, frame: &mut Frame, area: Rect) {
    let (elapsed_str, total_str, ratio) = if let Some(_) = &app.playback.current_song {
        let e = app.playback.elapsed.as_secs();
        let elapsed_str = format!("{}:{:02}", e / 60, e % 60);
        let (total_str, ratio) = match app.playback.total {
            Some(t) => {
                let ts = t.as_secs();
                let r = if ts > 0 { (e as f64 / ts as f64).clamp(0.0, 1.0) } else { 0.0 };
                (format!("{}:{:02}", ts / 60, ts % 60), r)
            }
            None => ("--:--".to_string(), 0.0),
        };
        (elapsed_str, total_str, ratio)
    } else {
        ("0:00".to_string(), "0:00".to_string(), 0.0)
    };

    // Bar width: column width minus elapsed, total, and two 2-space gaps.
    let col_w = area.width as usize;
    let bar_w = col_w.saturating_sub(elapsed_str.len() + total_str.len() + 4);
    let filled = ((ratio * bar_w as f64) as usize).min(bar_w);
    let empty = bar_w - filled;

    let progress = Line::from(vec![
        Span::styled(elapsed_str, Style::default().fg(TEXT_MUTED)),
        Span::raw("  "),
        Span::styled("█".repeat(filled), Style::default().fg(ACCENT)),
        Span::styled("░".repeat(empty), Style::default().fg(TEXT_MUTED)),
        Span::raw("  "),
        Span::styled(total_str, Style::default().fg(TEXT_MUTED)),
    ]);

    // Row 0: empty, Row 1: empty, Row 2: progress, Row 3: empty.
    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(""),
        progress,
        Line::from(""),
    ];

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(SURFACE)),
        area,
    );
}
