//! GuiApp — eframe::App implementation for the jalwa desktop GUI.

use jalwa_core::PlayQueue;
use jalwa_core::db::PersistentLibrary;
use jalwa_core::watcher::LibraryWatcher;
use jalwa_playback::mpris::{MprisCommand, spawn_mpris_server};
use jalwa_playback::{EngineEvent, PlaybackEngine};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::art_cache::ArtCache;
use crate::views;

/// Active view in the main panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Library,
    NowPlaying,
    Queue,
    Equalizer,
}

/// Library view layout mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryViewMode {
    List,
    Grid,
}

/// GuiApp owns all state; eframe::App::update() runs on the main thread.
pub struct GuiApp {
    pub library: PersistentLibrary,
    pub engine: PlaybackEngine,
    pub queue: PlayQueue,
    pub view: View,
    pub selected_index: usize,
    pub search_query: String,
    pub search_results: Vec<usize>,
    pub library_view_mode: LibraryViewMode,
    pub art_cache: ArtCache,
    pub current_playing_id: Option<Uuid>,

    mpris_rx: Receiver<MprisCommand>,
    _watcher: Option<LibraryWatcher>,
}

impl GuiApp {
    pub fn new(
        library: PersistentLibrary,
        engine: PlaybackEngine,
        _cc: &eframe::CreationContext<'_>,
    ) -> Self {
        let mpris_state = Arc::new(Mutex::new(jalwa_core::PlaybackState::Stopped));
        let mpris_rx = spawn_mpris_server(mpris_state);
        let watcher = LibraryWatcher::new(&library.library.scan_paths).ok();

        Self {
            library,
            engine,
            queue: PlayQueue::new(),
            view: View::Library,
            selected_index: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            library_view_mode: LibraryViewMode::List,
            art_cache: ArtCache::new(),
            current_playing_id: None,
            mpris_rx,
            _watcher: watcher,
        }
    }

    /// Update search results from current query.
    pub fn update_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            return;
        }
        let q = self.search_query.to_lowercase();
        self.search_results = self
            .library
            .library
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.title.to_lowercase().contains(&q)
                    || item
                        .artist
                        .as_ref()
                        .is_some_and(|a| a.to_lowercase().contains(&q))
                    || item
                        .album
                        .as_ref()
                        .is_some_and(|a| a.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect();
        if self.selected_index >= self.search_results.len() && !self.search_results.is_empty() {
            self.selected_index = self.search_results.len() - 1;
        }
    }

    /// Number of items in the current view's list.
    pub fn list_len(&self) -> usize {
        match self.view {
            View::Library => {
                if !self.search_query.is_empty() {
                    self.search_results.len()
                } else {
                    self.library.library.items.len()
                }
            }
            View::Queue => self.queue.len(),
            View::NowPlaying => 0,
            View::Equalizer => 10,
        }
    }

    fn poll_engine(&mut self) {
        let events = self.engine.poll_events();
        for ev in &events {
            match ev {
                EngineEvent::TrackFinished => {
                    if let Some(id) = self.current_playing_id.take() {
                        let _ = self.library.update_play_count(id);
                    }
                    if let Some(next_id) = self.queue.advance()
                        && let Some(item) = self.library.library.find_by_id(next_id)
                    {
                        let path = item.path.clone();
                        self.current_playing_id = Some(next_id);
                        let _ = self.engine.open(&path);
                        let _ = self.engine.play();
                    }
                }
                EngineEvent::TrackChanged => {
                    if let Some(id) = self.current_playing_id.take() {
                        let _ = self.library.update_play_count(id);
                    }
                    if self.queue.advance().is_some() {
                        self.current_playing_id = self.queue.current();
                    }
                }
                EngineEvent::NearEnd => {
                    if let Some(next_pos) = self.queue.position.map(|p| p + 1)
                        && let Some(next_id) = self.queue.items.get(next_pos)
                        && let Some(item) = self.library.library.find_by_id(*next_id)
                    {
                        self.engine.prepare_next(&item.path);
                    }
                }
                _ => {}
            }
        }
    }

    fn poll_mpris(&mut self) {
        while let Ok(cmd) = self.mpris_rx.try_recv() {
            match cmd {
                MprisCommand::PlayPause => {
                    let _ = self.engine.toggle();
                }
                MprisCommand::Play => {
                    let _ = self.engine.play();
                }
                MprisCommand::Pause => {
                    self.engine.pause();
                }
                MprisCommand::Stop => {
                    self.engine.stop();
                }
                MprisCommand::Next => {
                    if let Some(next_id) = self.queue.advance()
                        && let Some(item) = self.library.library.find_by_id(next_id)
                    {
                        let path = item.path.clone();
                        self.current_playing_id = Some(next_id);
                        let _ = self.engine.open(&path);
                        let _ = self.engine.play();
                    }
                }
                MprisCommand::Previous => {
                    if let Some(prev_id) = self.queue.go_back()
                        && let Some(item) = self.library.library.find_by_id(prev_id)
                    {
                        let path = item.path.clone();
                        self.current_playing_id = Some(prev_id);
                        let _ = self.engine.open(&path);
                        let _ = self.engine.play();
                    }
                }
                MprisCommand::Seek(offset) => {
                    let _ = self.engine.seek_relative(offset);
                }
                MprisCommand::SetVolume(vol) => {
                    self.engine.set_volume(vol as f32);
                }
            }
        }
    }

    /// Create a headless GuiApp for testing (no MPRIS, no watcher).
    #[cfg(test)]
    pub fn new_headless(library: PersistentLibrary, engine: PlaybackEngine) -> Self {
        let (_, mpris_rx) = std::sync::mpsc::channel();
        Self {
            library,
            engine,
            queue: PlayQueue::new(),
            view: View::Library,
            selected_index: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            library_view_mode: LibraryViewMode::List,
            art_cache: ArtCache::new(),
            current_playing_id: None,
            mpris_rx,
            _watcher: None,
        }
    }

    /// Play a specific library item by index.
    pub fn play_item(&mut self, lib_index: usize) {
        if let Some(item) = self.library.library.items.get(lib_index) {
            let path = item.path.clone();
            let id = item.id;
            self.current_playing_id = Some(id);
            let _ = self.engine.open(&path);
            let _ = self.engine.play();
            if self.queue.is_empty() {
                self.queue.enqueue(id);
            }
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_engine();
        self.poll_mpris();

        // Request repaint at ~30fps while playing
        if self.engine.state() == jalwa_core::PlaybackState::Playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }

        // Keyboard shortcuts (global)
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.key_pressed(egui::Key::Space) {
                    let _ = self.engine.toggle();
                }
            });
        }

        views::transport::top_bar(ctx, self);

        egui::TopBottomPanel::bottom("progress").show(ctx, |ui| {
            views::transport::progress_bar(ui, self);
        });

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(100.0)
            .show(ctx, |ui| {
                views::sidebar::sidebar(ui, self);
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::Library => views::library::library_view(ui, self),
            View::NowPlaying => views::now_playing::now_playing_view(ui, ctx, self),
            View::Queue => views::queue::queue_view(ui, self),
            View::Equalizer => views::equalizer::equalizer_view(ui, self),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jalwa_core::test_fixtures;

    fn test_db() -> (std::path::PathBuf, PersistentLibrary) {
        let path = std::env::temp_dir().join(format!("jalwa_gui_test_{}.db", uuid::Uuid::new_v4()));
        let plib = PersistentLibrary::open(&path).unwrap();
        (path, plib)
    }

    fn test_app() -> (std::path::PathBuf, GuiApp) {
        let (path, plib) = test_db();
        let engine = PlaybackEngine::new(jalwa_playback::EngineConfig::default());
        let app = GuiApp::new_headless(plib, engine);
        (path, app)
    }

    #[test]
    fn view_variants() {
        assert_ne!(View::Library, View::Queue);
        assert_ne!(View::NowPlaying, View::Equalizer);
    }

    #[test]
    fn headless_library_view_empty() {
        let (path, mut app) = test_app();
        let ctx = egui::Context::default();
        ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                views::library::library_view(ui, &mut app);
            });
        });
        assert_eq!(app.list_len(), 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn headless_now_playing_view() {
        let (path, mut app) = test_app();
        app.view = View::NowPlaying;
        let ctx = egui::Context::default();
        ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                views::now_playing::now_playing_view(ui, ctx, &mut app);
            });
        });
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn headless_queue_view() {
        let (path, mut app) = test_app();
        app.view = View::Queue;
        let ctx = egui::Context::default();
        ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                views::queue::queue_view(ui, &mut app);
            });
        });
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn headless_equalizer_view() {
        let (path, mut app) = test_app();
        app.view = View::Equalizer;
        let ctx = egui::Context::default();
        ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                views::equalizer::equalizer_view(ui, &mut app);
            });
        });
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn update_search_empty_query() {
        let (path, mut app) = test_app();
        app.search_query = String::new();
        app.update_search();
        assert!(app.search_results.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn update_search_filters() {
        let (path, mut app) = test_app();
        let item1 = test_fixtures::make_media_item("Moonlight Sonata", "Beethoven", 300);
        let item2 = test_fixtures::make_media_item("Clair de Lune", "Debussy", 280);
        let item3 = test_fixtures::make_media_item("Moonriver", "Mancini", 180);
        app.library.library.items.push(item1);
        app.library.library.items.push(item2);
        app.library.library.items.push(item3);

        app.search_query = "moon".to_string();
        app.update_search();
        assert_eq!(app.search_results.len(), 2);
        assert!(app.search_results.contains(&0));
        assert!(app.search_results.contains(&2));

        app.search_query = "debussy".to_string();
        app.update_search();
        assert_eq!(app.search_results.len(), 1);
        assert!(app.search_results.contains(&1));

        app.search_query = "nonexistent".to_string();
        app.update_search();
        assert!(app.search_results.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn list_len_library() {
        let (path, mut app) = test_app();
        assert_eq!(app.list_len(), 0);

        let item1 = test_fixtures::make_media_item("Track A", "Artist", 120);
        let item2 = test_fixtures::make_media_item("Track B", "Artist", 150);
        app.library.library.items.push(item1);
        app.library.library.items.push(item2);
        assert_eq!(app.list_len(), 2);

        app.search_query = "Track A".to_string();
        app.update_search();
        assert_eq!(app.list_len(), 1);

        app.view = View::Queue;
        assert_eq!(app.list_len(), 0);

        app.view = View::NowPlaying;
        assert_eq!(app.list_len(), 0);

        app.view = View::Equalizer;
        assert_eq!(app.list_len(), 10);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn play_item_valid_index() {
        let (path, mut app) = test_app();
        let wav_data = test_fixtures::make_test_wav(44100, 44100);
        let wav_path =
            std::env::temp_dir().join(format!("jalwa_test_{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&wav_path, &wav_data).unwrap();

        let mut item = test_fixtures::make_media_item("Test WAV", "Test", 1);
        item.path = wav_path.clone();
        let id = item.id;
        app.library.library.items.push(item);

        app.play_item(0);
        assert_eq!(app.current_playing_id, Some(id));
        assert!(!app.queue.is_empty());

        let _ = std::fs::remove_file(&wav_path);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn play_item_invalid_index() {
        let (path, mut app) = test_app();
        app.play_item(999);
        assert!(app.current_playing_id.is_none());
        assert!(app.queue.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn view_switching() {
        let (path, mut app) = test_app();
        let item = test_fixtures::make_media_item("Track", "Artist", 120);
        app.library.library.items.push(item);
        app.selected_index = 0;

        for view in [
            View::Queue,
            View::NowPlaying,
            View::Equalizer,
            View::Library,
        ] {
            app.view = view;
            app.selected_index = 0;
            assert_eq!(app.view, view);
            assert_eq!(app.selected_index, 0);
        }

        let _ = std::fs::remove_file(&path);
    }
}
