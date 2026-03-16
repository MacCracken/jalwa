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
