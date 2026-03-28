pub mod artists;
pub mod albums;
pub mod layout;
pub mod now_playing;
pub mod queue;
pub mod status_bar;
pub mod tracks;

use ratatui::Frame;
use crate::app::App;

// ── Palette ───────────────────────────────────────────────────────────────────

use ratatui::style::Color;

pub const BG: Color = Color::Rgb(26, 26, 26);
pub const SURFACE: Color = Color::Rgb(22, 22, 22);
pub const ACCENT: Color = Color::Rgb(255, 140, 0);
pub const TEXT: Color = Color::Rgb(212, 208, 200);
pub const TEXT_MUTED: Color = Color::Rgb(90, 88, 88);
pub const BORDER: Color = Color::Rgb(37, 37, 37);
pub const BORDER_ACTIVE: Color = Color::Rgb(58, 58, 58);

// ── Top-level render ──────────────────────────────────────────────────────────

pub fn render(app: &App, frame: &mut Frame) {
    let areas = layout::build(frame.area());
    now_playing::render(app, frame, areas.now_playing);
    render_center(app, frame, areas.center);
    status_bar::render(app, frame, areas.status_bar);
}

fn render_center(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    use crate::app::Pane;
    match app.active_pane {
        Pane::Artists => artists::render(app, frame, area),
        Pane::Albums => albums::render(app, frame, area),
        Pane::Tracks => tracks::render(app, frame, area),
        Pane::Queue => queue::render(app, frame, area),
    }
}
