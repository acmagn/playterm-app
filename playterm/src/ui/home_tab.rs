use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{HomeSection, HomeState, RecentAlbum};
use crate::ui::kitty_art::{art_strip_thumbnail_size, visible_thumbnail_count};

// ── Relative time formatting ──────────────────────────────────────────────────

fn relative_time(played_at: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(played_at);
    let secs = (now - played_at).max(0) as u64;
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{} min ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hr ago", secs / 3600)
    } else {
        format!("{} days ago", secs / 86400)
    }
}

// ── Section header line ───────────────────────────────────────────────────────

fn section_header<'a>(label: &'a str, is_active: bool, accent: Color) -> Line<'a> {
    if is_active {
        Line::from(Span::styled(
            format!(" \u{25B6} {}", label),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(
            format!("   {}", label),
            Style::default().add_modifier(Modifier::BOLD),
        ))
    }
}

// ── Top-level render ──────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn render_home_tab(
    f: &mut Frame,
    area: Rect,
    home: &HomeState,
    accent: Color,
    kitty_supported: bool,
    home_art_cache: &HashMap<String, Vec<u8>>,
    cell_px: Option<(u16, u16)>,
) {
    let total_rows = area.height;

    // Row 1: 8-row art strip.
    // Row 2: remaining space, split into sections.
    let art_height = 8u16;
    let art_h = art_height.min(total_rows);
    let content_h = total_rows.saturating_sub(art_h);

    let top_level = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(art_h),
            Constraint::Length(content_h),
        ])
        .split(area);

    let art_area     = top_level[0];
    let content_area = top_level[1];

    // ── Art strip ─────────────────────────────────────────────────────────────
    let is_albums_active = home.active_section == HomeSection::RecentAlbums;
    if kitty_supported {
        crate::ui::kitty_art::render_art_strip(
            &home.recent_albums,
            home.album_scroll_offset,
            home.album_selected_index,
            home_art_cache,
            art_area,
            cell_px,
            art_area.x,
            art_area.y,
        );
        // Draw selection indicator (a bracket line below the selected thumbnail)
        // using ratatui so it stays within the layout system.
        render_art_strip_selection_hint(f, art_area, home, accent, cell_px, is_albums_active);
    } else {
        render_art_strip_text_fallback(f, art_area, &home.recent_albums, home.album_selected_index, accent, is_albums_active);
    }

    // ── Content sections ──────────────────────────────────────────────────────
    // Heights: RecentTracks=7, TopArtists=7, Rediscover=4
    // If terminal is short (< 30 rows total), collapse gracefully.
    let (recent_h, top_h, rediscover_h): (u16, u16, u16) = if total_rows < 30 {
        if content_h >= 11 {
            (5, 5, 0) // hide Rediscover, reduce each by 2
        } else {
            let half = content_h / 2;
            (half, content_h.saturating_sub(half), 0)
        }
    } else {
        (7, 7, 4)
    };

    if content_h == 0 {
        return;
    }

    // Build constraints dynamically to avoid zero-height chunks.
    let mut constraints = Vec::new();
    if recent_h > 0    { constraints.push(Constraint::Length(recent_h)); }
    if top_h > 0       { constraints.push(Constraint::Length(top_h)); }
    if rediscover_h > 0 { constraints.push(Constraint::Length(rediscover_h)); }
    // Fill any leftover space.
    constraints.push(Constraint::Min(0));

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(content_area);

    let mut sec_idx = 0usize;

    // ── Recent Tracks ─────────────────────────────────────────────────────────
    if recent_h > 0 {
        render_recent_tracks(f, sections[sec_idx], home, accent);
        sec_idx += 1;
    }

    // ── Top Artists ───────────────────────────────────────────────────────────
    if top_h > 0 {
        render_top_artists(f, sections[sec_idx], home, accent);
        sec_idx += 1;
    }

    // ── Rediscover ────────────────────────────────────────────────────────────
    if rediscover_h > 0 {
        render_rediscover(f, sections[sec_idx], home, accent);
    }
}

// ── Art strip helpers ─────────────────────────────────────────────────────────

/// Text fallback for the art strip (non-Kitty terminals).
/// Renders a horizontal list of album names, with the selected one highlighted.
pub fn render_art_strip_text_fallback(
    f: &mut Frame,
    area: Rect,
    albums: &[RecentAlbum],
    selected_index: usize,
    accent: Color,
    is_active: bool,
) {
    if area.height == 0 {
        return;
    }

    // Section header on row 0.
    let header = section_header("Recently Played Albums", is_active, accent);
    f.render_widget(
        Paragraph::new(header),
        Rect { height: 1, ..area },
    );

    if albums.is_empty() {
        let hint = Line::from(Span::styled(
            "  No album history yet",
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(
            Paragraph::new(hint),
            Rect { y: area.y + 1, height: 1, ..area },
        );
        return;
    }

    // Row 1: horizontal album list — each album name truncated to fit.
    // Available width split roughly evenly across visible albums.
    let visible = (area.width as usize / 16).max(1);
    let mut spans: Vec<Span> = Vec::new();
    for (i, album) in albums.iter().enumerate().take(visible) {
        let label = format!(" {} ", truncate(&album.album_name, 14));
        let selected = is_active && i == selected_index;
        let style = if selected {
            Style::default().bg(accent).fg(Color::Black)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(label, style));
    }
    if area.height > 1 {
        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect { y: area.y + 1, height: 1, ..area },
        );
    }

    // Remaining rows: show selected album info.
    if area.height > 2 {
        if let Some(album) = albums.get(selected_index) {
            let info = format!("  {} — {}", album.album_name, album.artist_name);
            f.render_widget(
                Paragraph::new(Line::from(Span::raw(info))),
                Rect { y: area.y + 2, height: 1, ..area },
            );
        }
    }

    // Key hint on last row.
    if area.height > 3 && is_active {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  h/l navigate  Enter play  a add to queue",
                Style::default().fg(Color::DarkGray),
            ))),
            Rect { y: area.y + area.height.saturating_sub(1), height: 1, ..area },
        );
    }
}

/// Draw a thin selection indicator row beneath the Kitty art strip.
/// Just shows the selected album name + artist in the bottom row of the strip area.
fn render_art_strip_selection_hint(
    f: &mut Frame,
    area: Rect,
    home: &HomeState,
    accent: Color,
    cell_px: Option<(u16, u16)>,
    is_active: bool,
) {
    if area.height == 0 {
        return;
    }

    // Show the selected album name at the bottom row of the strip area.
    let (thumb_cols, _) = art_strip_thumbnail_size(cell_px, area.height.saturating_sub(1).max(1));
    let visible_count = visible_thumbnail_count(area.width, thumb_cols, 1);

    // Section header on the last row of the art strip area.
    let header_y = area.y + area.height.saturating_sub(1);

    if let Some(album) = home.recent_albums.get(home.album_selected_index) {
        let info = format!(
            "  {} — {}  [{}/{}]",
            truncate(&album.album_name, 25),
            truncate(&album.artist_name, 20),
            home.album_selected_index + 1,
            home.recent_albums.len(),
        );
        let style = if is_active {
            Style::default().fg(accent)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(info, style))),
            Rect { y: header_y, height: 1, ..area },
        );
    } else {
        let header = section_header("Recently Played Albums", is_active, accent);
        f.render_widget(
            Paragraph::new(header),
            Rect { y: header_y, height: 1, ..area },
        );
    }

    // Scroll arrows if there are more albums than visible.
    let _ = visible_count; // used for future scroll-indicator logic
}

// ── Section renderers ─────────────────────────────────────────────────────────

fn render_recent_tracks(f: &mut Frame, area: Rect, home: &HomeState, accent: Color) {
    let is_active = home.active_section == HomeSection::RecentTracks;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(section_header("Recent Tracks", is_active, accent));

    if home.recent_tracks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No play history yet \u{2014} play some tracks to see them here",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let max_items = (area.height as usize).saturating_sub(2).min(home.recent_tracks.len());
        for (i, record) in home.recent_tracks.iter().enumerate().take(max_items) {
            let rel = relative_time(record.played_at);
            let text = format!(
                "  {:>2}. {:<30} {:<20} {}",
                i + 1,
                truncate(&record.track_name, 30),
                truncate(&record.artist_name, 20),
                rel,
            );
            let selected = is_active && home.selected_index == i;
            let style = if selected {
                Style::default().bg(accent).fg(Color::Black)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text, style)));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_top_artists(f: &mut Frame, area: Rect, home: &HomeState, accent: Color) {
    let is_active = home.active_section == HomeSection::TopArtists;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(section_header("Top Artists", is_active, accent));

    if home.top_artists.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Play some music to see your top artists",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let max_items = (area.height as usize).saturating_sub(2).min(home.top_artists.len());
        for (i, (_, name, count)) in home.top_artists.iter().enumerate().take(max_items) {
            let text = format!(
                "  {:>2}. {:<35} {} plays",
                i + 1,
                truncate(name, 35),
                count,
            );
            let selected = is_active && home.selected_index == i;
            let style = if selected {
                Style::default().bg(accent).fg(Color::Black)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text, style)));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_rediscover(f: &mut Frame, area: Rect, home: &HomeState, accent: Color) {
    let is_active = home.active_section == HomeSection::Rediscover;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(section_header("Rediscover", is_active, accent));

    if home.rediscover.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Listen to more music to unlock rediscover suggestions",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let names: Vec<&str> = home.rediscover.iter().map(|(_, n)| n.as_str()).collect();
        let suggestion = format!("  Try: {}", names.join(", "));
        lines.push(Line::from(Span::raw(suggestion)));
    }

    lines.push(Line::from(Span::styled(
        "  Press r to re-roll",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(Paragraph::new(lines), area);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Truncate `s` to at most `max` characters, adding `…` if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('\u{2026}'); // …
        out
    }
}
