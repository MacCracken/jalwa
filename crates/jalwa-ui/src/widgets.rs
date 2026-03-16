//! Ratatui widgets for the Jalwa TUI.

use jalwa_core::{MediaItem, PlaybackState, RepeatMode};
use jalwa_playback::EqSettings;
use jalwa_playback::format_duration;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use crate::app::{App, InputMode, View};

/// Render the full TUI layout.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // status bar + progress
            Constraint::Min(5),   // main view
            Constraint::Length(1), // keybind help
        ])
        .split(frame.area());

    render_status_area(frame, chunks[0], app);
    render_main_view(frame, chunks[1], app);
    render_keybinds(frame, chunks[2], app);
}

fn render_status_area(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Status bar
    let status = app.engine.status();
    let state_icon = match status.state {
        PlaybackState::Playing => "▶",
        PlaybackState::Paused => "⏸",
        PlaybackState::Stopped => "⏹",
        PlaybackState::Buffering => "…",
    };

    let now_playing = app.engine.current_path().and_then(|p| {
        app.library.library.find_by_path(p)
    });

    let title_str = match now_playing {
        Some(item) => {
            let artist = item.artist.as_deref().unwrap_or("");
            if artist.is_empty() {
                item.title.clone()
            } else {
                format!("{} - {}", artist, item.title)
            }
        }
        None => "No media loaded".to_string(),
    };

    let position = format_duration(status.position);
    let duration = status
        .duration
        .map(format_duration)
        .unwrap_or_else(|| "--:--".to_string());

    let volume = if status.muted {
        "MUTE".to_string()
    } else {
        format!("{}%", (status.volume * 100.0) as u8)
    };

    let status_line = Line::from(vec![
        Span::styled(
            format!(" {state_icon} "),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(&title_str),
        Span::raw("  "),
        Span::styled(
            format!("[{position} / {duration}]"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(volume, Style::default().fg(Color::Yellow)),
    ]);

    frame.render_widget(Paragraph::new(status_line), chunks[0]);

    // Progress bar
    let progress = status.progress().unwrap_or(0.0);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
        .ratio(progress.clamp(0.0, 1.0));
    frame.render_widget(gauge, chunks[1]);
}

fn render_main_view(frame: &mut Frame, area: Rect, app: &App) {
    match app.view {
        View::Library => render_library_view(frame, area, app),
        View::NowPlaying => render_now_playing_view(frame, area, app),
        View::Queue => render_queue_view(frame, area, app),
        View::Equalizer => render_eq_view(frame, area, app),
    }
}

fn render_library_view(frame: &mut Frame, area: Rect, app: &App) {
    let header = if !app.search_query.is_empty() {
        format!(
            "Library ({} matches) [/{}]",
            app.search_results.len(),
            app.search_query
        )
    } else {
        format!("Library ({} items)", app.library.library.items.len())
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .title(header);

    let items: Vec<ListItem> = if !app.search_query.is_empty() {
        app.search_results
            .iter()
            .enumerate()
            .filter_map(|(display_idx, &lib_idx)| {
                app.library.library.items.get(lib_idx).map(|item| {
                    make_list_item(item, display_idx)
                })
            })
            .collect()
    } else {
        app.library
            .library
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| make_list_item(item, i))
            .collect()
    };

    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(app.selected_index.min(items.len().saturating_sub(1))));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_now_playing_view(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::TOP)
        .title("Now Playing");

    let now_playing = app.engine.current_path().and_then(|p| {
        app.library.library.find_by_path(p)
    });

    let text = match now_playing {
        Some(item) => {
            let artist = item.artist.as_deref().unwrap_or("Unknown Artist");
            let album = item.album.as_deref().unwrap_or("Unknown Album");
            let duration = item
                .duration
                .map(format_duration)
                .unwrap_or_else(|| "?:??".to_string());
            let codec = item
                .audio_codec
                .map(|c| c.to_string())
                .unwrap_or_else(|| "?".to_string());

            vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    &item.title,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![Span::styled(artist, Style::default().fg(Color::White))]),
                Line::from(vec![Span::styled(album, Style::default().fg(Color::DarkGray))]),
                Line::from(""),
                Line::from(format!("Duration: {duration}  Codec: {codec}  Format: {}", item.format)),
                Line::from(format!(
                    "Plays: {}  Rating: {}",
                    item.play_count,
                    item.rating.map(|r| format!("{r}/5")).unwrap_or_else(|| "-".to_string())
                )),
            ]
        }
        None => vec![
            Line::from(""),
            Line::from("Nothing playing"),
            Line::from("Select a track from the Library view and press Enter"),
        ],
    };

    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn render_queue_view(frame: &mut Frame, area: Rect, app: &App) {
    let header = format!(
        "Queue ({} items){}{}",
        app.queue.len(),
        match app.queue.repeat_mode {
            RepeatMode::Off => "",
            RepeatMode::One => " [R1]",
            RepeatMode::All => " [RA]",
        },
        if app.queue.shuffle { " [S]" } else { "" }
    );

    let block = Block::default()
        .borders(Borders::TOP)
        .title(header);

    let items: Vec<ListItem> = app
        .queue
        .items
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let is_current = app.queue.position == Some(i);
            let prefix = if is_current { "▸ " } else { "  " };
            let title = app
                .library
                .library
                .find_by_id(*id)
                .map(|item| {
                    let artist = item.artist.as_deref().unwrap_or("Unknown");
                    format!("{}{:>3}. {} - {}", prefix, i + 1, artist, item.title)
                })
                .unwrap_or_else(|| format!("{}{:>3}. (unknown)", prefix, i + 1));
            ListItem::new(title)
        })
        .collect();

    let mut state = ListState::default();
    if !items.is_empty() && app.view == View::Queue {
        state.select(Some(app.selected_index.min(items.len().saturating_sub(1))));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_eq_view(frame: &mut Frame, area: Rect, app: &App) {
    let eq = app.engine.eq_settings();
    let norm = app.engine.normalize_enabled();

    let eq_status = if eq.enabled { "ON" } else { "OFF" };
    let norm_status = if norm { "ON" } else { "OFF" };

    let header = format!("Equalizer [{}]  Normalize [{}]", eq_status, norm_status);
    let block = Block::default()
        .borders(Borders::TOP)
        .title(header);

    let items: Vec<ListItem> = (0..10)
        .map(|i| {
            let name = EqSettings::band_name(i);
            let gain = eq.bands[i];
            let bar_width = 20;
            let center = bar_width / 2;
            let filled = ((gain / 12.0) * center as f32) as i32;

            let mut bar = vec![' '; bar_width];
            bar[center] = '|';
            if filled > 0 {
                for j in 1..=filled.min(center as i32) {
                    bar[(center as i32 + j) as usize] = '=';
                }
            } else if filled < 0 {
                for j in filled..0 {
                    bar[(center as i32 + j) as usize] = '=';
                }
            }
            let bar_str: String = bar.into_iter().collect();

            ListItem::new(format!(
                "{:>6}  [{bar_str}]  {:+.1} dB",
                name, gain,
            ))
        })
        .collect();

    let mut state = ListState::default();
    if app.view == View::Equalizer {
        state.select(Some(app.selected_index.min(9)));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_keybinds(frame: &mut Frame, area: Rect, app: &App) {
    let binds = if app.input_mode == InputMode::Search {
        "type to search | Esc:cancel | Enter:select"
    } else if app.view == View::Equalizer {
        "↑↓:band  ←→:gain  e:toggle EQ  N:normalize  Tab:view  ␣:play/pause  q:quit"
    } else {
        "␣:play/pause  /:search  ←→:seek  ↑↓:nav  Enter:play  Tab:view  a:enqueue  q:quit  +/-:vol  m:mute  n/p:next/prev  r:repeat  s:shuffle  e:EQ  N:norm"
    };
    let line = Line::from(Span::styled(binds, Style::default().fg(Color::DarkGray)));
    frame.render_widget(Paragraph::new(line), area);
}

fn make_list_item(item: &MediaItem, index: usize) -> ListItem<'static> {
    let artist = item.artist.as_deref().unwrap_or("Unknown");
    let duration = item
        .duration
        .map(format_duration)
        .unwrap_or_else(|| "?:??".to_string());
    ListItem::new(format!(
        "{:>3}. {} - {} [{}]",
        index + 1,
        artist,
        item.title,
        duration
    ))
}
