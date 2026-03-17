//! GuiApp — eframe::App implementation for the jalwa desktop GUI.

use jalwa_core::PlayQueue;
use jalwa_core::db::PersistentLibrary;
use jalwa_core::watcher::LibraryWatcher;
use jalwa_playback::mpris::{MprisCommand, spawn_mpris_server};
use jalwa_playback::{EngineEvent, PlaybackEngine};
use std::sync::mpsc::Receiver;
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

/// GuiApp owns all state; eframe::App::update() runs on the main thread.
pub struct GuiApp {
    pub library: PersistentLibrary,
    pub engine: PlaybackEngine,
    pub queue: PlayQueue,
    pub view: View,
    pub selected_index: usize,
    pub search_query: String,
    pub search_results: Vec<usize>,
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
        let mpris_rx = spawn_mpris_server();
        let watcher = LibraryWatcher::new(&library.library.scan_paths).ok();

        Self {
            library,
            engine,
            queue: PlayQueue::new(),
            view: View::Library,
            selected_index: 0,
            search_query: String::new(),
            search_results: Vec::new(),
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

    #[test]
    fn view_variants() {
        assert_ne!(View::Library, View::Queue);
        assert_ne!(View::NowPlaying, View::Equalizer);
    }
}
