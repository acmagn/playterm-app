use std::sync::{Arc, mpsc as std_mpsc};

use anyhow::Result;
use tokio::sync::mpsc;

use playterm_player::{PlayerCommand, PlayerEvent, spawn_player};
use playterm_subsonic::SubsonicClient;

use crate::action::{Action, Direction};
use crate::config::Config;
use crate::state::{LibraryState, PlaybackState, QueueState};

// ── Pane ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Artists,
    Albums,
    Tracks,
    Queue,
}

impl Pane {
    /// Cycle to the next pane in tab order.
    pub fn next(self) -> Self {
        match self {
            Pane::Artists => Pane::Albums,
            Pane::Albums => Pane::Tracks,
            Pane::Tracks => Pane::Queue,
            Pane::Queue => Pane::Artists,
        }
    }
}

// ── LibraryUpdate — messages sent back from background fetch tasks ─────────────

#[derive(Debug)]
pub enum LibraryUpdate {
    Artists(Result<Vec<playterm_subsonic::Artist>, String>),
    Albums {
        artist_id: String,
        result: Result<Vec<playterm_subsonic::Album>, String>,
    },
    Tracks {
        album_id: String,
        result: Result<Vec<playterm_subsonic::Song>, String>,
    },
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub active_pane: Pane,
    pub library: LibraryState,
    pub queue: QueueState,
    pub playback: PlaybackState,
    pub config: Config,
    pub subsonic: Arc<SubsonicClient>,
    /// Receives library data from background tokio tasks.
    pub library_rx: mpsc::Receiver<LibraryUpdate>,
    library_tx: mpsc::Sender<LibraryUpdate>,
    /// Send commands to the audio engine thread.
    pub player_tx: std_mpsc::Sender<PlayerCommand>,
    /// Receive events from the audio engine thread.
    pub player_rx: std_mpsc::Receiver<PlayerEvent>,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config) -> Result<Self> {
        let subsonic =
            SubsonicClient::new(&config.subsonic_url, &config.subsonic_user, &config.subsonic_pass)?;
        let (library_tx, library_rx) = mpsc::channel(64);
        let (player_tx, player_rx) = spawn_player();
        Ok(Self {
            active_pane: Pane::Artists,
            library: LibraryState::default(),
            queue: QueueState::default(),
            playback: PlaybackState::default(),
            subsonic: Arc::new(subsonic),
            library_rx,
            library_tx,
            player_tx,
            player_rx,
            config,
            should_quit: false,
        })
    }

    // ── Background fetch helpers ──────────────────────────────────────────────

    /// Spawn a task to fetch the artist list.
    pub fn fetch_artists(&self) {
        let client = self.subsonic.clone();
        let tx = self.library_tx.clone();
        tokio::spawn(async move {
            let result = playterm_subsonic::fetch_library(&client)
                .await
                .map(|lib| lib.artists)
                .map_err(|e| e.to_string());
            let _ = tx.send(LibraryUpdate::Artists(result)).await;
        });
    }

    /// Spawn a task to fetch albums for the given artist.
    pub fn fetch_albums(&self, artist_id: String) {
        let client = self.subsonic.clone();
        let tx = self.library_tx.clone();
        tokio::spawn(async move {
            let result = client
                .get_artist(&artist_id)
                .await
                .map(|a| a.album)
                .map_err(|e| e.to_string());
            let _ = tx
                .send(LibraryUpdate::Albums {
                    artist_id,
                    result,
                })
                .await;
        });
    }

    /// Spawn a task to fetch the track list for the given album.
    pub fn fetch_tracks(&self, album_id: String) {
        let client = self.subsonic.clone();
        let tx = self.library_tx.clone();
        tokio::spawn(async move {
            let result = client
                .get_album(&album_id)
                .await
                .map(|a| a.song)
                .map_err(|e| e.to_string());
            let _ = tx
                .send(LibraryUpdate::Tracks { album_id, result })
                .await;
        });
    }

    // ── Library update ingestion ──────────────────────────────────────────────

    pub fn apply_library_update(&mut self, update: LibraryUpdate) {
        use crate::state::LoadingState;
        match update {
            LibraryUpdate::Artists(result) => {
                self.library.artists = match result {
                    Ok(artists) => {
                        if self.library.selected_artist.is_none() && !artists.is_empty() {
                            self.library.selected_artist = Some(0);
                        }
                        LoadingState::Loaded(artists)
                    }
                    Err(e) => LoadingState::Error(e),
                };
            }
            LibraryUpdate::Albums { artist_id, result } => {
                self.library.albums.insert(
                    artist_id,
                    match result {
                        Ok(albums) => LoadingState::Loaded(albums),
                        Err(e) => LoadingState::Error(e),
                    },
                );
                if self.library.selected_album.is_none() {
                    self.library.selected_album = Some(0);
                }
            }
            LibraryUpdate::Tracks { album_id, result } => {
                self.library.tracks.insert(
                    album_id,
                    match result {
                        Ok(songs) => LoadingState::Loaded(songs),
                        Err(e) => LoadingState::Error(e),
                    },
                );
                if self.library.selected_track.is_none() {
                    self.library.selected_track = Some(0);
                }
            }
        }
    }

    // ── Player event ingestion ────────────────────────────────────────────────

    pub fn handle_player_event(&mut self, event: PlayerEvent) {
        match event {
            PlayerEvent::TrackStarted => {
                self.playback.paused = false;
                if let Some(song) = self.queue.current().cloned() {
                    self.playback.current_song = Some(song);
                }
            }
            PlayerEvent::Progress { elapsed, total } => {
                self.playback.elapsed = elapsed;
                self.playback.total = total;
            }
            PlayerEvent::TrackEnded => {
                if self.queue.next() {
                    self.play_current();
                } else {
                    self.playback.current_song = None;
                    self.playback.elapsed = std::time::Duration::ZERO;
                }
            }
            PlayerEvent::Error(e) => {
                // Surface the error in the status bar (future work); for now just print.
                eprintln!("player error: {e}");
            }
        }
    }

    /// Send a PlayUrl command for whatever song the queue cursor currently points at.
    fn play_current(&mut self) {
        if let Some(song) = self.queue.current().cloned() {
            let url = self.subsonic.stream_url(&song.id, 0);
            let duration = song.duration.map(|s| std::time::Duration::from_secs(u64::from(s)));
            self.playback.current_song = Some(song);
            let _ = self.player_tx.send(PlayerCommand::PlayUrl { url, duration });
        }
    }

    // ── Action dispatch ───────────────────────────────────────────────────────

    pub fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::SwitchPane(pane) => self.active_pane = pane,
            Action::Navigate(dir) => self.handle_navigate(dir),
            Action::Select => self.handle_select(),
            Action::Back => self.handle_back(),
            Action::AddToQueue => self.handle_add_to_queue(),
            Action::AddAllToQueue => self.handle_add_all_to_queue(),
            Action::PlayPause => {
                if self.playback.paused {
                    self.playback.paused = false;
                    let _ = self.player_tx.send(PlayerCommand::Resume);
                } else {
                    self.playback.paused = true;
                    let _ = self.player_tx.send(PlayerCommand::Pause);
                }
            }
            Action::NextTrack => {
                if self.queue.next() {
                    self.play_current();
                }
            }
            Action::PrevTrack => {
                if self.queue.prev() {
                    self.play_current();
                }
            }
            Action::VolumeUp | Action::VolumeDown => { /* Phase 2 */ }
            Action::None => {}
        }
    }

    fn handle_navigate(&mut self, dir: Direction) {
        use crate::state::LoadingState;
        match self.active_pane {
            Pane::Artists => {
                if let LoadingState::Loaded(artists) = &self.library.artists {
                    let len = artists.len();
                    if len == 0 {
                        return;
                    }
                    let cur = self.library.selected_artist.unwrap_or(0);
                    self.library.selected_artist = Some(match dir {
                        Direction::Up => cur.saturating_sub(1),
                        Direction::Down => (cur + 1).min(len - 1),
                        Direction::Top => 0,
                        Direction::Bottom => len - 1,
                    });
                }
            }
            Pane::Albums => {
                let artist_id = match self.library.current_artist() {
                    Some(a) => a.id.clone(),
                    None => return,
                };
                if let Some(LoadingState::Loaded(albums)) = self.library.albums.get(&artist_id) {
                    let len = albums.len();
                    if len == 0 {
                        return;
                    }
                    let cur = self.library.selected_album.unwrap_or(0);
                    self.library.selected_album = Some(match dir {
                        Direction::Up => cur.saturating_sub(1),
                        Direction::Down => (cur + 1).min(len - 1),
                        Direction::Top => 0,
                        Direction::Bottom => len - 1,
                    });
                }
            }
            Pane::Tracks => {
                let album_id = match self.library.current_album() {
                    Some(a) => a.id.clone(),
                    None => return,
                };
                if let Some(LoadingState::Loaded(songs)) = self.library.tracks.get(&album_id) {
                    let len = songs.len();
                    if len == 0 {
                        return;
                    }
                    let cur = self.library.selected_track.unwrap_or(0);
                    self.library.selected_track = Some(match dir {
                        Direction::Up => cur.saturating_sub(1),
                        Direction::Down => (cur + 1).min(len - 1),
                        Direction::Top => 0,
                        Direction::Bottom => len - 1,
                    });
                }
            }
            Pane::Queue => {
                let len = self.queue.songs.len();
                if len == 0 {
                    return;
                }
                self.queue.cursor = match dir {
                    Direction::Up => self.queue.cursor.saturating_sub(1),
                    Direction::Down => (self.queue.cursor + 1).min(len - 1),
                    Direction::Top => 0,
                    Direction::Bottom => len - 1,
                };
                // Keep scroll window in sync so the selected row is always visible.
                self.queue.scroll = self.queue.cursor;
            }
        }
    }

    fn handle_select(&mut self) {
        use crate::state::LoadingState;
        match self.active_pane {
            Pane::Artists => {
                if let Some(artist) = self.library.current_artist() {
                    let artist_id = artist.id.clone();
                    // Only fetch if not already loading/loaded
                    if !self.library.albums.contains_key(&artist_id) {
                        self.library
                            .albums
                            .insert(artist_id.clone(), LoadingState::Loading);
                        self.fetch_albums(artist_id);
                    }
                    self.library.selected_album = Some(0);
                    self.active_pane = Pane::Albums;
                }
            }
            Pane::Albums => {
                if let Some(album) = self.library.current_album() {
                    let album_id = album.id.clone();
                    if !self.library.tracks.contains_key(&album_id) {
                        self.library
                            .tracks
                            .insert(album_id.clone(), LoadingState::Loading);
                        self.fetch_tracks(album_id);
                    }
                    self.library.selected_track = Some(0);
                    self.active_pane = Pane::Tracks;
                }
            }
            Pane::Tracks => {
                // Enter on a track: add it to the queue and start playing if idle.
                let was_empty = self.queue.songs.is_empty();
                self.handle_add_to_queue();
                if was_empty && !self.queue.songs.is_empty() {
                    self.queue.cursor = self.queue.songs.len() - 1;
                    self.play_current();
                }
            }
            Pane::Queue => {}
        }
    }

    fn handle_back(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::Albums => Pane::Artists,
            Pane::Tracks => Pane::Albums,
            Pane::Queue | Pane::Artists => Pane::Artists,
        };
    }

    fn handle_add_to_queue(&mut self) {
        if let Some(song) = self.library.current_track().cloned() {
            let was_empty = self.queue.songs.is_empty();
            self.queue.push(song);
            if was_empty {
                self.queue.cursor = 0;
                self.play_current();
            }
        }
    }

    fn handle_add_all_to_queue(&mut self) {
        use crate::state::LoadingState;
        let album_id = match self.library.current_album() {
            Some(a) => a.id.clone(),
            None => return,
        };
        if let Some(LoadingState::Loaded(songs)) = self.library.tracks.get(&album_id) {
            let was_empty = self.queue.songs.is_empty();
            for song in songs.clone() {
                self.queue.push(song);
            }
            if was_empty && !self.queue.songs.is_empty() {
                self.queue.cursor = 0;
                self.play_current();
            }
        }
    }
}
