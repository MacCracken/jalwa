//! Interactive terminal UI event loop.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use jalwa_core::watcher::LibraryWatcher;
use jalwa_playback::EngineEvent;
use jalwa_playback::mpris::{MprisCommand, spawn_mpris_server};

use crate::app::{App, InputMode, View};
use crate::widgets;

/// Run the interactive TUI. Blocks until the user quits.
pub fn run(mut app: App) -> io::Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(50);

    // Start MPRIS D-Bus server for media key support
    let mpris_rx = spawn_mpris_server();

    // Start file watcher for library directories
    let watcher = LibraryWatcher::new(&app.library.library.scan_paths).ok();

    // Track the currently playing item ID for play count
    let mut current_playing_id: Option<uuid::Uuid> = None;

    while app.running {
        // Draw
        terminal.draw(|frame| {
            widgets::render(frame, &app);
        })?;

        // Poll engine events
        let events = app.engine.poll_events();
        for ev in &events {
            match ev {
                EngineEvent::TrackFinished => {
                    // Update play count for finished track
                    if let Some(id) = current_playing_id.take() {
                        let _ = app.library.update_play_count(id);
                    }
                    // Advance queue
                    if let Some(next_id) = app.queue.advance()
                        && let Some(item) = app.library.library.find_by_id(next_id)
                    {
                        let path = item.path.clone();
                        current_playing_id = Some(next_id);
                        let _ = app.engine.open(&path);
                        let _ = app.engine.play();
                    }
                }
                EngineEvent::TrackChanged => {
                    // Gapless transition — update play count for previous track
                    if let Some(id) = current_playing_id.take() {
                        let _ = app.library.update_play_count(id);
                    }
                    if app.queue.advance().is_some() {
                        current_playing_id = app.queue.current();
                    }
                }
                EngineEvent::NearEnd => {
                    // Prepare next track for gapless playback
                    if let Some(next_pos) = app.queue.position.map(|p| p + 1)
                        && let Some(next_id) = app.queue.items.get(next_pos)
                        && let Some(item) = app.library.library.find_by_id(*next_id)
                    {
                        app.engine.prepare_next(&item.path);
                    }
                }
                _ => {}
            }
        }

        // Poll MPRIS commands (media keys)
        while let Ok(cmd) = mpris_rx.try_recv() {
            handle_mpris_command(&mut app, &cmd, &mut current_playing_id);
        }

        // Poll file watcher events
        if let Some(ref w) = watcher {
            for ev in w.poll() {
                match ev {
                    jalwa_core::watcher::LibraryEvent::FileCreated(path) => {
                        // Auto-add new media files to library
                        if app.library.library.find_by_path(&path).is_none()
                            && let Ok(scanned) =
                                jalwa_core::scanner::scan_directory(path.parent().unwrap_or(&path))
                        {
                            for s in scanned {
                                if s.path == path {
                                    let item = jalwa_core::scanner::scanned_to_media_item(s);
                                    let _ = app.library.add_item(item);
                                    break;
                                }
                            }
                        }
                    }
                    jalwa_core::watcher::LibraryEvent::FileRemoved(path) => {
                        if let Some(item) = app.library.library.find_by_path(&path) {
                            let id = item.id;
                            let _ = app.library.remove_item(id);
                        }
                    }
                    _ => {} // FileModified — could re-scan metadata
                }
            }
        }

        // Handle keyboard input
        if event::poll(tick_rate)?
            && let Event::Key(key) = event::read()?
        {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.running = false;
                continue;
            }

            match app.input_mode {
                InputMode::Search => handle_search_input(&mut app, key.code),
                InputMode::Normal => {
                    handle_normal_input(&mut app, key.code, &mut current_playing_id)
                }
            }
        }
    }

    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Cleanup
    app.engine.stop();

    Ok(())
}

fn handle_mpris_command(app: &mut App, cmd: &MprisCommand, current_id: &mut Option<uuid::Uuid>) {
    match cmd {
        MprisCommand::PlayPause => {
            let _ = app.engine.toggle();
        }
        MprisCommand::Play => {
            let _ = app.engine.play();
        }
        MprisCommand::Pause => {
            app.engine.pause();
        }
        MprisCommand::Stop => {
            app.engine.stop();
        }
        MprisCommand::Next => {
            if let Some(next_id) = app.queue.advance()
                && let Some(item) = app.library.library.find_by_id(next_id)
            {
                let path = item.path.clone();
                *current_id = Some(next_id);
                let _ = app.engine.open(&path);
                let _ = app.engine.play();
            }
        }
        MprisCommand::Previous => {
            if let Some(prev_id) = app.queue.go_back()
                && let Some(item) = app.library.library.find_by_id(prev_id)
            {
                let path = item.path.clone();
                *current_id = Some(prev_id);
                let _ = app.engine.open(&path);
                let _ = app.engine.play();
            }
        }
        MprisCommand::Seek(offset) => {
            let _ = app.engine.seek_relative(*offset);
        }
        MprisCommand::SetVolume(vol) => {
            app.engine.set_volume(*vol as f32);
        }
    }
}

fn handle_normal_input(app: &mut App, key: KeyCode, current_id: &mut Option<uuid::Uuid>) {
    match key {
        KeyCode::Char('q') => app.running = false,

        KeyCode::Char(' ') => {
            let _ = app.engine.toggle();
        }

        KeyCode::Left => {
            if app.view == View::Equalizer {
                let band = app.selected_index.min(9);
                let gain = app.engine.eq_settings().bands[band] - 1.0;
                app.engine.set_eq_band(band, gain);
            } else {
                let _ = app.engine.seek_relative(-10.0);
            }
        }
        KeyCode::Right => {
            if app.view == View::Equalizer {
                let band = app.selected_index.min(9);
                let gain = app.engine.eq_settings().bands[band] + 1.0;
                app.engine.set_eq_band(band, gain);
            } else {
                let _ = app.engine.seek_relative(10.0);
            }
        }

        KeyCode::Up => app.select_prev(),
        KeyCode::Down => app.select_next(),

        KeyCode::Enter => {
            match app.view {
                View::Library => {
                    if let Some(idx) = app.selected_library_index() {
                        let item = &app.library.library.items[idx];
                        let path = item.path.clone();
                        let id = item.id;
                        *current_id = Some(id);
                        let _ = app.engine.open(&path);
                        let _ = app.engine.play();
                        if app.queue.is_empty() {
                            app.queue.enqueue(id);
                        }
                    }
                }
                View::Queue => {
                    if app.selected_index < app.queue.len() {
                        app.queue.position = Some(app.selected_index);
                        if let Some(id) = app.queue.current()
                            && let Some(item) = app.library.library.find_by_id(id)
                        {
                            let path = item.path.clone();
                            *current_id = Some(id);
                            let _ = app.engine.open(&path);
                            let _ = app.engine.play();
                        }
                    }
                }
                View::Equalizer => {
                    // Enter on EQ band cycles through presets
                    let names = jalwa_playback::EqSettings::preset_names();
                    let current = &app.engine.eq_settings().bands;
                    // Find next preset
                    let mut next_idx = 0;
                    for (i, name) in names.iter().enumerate() {
                        let preset = jalwa_playback::EqSettings::preset(name);
                        if preset.bands == *current {
                            next_idx = (i + 1) % names.len();
                            break;
                        }
                    }
                    let preset_name = names[next_idx];
                    let settings = if preset_name == "flat" {
                        let mut s = jalwa_playback::EqSettings::flat();
                        s.enabled = app.engine.eq_settings().enabled;
                        s
                    } else {
                        jalwa_playback::EqSettings::preset(preset_name)
                    };
                    app.engine.set_eq_settings(settings);
                }
                _ => {}
            }
        }

        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
            app.search_results.clear();
            app.view = View::Library;
        }

        KeyCode::Tab => {
            app.view = app.view.cycle();
            app.selected_index = 0;
        }

        KeyCode::Char('+') | KeyCode::Char('=') => {
            let vol = (app.engine.volume() + 0.05).min(1.0);
            app.engine.set_volume(vol);
        }
        KeyCode::Char('-') => {
            let vol = (app.engine.volume() - 0.05).max(0.0);
            app.engine.set_volume(vol);
        }
        KeyCode::Char('m') => {
            app.engine.toggle_mute();
        }

        KeyCode::Char('n') => {
            if let Some(next_id) = app.queue.advance()
                && let Some(item) = app.library.library.find_by_id(next_id)
            {
                let path = item.path.clone();
                *current_id = Some(next_id);
                let _ = app.engine.open(&path);
                let _ = app.engine.play();
            }
        }
        KeyCode::Char('p') => {
            if let Some(prev_id) = app.queue.go_back()
                && let Some(item) = app.library.library.find_by_id(prev_id)
            {
                let path = item.path.clone();
                *current_id = Some(prev_id);
                let _ = app.engine.open(&path);
                let _ = app.engine.play();
            }
        }

        KeyCode::Char('r') => {
            app.queue.repeat_mode = app.queue.repeat_mode.cycle();
        }
        KeyCode::Char('s') => {
            app.queue.shuffle = !app.queue.shuffle;
        }

        KeyCode::Char('e') => {
            if app.view == View::Equalizer {
                app.engine.toggle_eq();
            } else {
                app.view = View::Equalizer;
                app.selected_index = 0;
            }
        }

        KeyCode::Char('N') => {
            app.engine.toggle_normalize();
        }

        KeyCode::Char('a') => {
            if app.view == View::Library
                && let Some(idx) = app.selected_library_index()
            {
                let id = app.library.library.items[idx].id;
                app.queue.enqueue(id);
            }
        }

        KeyCode::Char('d') => {
            if app.view == View::Queue && app.selected_index < app.queue.len() {
                app.queue.items.remove(app.selected_index);
                if app.selected_index >= app.queue.len() && !app.queue.is_empty() {
                    app.selected_index = app.queue.len() - 1;
                }
                if let Some(pos) = app.queue.position
                    && app.selected_index <= pos
                    && pos > 0
                {
                    app.queue.position = Some(pos - 1);
                }
            }
        }

        KeyCode::Char('c') => {
            if app.view == View::Queue {
                app.queue.clear();
                app.selected_index = 0;
            }
        }

        _ => {}
    }
}

fn handle_search_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.search_query.clear();
            app.search_results.clear();
            app.selected_index = 0;
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.update_search();
            app.selected_index = 0;
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.update_search();
            app.selected_index = 0;
        }
        KeyCode::Up => app.select_prev(),
        KeyCode::Down => app.select_next(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jalwa_core::db::PersistentLibrary;
    use jalwa_core::{MediaItem, MediaType};
    use jalwa_playback::EngineConfig;
    use jalwa_playback::mpris::MprisCommand;
    use std::path::PathBuf;
    use std::time::Duration;
    use tarang_core::{AudioCodec, ContainerFormat};
    use uuid::Uuid;

    fn make_item(title: &str, artist: &str) -> MediaItem {
        MediaItem {
            id: Uuid::new_v4(),
            path: PathBuf::from(format!("/music/{title}.flac")),
            title: title.to_string(),
            artist: Some(artist.to_string()),
            album: Some("Album".to_string()),
            duration: Some(Duration::from_secs(200)),
            format: ContainerFormat::Flac,
            audio_codec: Some(AudioCodec::Flac),
            video_codec: None,
            media_type: MediaType::Audio,
            added_at: chrono::Utc::now(),
            last_played: None,
            play_count: 0,
            rating: None,
            tags: Vec::new(),
            art_mime: None,
            art_data: None,
        }
    }

    fn make_test_app() -> App {
        let tmp = std::env::temp_dir().join(format!("jalwa_tui_test_{}.db", Uuid::new_v4()));
        let plib = PersistentLibrary::open(&tmp).unwrap();
        let engine = jalwa_playback::PlaybackEngine::new(EngineConfig::default());
        App::new(plib, engine)
    }

    // ---- handle_normal_input tests ----

    #[test]
    fn normal_input_quit() {
        let mut app = make_test_app();
        let mut current_id = None;
        handle_normal_input(&mut app, KeyCode::Char('q'), &mut current_id);
        assert!(!app.running);
    }

    #[test]
    fn normal_input_tab_cycles_view() {
        let mut app = make_test_app();
        let mut current_id = None;
        assert_eq!(app.view, View::Library);
        handle_normal_input(&mut app, KeyCode::Tab, &mut current_id);
        assert_eq!(app.view, View::NowPlaying);
        handle_normal_input(&mut app, KeyCode::Tab, &mut current_id);
        assert_eq!(app.view, View::Queue);
        handle_normal_input(&mut app, KeyCode::Tab, &mut current_id);
        assert_eq!(app.view, View::Equalizer);
        handle_normal_input(&mut app, KeyCode::Tab, &mut current_id);
        assert_eq!(app.view, View::Library);
    }

    #[test]
    fn normal_input_up_down_navigation() {
        let mut app = make_test_app();
        app.library.library.add_item(make_item("A", "X"));
        app.library.library.add_item(make_item("B", "Y"));
        app.library.library.add_item(make_item("C", "Z"));
        let mut current_id = None;

        handle_normal_input(&mut app, KeyCode::Down, &mut current_id);
        assert_eq!(app.selected_index, 1);
        handle_normal_input(&mut app, KeyCode::Down, &mut current_id);
        assert_eq!(app.selected_index, 2);
        handle_normal_input(&mut app, KeyCode::Up, &mut current_id);
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn normal_input_volume_controls() {
        let mut app = make_test_app();
        let mut current_id = None;

        // Start at a level where both + and - are effective
        app.engine.set_volume(0.5);
        let mid = app.engine.volume();

        handle_normal_input(&mut app, KeyCode::Char('+'), &mut current_id);
        assert!(app.engine.volume() > mid);

        handle_normal_input(&mut app, KeyCode::Char('-'), &mut current_id);
        handle_normal_input(&mut app, KeyCode::Char('-'), &mut current_id);
        assert!(app.engine.volume() < mid);
    }

    #[test]
    fn normal_input_mute_toggle() {
        let mut app = make_test_app();
        let mut current_id = None;
        assert!(!app.engine.muted());
        handle_normal_input(&mut app, KeyCode::Char('m'), &mut current_id);
        assert!(app.engine.muted());
        handle_normal_input(&mut app, KeyCode::Char('m'), &mut current_id);
        assert!(!app.engine.muted());
    }

    #[test]
    fn normal_input_enter_search() {
        let mut app = make_test_app();
        let mut current_id = None;
        handle_normal_input(&mut app, KeyCode::Char('/'), &mut current_id);
        assert_eq!(app.input_mode, InputMode::Search);
        assert_eq!(app.view, View::Library);
    }

    #[test]
    fn normal_input_repeat_cycle() {
        let mut app = make_test_app();
        let mut current_id = None;
        assert_eq!(app.queue.repeat_mode, jalwa_core::RepeatMode::Off);
        handle_normal_input(&mut app, KeyCode::Char('r'), &mut current_id);
        assert_eq!(app.queue.repeat_mode, jalwa_core::RepeatMode::One);
        handle_normal_input(&mut app, KeyCode::Char('r'), &mut current_id);
        assert_eq!(app.queue.repeat_mode, jalwa_core::RepeatMode::All);
    }

    #[test]
    fn normal_input_shuffle_toggle() {
        let mut app = make_test_app();
        let mut current_id = None;
        assert!(!app.queue.shuffle);
        handle_normal_input(&mut app, KeyCode::Char('s'), &mut current_id);
        assert!(app.queue.shuffle);
    }

    #[test]
    fn normal_input_enqueue_from_library() {
        let mut app = make_test_app();
        let item = make_item("Song", "Artist");
        let id = item.id;
        app.library.library.add_item(item);
        let mut current_id = None;
        app.selected_index = 0;
        handle_normal_input(&mut app, KeyCode::Char('a'), &mut current_id);
        assert_eq!(app.queue.len(), 1);
        assert_eq!(app.queue.items[0], id);
    }

    #[test]
    fn normal_input_eq_toggle() {
        let mut app = make_test_app();
        let mut current_id = None;
        app.view = View::Equalizer;
        assert!(!app.engine.eq_settings().enabled);
        handle_normal_input(&mut app, KeyCode::Char('e'), &mut current_id);
        assert!(app.engine.eq_settings().enabled);
    }

    #[test]
    fn normal_input_eq_band_adjust() {
        let mut app = make_test_app();
        let mut current_id = None;
        app.view = View::Equalizer;
        app.selected_index = 3;
        let initial = app.engine.eq_settings().bands[3];
        handle_normal_input(&mut app, KeyCode::Right, &mut current_id);
        assert!(app.engine.eq_settings().bands[3] > initial);
        handle_normal_input(&mut app, KeyCode::Left, &mut current_id);
        assert!((app.engine.eq_settings().bands[3] - initial).abs() < f32::EPSILON);
    }

    #[test]
    fn normal_input_normalize_toggle() {
        let mut app = make_test_app();
        let mut current_id = None;
        assert!(!app.engine.normalize_enabled());
        handle_normal_input(&mut app, KeyCode::Char('N'), &mut current_id);
        assert!(app.engine.normalize_enabled());
    }

    #[test]
    fn normal_input_delete_from_queue() {
        let mut app = make_test_app();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        app.queue.enqueue(id1);
        app.queue.enqueue(id2);
        app.view = View::Queue;
        app.selected_index = 0;
        let mut current_id = None;
        handle_normal_input(&mut app, KeyCode::Char('d'), &mut current_id);
        assert_eq!(app.queue.len(), 1);
        assert_eq!(app.queue.items[0], id2);
    }

    #[test]
    fn normal_input_clear_queue() {
        let mut app = make_test_app();
        app.queue.enqueue(Uuid::new_v4());
        app.queue.enqueue(Uuid::new_v4());
        app.view = View::Queue;
        let mut current_id = None;
        handle_normal_input(&mut app, KeyCode::Char('c'), &mut current_id);
        assert!(app.queue.is_empty());
        assert_eq!(app.selected_index, 0);
    }

    // ---- handle_search_input tests ----

    #[test]
    fn search_input_type_query() {
        let mut app = make_test_app();
        app.input_mode = InputMode::Search;
        app.library.library.add_item(make_item("Song", "Artist"));

        handle_search_input(&mut app, KeyCode::Char('s'));
        assert_eq!(app.search_query, "s");
        handle_search_input(&mut app, KeyCode::Char('o'));
        assert_eq!(app.search_query, "so");
    }

    #[test]
    fn search_input_backspace() {
        let mut app = make_test_app();
        app.input_mode = InputMode::Search;
        app.search_query = "test".to_string();

        handle_search_input(&mut app, KeyCode::Backspace);
        assert_eq!(app.search_query, "tes");
    }

    #[test]
    fn search_input_escape() {
        let mut app = make_test_app();
        app.input_mode = InputMode::Search;
        app.search_query = "query".to_string();
        app.search_results = vec![0, 1];

        handle_search_input(&mut app, KeyCode::Esc);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.search_query.is_empty());
        assert!(app.search_results.is_empty());
    }

    #[test]
    fn search_input_enter() {
        let mut app = make_test_app();
        app.input_mode = InputMode::Search;
        app.search_query = "query".to_string();

        handle_search_input(&mut app, KeyCode::Enter);
        assert_eq!(app.input_mode, InputMode::Normal);
        // Query preserved after enter
        assert_eq!(app.search_query, "query");
    }

    #[test]
    fn search_input_navigation() {
        let mut app = make_test_app();
        app.input_mode = InputMode::Search;
        app.library.library.add_item(make_item("A", "X"));
        app.library.library.add_item(make_item("B", "X"));
        app.search_query = "X".to_string();
        app.update_search();

        handle_search_input(&mut app, KeyCode::Down);
        assert_eq!(app.selected_index, 1);
        handle_search_input(&mut app, KeyCode::Up);
        assert_eq!(app.selected_index, 0);
    }

    // ---- handle_mpris_command tests ----

    #[test]
    fn mpris_play_pause() {
        let mut app = make_test_app();
        let mut current_id = None;
        // Toggle without loaded file — engine.toggle() will error but shouldn't panic
        handle_mpris_command(&mut app, &MprisCommand::PlayPause, &mut current_id);
    }

    #[test]
    fn mpris_pause() {
        let mut app = make_test_app();
        let mut current_id = None;
        handle_mpris_command(&mut app, &MprisCommand::Pause, &mut current_id);
        // Should not panic
    }

    #[test]
    fn mpris_stop() {
        let mut app = make_test_app();
        let mut current_id = None;
        handle_mpris_command(&mut app, &MprisCommand::Stop, &mut current_id);
        assert_eq!(app.engine.state(), jalwa_core::PlaybackState::Stopped);
    }

    #[test]
    fn mpris_seek() {
        let mut app = make_test_app();
        let mut current_id = None;
        // Seek without loaded file — should gracefully handle
        handle_mpris_command(&mut app, &MprisCommand::Seek(10.0), &mut current_id);
    }

    #[test]
    fn mpris_set_volume() {
        let mut app = make_test_app();
        let mut current_id = None;
        handle_mpris_command(&mut app, &MprisCommand::SetVolume(0.7), &mut current_id);
        assert!((app.engine.volume() - 0.7).abs() < 0.01);
    }

    #[test]
    fn mpris_next_empty_queue() {
        let mut app = make_test_app();
        let mut current_id = None;
        handle_mpris_command(&mut app, &MprisCommand::Next, &mut current_id);
        // Should not panic with empty queue
        assert!(current_id.is_none());
    }

    #[test]
    fn mpris_previous_empty_queue() {
        let mut app = make_test_app();
        let mut current_id = None;
        handle_mpris_command(&mut app, &MprisCommand::Previous, &mut current_id);
        assert!(current_id.is_none());
    }
}
