use crate::app::Pane;

#[derive(Debug, Clone)]
pub enum Direction {
    Up,
    Down,
    Top,    // g
    Bottom, // G
}

#[derive(Debug, Clone)]
pub enum Action {
    Navigate(Direction),
    Select,
    Back,
    SwitchPane(Pane),
    AddToQueue,
    AddAllToQueue,
    PlayPause,
    NextTrack,
    PrevTrack,
    VolumeUp,
    VolumeDown,
    Quit,
    None,
}
