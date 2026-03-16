//! Interactive terminal UI event loop.

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use jalwa_playback::EngineEvent;

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
                    // Advance queue
                    if let Some(next_id) = app.queue.advance() {
                        if let Some(item) = app.library.library.find_by_id(next_id) {
                            let path = item.path.clone();
                            let _ = app.engine.open(&path);
                            let _ = app.engine.play();
                        }
                    }
                }
                EngineEvent::TrackChanged => {
                    // Gapless transition happened in decode thread
                    if let Some(_) = app.queue.advance() {
                        // Queue advanced
                    }
                }
                EngineEvent::NearEnd => {
                    // Prepare next track for gapless playback
                    if let Some(next_pos) = app.queue.position.map(|p| p + 1) {
                        if let Some(next_id) = app.queue.items.get(next_pos) {
                            if let Some(item) = app.library.library.find_by_id(*next_id) {
                                app.engine.prepare_next(&item.path);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Handle input
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                // Ctrl+C always quits
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
                {
                    app.running = false;
                    continue;
                }

                match app.input_mode {
                    InputMode::Search => handle_search_input(&mut app, key.code),
                    InputMode::Normal => handle_normal_input(&mut app, key.code),
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

fn handle_normal_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.running = false,

        KeyCode::Char(' ') => {
            let _ = app.engine.toggle();
        }

        KeyCode::Left => {
            let _ = app.engine.seek_relative(-10.0);
        }
        KeyCode::Right => {
            let _ = app.engine.seek_relative(10.0);
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
                        let _ = app.engine.open(&path);
                        let _ = app.engine.play();
                        // Also enqueue if queue is empty
                        if app.queue.is_empty() {
                            app.queue.enqueue(id);
                        }
                    }
                }
                View::Queue => {
                    // Jump to selected queue item
                    if app.selected_index < app.queue.len() {
                        app.queue.position = Some(app.selected_index);
                        if let Some(id) = app.queue.current() {
                            if let Some(item) = app.library.library.find_by_id(id) {
                                let path = item.path.clone();
                                let _ = app.engine.open(&path);
                                let _ = app.engine.play();
                            }
                        }
                    }
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
            // Next track
            if let Some(next_id) = app.queue.advance() {
                if let Some(item) = app.library.library.find_by_id(next_id) {
                    let path = item.path.clone();
                    let _ = app.engine.open(&path);
                    let _ = app.engine.play();
                }
            }
        }
        KeyCode::Char('p') => {
            // Previous track
            if let Some(prev_id) = app.queue.go_back() {
                if let Some(item) = app.library.library.find_by_id(prev_id) {
                    let path = item.path.clone();
                    let _ = app.engine.open(&path);
                    let _ = app.engine.play();
                }
            }
        }

        KeyCode::Char('r') => {
            app.queue.repeat_mode = app.queue.repeat_mode.cycle();
        }
        KeyCode::Char('s') => {
            app.queue.shuffle = !app.queue.shuffle;
        }

        KeyCode::Char('a') => {
            // Add selected library item to queue
            if app.view == View::Library {
                if let Some(idx) = app.selected_library_index() {
                    let id = app.library.library.items[idx].id;
                    app.queue.enqueue(id);
                }
            }
        }

        KeyCode::Char('d') => {
            // Remove selected item from queue
            if app.view == View::Queue && app.selected_index < app.queue.len() {
                app.queue.items.remove(app.selected_index);
                if app.selected_index >= app.queue.len() && app.queue.len() > 0 {
                    app.selected_index = app.queue.len() - 1;
                }
                // Fix position
                if let Some(pos) = app.queue.position {
                    if app.selected_index <= pos && pos > 0 {
                        app.queue.position = Some(pos - 1);
                    }
                }
            }
        }

        KeyCode::Char('c') => {
            // Clear queue
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
            // Keep search results visible
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
