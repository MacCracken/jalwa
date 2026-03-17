//! Application state for the interactive TUI.

use jalwa_core::PlayQueue;
use jalwa_core::db::PersistentLibrary;
use jalwa_playback::PlaybackEngine;

/// Which view is active in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Library,
    NowPlaying,
    Queue,
    Equalizer,
}

impl View {
    pub fn cycle(self) -> Self {
        match self {
            Self::Library => Self::NowPlaying,
            Self::NowPlaying => Self::Queue,
            Self::Queue => Self::Equalizer,
            Self::Equalizer => Self::Library,
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
            View::Equalizer => 10, // 10 EQ bands
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

#[cfg(test)]
mod tests {
    use super::*;
    use jalwa_core::db::PersistentLibrary;
    use jalwa_core::MediaItem;
    use jalwa_playback::EngineConfig;
    use uuid::Uuid;

    fn make_test_app() -> App {
        let tmp = std::env::temp_dir().join(format!("jalwa_app_test_{}.db", Uuid::new_v4()));
        let plib = PersistentLibrary::open(&tmp).unwrap();
        let engine = PlaybackEngine::new(EngineConfig::default());
        App::new(plib, engine)
    }

    fn make_item(title: &str, artist: &str) -> MediaItem {
        jalwa_core::test_fixtures::make_media_item(title, artist, 200)
    }

    #[test]
    fn view_cycle() {
        assert_eq!(View::Library.cycle(), View::NowPlaying);
        assert_eq!(View::NowPlaying.cycle(), View::Queue);
        assert_eq!(View::Queue.cycle(), View::Equalizer);
        assert_eq!(View::Equalizer.cycle(), View::Library);
    }

    #[test]
    fn app_new_defaults() {
        let app = make_test_app();
        assert_eq!(app.view, View::Library);
        assert_eq!(app.selected_index, 0);
        assert!(app.search_query.is_empty());
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.running);
    }

    #[test]
    fn list_len_empty_library() {
        let app = make_test_app();
        assert_eq!(app.list_len(), 0);
    }

    #[test]
    fn list_len_with_items() {
        let mut app = make_test_app();
        app.library.library.add_item(make_item("A", "X"));
        app.library.library.add_item(make_item("B", "Y"));
        assert_eq!(app.list_len(), 2);
    }

    #[test]
    fn list_len_queue_view() {
        let mut app = make_test_app();
        app.view = View::Queue;
        app.queue.enqueue(Uuid::new_v4());
        assert_eq!(app.list_len(), 1);
    }

    #[test]
    fn list_len_now_playing_always_zero() {
        let mut app = make_test_app();
        app.view = View::NowPlaying;
        assert_eq!(app.list_len(), 0);
    }

    #[test]
    fn select_prev_at_zero() {
        let mut app = make_test_app();
        app.select_prev();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn select_next_clamps() {
        let mut app = make_test_app();
        app.library.library.add_item(make_item("A", "X"));
        app.library.library.add_item(make_item("B", "Y"));
        app.select_next();
        assert_eq!(app.selected_index, 1);
        app.select_next(); // should clamp
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn select_nav_up_down() {
        let mut app = make_test_app();
        app.library.library.add_item(make_item("A", "X"));
        app.library.library.add_item(make_item("B", "Y"));
        app.library.library.add_item(make_item("C", "Z"));
        app.select_next();
        app.select_next();
        assert_eq!(app.selected_index, 2);
        app.select_prev();
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn update_search_empty_query() {
        let mut app = make_test_app();
        app.search_results = vec![0, 1];
        app.update_search();
        assert!(app.search_results.is_empty());
    }

    #[test]
    fn update_search_finds_matches() {
        let mut app = make_test_app();
        app.library
            .library
            .add_item(make_item("Bohemian Rhapsody", "Queen"));
        app.library
            .library
            .add_item(make_item("Time", "Pink Floyd"));
        app.search_query = "queen".to_string();
        app.update_search();
        assert_eq!(app.search_results.len(), 1);
        assert_eq!(app.search_results[0], 0);
    }

    #[test]
    fn update_search_clamps_selection() {
        let mut app = make_test_app();
        app.library.library.add_item(make_item("A", "X"));
        app.selected_index = 5;
        app.search_query = "A".to_string();
        app.update_search();
        assert_eq!(app.selected_index, 0); // clamped to results len - 1
    }

    #[test]
    fn list_len_with_search() {
        let mut app = make_test_app();
        app.library.library.add_item(make_item("Unique", "X"));
        app.library.library.add_item(make_item("Other", "Y"));
        app.search_query = "Unique".to_string();
        app.update_search();
        assert_eq!(app.list_len(), 1);
    }

    #[test]
    fn selected_library_index_normal() {
        let mut app = make_test_app();
        app.library.library.add_item(make_item("A", "X"));
        app.selected_index = 0;
        assert_eq!(app.selected_library_index(), Some(0));
    }

    #[test]
    fn selected_library_index_out_of_range() {
        let app = make_test_app();
        assert_eq!(app.selected_library_index(), None);
    }

    #[test]
    fn selected_library_index_search() {
        let mut app = make_test_app();
        app.library.library.add_item(make_item("Alpha", "X"));
        app.library.library.add_item(make_item("Beta", "Y"));
        app.search_query = "Beta".to_string();
        app.update_search();
        app.selected_index = 0;
        assert_eq!(app.selected_library_index(), Some(1)); // maps through search_results
    }

    #[test]
    fn selected_library_index_wrong_view() {
        let mut app = make_test_app();
        app.view = View::Queue;
        assert_eq!(app.selected_library_index(), None);
    }
}
