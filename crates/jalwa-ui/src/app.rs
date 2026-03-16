//! Application state for the interactive TUI.

use jalwa_core::db::PersistentLibrary;
use jalwa_core::PlayQueue;
use jalwa_playback::PlaybackEngine;

/// Which view is active in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Library,
    NowPlaying,
    Queue,
}

impl View {
    pub fn cycle(self) -> Self {
        match self {
            Self::Library => Self::NowPlaying,
            Self::NowPlaying => Self::Queue,
            Self::Queue => Self::Library,
        }
    }
}

/// Input mode — normal navigation or text search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

/// Top-level application state for the TUI.
pub struct App {
    pub library: PersistentLibrary,
    pub engine: PlaybackEngine,
    pub queue: PlayQueue,
    pub view: View,
    pub selected_index: usize,
    pub search_query: String,
    pub search_results: Vec<usize>,
    pub input_mode: InputMode,
    pub running: bool,
}

impl App {
    pub fn new(library: PersistentLibrary, engine: PlaybackEngine) -> Self {
        Self {
            library,
            engine,
            queue: PlayQueue::new(),
            view: View::Library,
            selected_index: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            input_mode: InputMode::Normal,
            running: true,
        }
    }

    /// Number of items in the current view's list.
    pub fn list_len(&self) -> usize {
        match self.view {
            View::Library => {
                if self.input_mode == InputMode::Search || !self.search_query.is_empty() {
                    self.search_results.len()
                } else {
                    self.library.library.items.len()
                }
            }
            View::Queue => self.queue.len(),
            View::NowPlaying => 0,
        }
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        let len = self.list_len();
        if len > 0 && self.selected_index < len - 1 {
            self.selected_index += 1;
        }
    }

    /// Update search results based on current query.
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
                    || item.artist.as_ref().is_some_and(|a| a.to_lowercase().contains(&q))
                    || item.album.as_ref().is_some_and(|a| a.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect();
        // Clamp selection
        if self.selected_index >= self.search_results.len() && !self.search_results.is_empty() {
            self.selected_index = self.search_results.len() - 1;
        }
    }

    /// Get the library index for the currently selected item.
    pub fn selected_library_index(&self) -> Option<usize> {
        if self.view != View::Library {
            return None;
        }
        if !self.search_query.is_empty() {
            self.search_results.get(self.selected_index).copied()
        } else if self.selected_index < self.library.library.items.len() {
            Some(self.selected_index)
        } else {
            None
        }
    }
}
