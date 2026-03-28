use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct Areas {
    pub now_playing: Rect,
    pub center: Rect,
    pub status_bar: Rect,
}

/// Split the terminal area into the three fixed zones.
pub fn build(area: Rect) -> Areas {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // now-playing bar
            Constraint::Min(0),    // center pane
            Constraint::Length(1), // status bar
        ])
        .split(area);

    Areas {
        now_playing: chunks[0],
        center: chunks[1],
        status_bar: chunks[2],
    }
}
