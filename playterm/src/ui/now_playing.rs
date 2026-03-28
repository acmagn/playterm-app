use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use super::{ACCENT, TEXT_MUTED, BG};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let line = if let Some(song) = &app.playback.current_song {
        let title = song.title.as_str();
        let artist = song.artist.as_deref().unwrap_or("Unknown Artist");
        let album = song.album.as_deref().unwrap_or("");

        let elapsed = app.playback.elapsed.as_secs();
        let total = app.playback.total.map(|d| d.as_secs());
        let time_str = match total {
            Some(t) => format!("  {}:{:02} / {}:{:02}", elapsed / 60, elapsed % 60, t / 60, t % 60),
            None => format!("  {}:{:02}", elapsed / 60, elapsed % 60),
        };

        let pause_indicator = if app.playback.paused { "⏸ " } else { "▶ " };

        Line::from(vec![
            Span::styled(pause_indicator, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
            Span::styled(artist, Style::default().fg(super::TEXT)),
            Span::styled("  ·  ", Style::default().fg(TEXT_MUTED)),
            Span::styled(album, Style::default().fg(TEXT_MUTED)),
            Span::styled(time_str, Style::default().fg(TEXT_MUTED)),
        ])
    } else {
        Line::from(Span::styled("No track playing", Style::default().fg(TEXT_MUTED)))
    };

    let para = Paragraph::new(line).style(Style::default().bg(BG));
    frame.render_widget(para, area);
}
