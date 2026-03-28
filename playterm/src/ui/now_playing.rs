use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};

use crate::app::App;
use super::{ACCENT, BG, SURFACE, TEXT_MUTED};

// ── Top-level: 3-column Spotify-style bar ────────────────────────────────────

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // track info
            Constraint::Percentage(40), // transport controls
            Constraint::Percentage(30), // progress + time
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

fn render_controls(app: &App, frame: &mut Frame, area: Rect) {
    // Play/pause button: bracketed + accent when a song is loaded, muted otherwise.
    let (play_label, play_style) = if app.playback.current_song.is_none() {
        (
            "  ▶  ",
            Style::default().fg(TEXT_MUTED),
        )
    } else if app.playback.paused {
        (
            "( ▶ )",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        (
            "( ⏸ )",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    };

    let controls = Line::from(vec![
        Span::styled("  ⇄  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("  ⏮  ", Style::default().fg(TEXT_MUTED)),
        Span::styled(play_label, play_style),
        Span::styled("  ⏭  ", Style::default().fg(TEXT_MUTED)),
        Span::styled("  ↻  ", Style::default().fg(TEXT_MUTED)),
    ]);

    // Vertically: 1 blank row, controls row, 2 blank rows → sits at row 1 of 4.
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

// ── Right 30%: progress gauge + elapsed / total ───────────────────────────────

fn render_progress(app: &App, frame: &mut Frame, area: Rect) {
    // Vertical: top pad | gauge | time text | bottom fill
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top padding
            Constraint::Length(1), // progress gauge
            Constraint::Length(1), // elapsed / total
            Constraint::Min(0),    // bottom fill
        ])
        .split(area);

    // Background fill for padding rows
    let fill_style = Style::default().bg(SURFACE);
    frame.render_widget(Paragraph::new("").style(fill_style), rows[0]);
    if rows[3].height > 0 {
        frame.render_widget(Paragraph::new("").style(fill_style), rows[3]);
    }

    // Gauge (no embedded label — time is on its own row below)
    let ratio = match (&app.playback.current_song, app.playback.total) {
        (Some(_), Some(total)) if !total.is_zero() => {
            (app.playback.elapsed.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };
    frame.render_widget(
        Gauge::default()
            .style(Style::default().bg(SURFACE))
            .gauge_style(Style::default().bg(ACCENT).fg(BG))
            .ratio(ratio),
        rows[1],
    );

    // Time: right-aligned, small padding on the right edge
    let elapsed = app.playback.elapsed.as_secs();
    let time_str = match app.playback.total {
        Some(t) => {
            let ts = t.as_secs();
            format!(
                "{}:{:02}  ·  {}:{:02}  ",
                elapsed / 60,
                elapsed % 60,
                ts / 60,
                ts % 60,
            )
        }
        None => format!("{}:{:02}  ", elapsed / 60, elapsed % 60),
    };
    frame.render_widget(
        Paragraph::new(time_str)
            .alignment(Alignment::Right)
            .style(Style::default().fg(TEXT_MUTED).bg(SURFACE)),
        rows[2],
    );
}
