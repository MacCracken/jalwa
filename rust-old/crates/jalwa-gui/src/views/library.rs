//! Library view — scrollable track list or grid with search.

use crate::app::{GuiApp, LibraryViewMode};
use crate::theme;
use jalwa_playback::format_duration;
use std::path::PathBuf;

const CELL_WIDTH: f32 = 130.0;
const ART_SIZE: f32 = 120.0;
const CELL_HEIGHT: f32 = 170.0;

pub fn library_view(ui: &mut egui::Ui, app: &mut GuiApp) {
    // Search bar + view mode toggle
    ui.horizontal(|ui| {
        ui.label("\u{1F50D}");
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.search_query)
                .hint_text("Search library...")
                .desired_width(250.0),
        );
        if response.changed() {
            app.update_search();
            app.selected_index = 0;
        }
        if !app.search_query.is_empty() {
            if ui.button("\u{2715}").clicked() {
                app.search_query.clear();
                app.search_results.clear();
                app.selected_index = 0;
            }
            ui.label(
                egui::RichText::new(format!("{} matches", app.search_results.len()))
                    .color(theme::TEXT_MUTED),
            );
        } else {
            ui.label(
                egui::RichText::new(format!("{} items", app.library.library.items.len()))
                    .color(theme::TEXT_MUTED),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let grid_selected = app.library_view_mode == LibraryViewMode::Grid;
            let list_selected = app.library_view_mode == LibraryViewMode::List;

            if ui
                .selectable_label(grid_selected, "\u{229e}")
                .on_hover_text("Grid view")
                .clicked()
            {
                app.library_view_mode = LibraryViewMode::Grid;
            }
            if ui
                .selectable_label(list_selected, "\u{2261}")
                .on_hover_text("List view")
                .clicked()
            {
                app.library_view_mode = LibraryViewMode::List;
            }
        });
    });

    ui.separator();

    // Collect visible item indices.
    // When search is active, temporarily take search_results to avoid cloning
    // (it's swapped back after rendering). When not searching, build a range vec.
    let (items, took_search) = if !app.search_query.is_empty() {
        (std::mem::take(&mut app.search_results), true)
    } else {
        ((0..app.library.library.items.len()).collect(), false)
    };

    if items.is_empty() {
        ui.centered_and_justified(|ui| {
            if app.library.library.items.is_empty() {
                ui.label(
                    egui::RichText::new("Library is empty. Use 'jalwa scan <dir>' to add files.")
                        .color(theme::TEXT_MUTED),
                );
            } else {
                ui.label(egui::RichText::new("No matches").color(theme::TEXT_MUTED));
            }
        });
        if took_search {
            app.search_results = items;
        }
        return;
    }

    match app.library_view_mode {
        LibraryViewMode::List => list_view(ui, app, &items),
        LibraryViewMode::Grid => grid_view(ui, app, &items),
    }

    // Restore search_results if we took them
    if took_search {
        app.search_results = items;
    }
}

/// List view — the original scrollable track list.
fn list_view(ui: &mut egui::Ui, app: &mut GuiApp, items: &[usize]) {
    // Pre-collect display data to avoid borrow conflicts
    let rows: Vec<(usize, usize, String, egui::Color32, uuid::Uuid)> = items
        .iter()
        .enumerate()
        .filter_map(|(display_idx, &lib_idx)| {
            let item = app.library.library.items.get(lib_idx)?;
            let is_playing = app.current_playing_id == Some(item.id);
            let is_selected = display_idx == app.selected_index;
            let artist = item.artist.as_deref().unwrap_or("Unknown");
            let duration = item
                .duration
                .map(format_duration)
                .unwrap_or_else(|| "?:??".to_string());
            let text = format!(
                "{:>3}. {artist} \u{2013} {} [{duration}]",
                display_idx + 1,
                item.title,
            );
            let color = if is_playing {
                theme::ACCENT
            } else if is_selected {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            };
            Some((display_idx, lib_idx, text, color, item.id))
        })
        .collect();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (display_idx, lib_idx, text, color, item_id) in &rows {
            let is_selected = *display_idx == app.selected_index;
            let label = egui::RichText::new(text).color(*color);
            let response = ui.selectable_label(is_selected, label);

            if response.clicked() {
                app.selected_index = *display_idx;
            }
            if response.double_clicked() {
                app.play_item(*lib_idx);
            }
            if is_selected && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                app.play_item(*lib_idx);
            }
            if is_selected && ui.input(|i| i.key_pressed(egui::Key::A)) {
                app.queue.enqueue(*item_id);
            }
        }
    });

    // Arrow key navigation (list: up/down)
    ui.input(|i| {
        if i.key_pressed(egui::Key::ArrowDown) && app.selected_index + 1 < items.len() {
            app.selected_index += 1;
        }
        if i.key_pressed(egui::Key::ArrowUp) && app.selected_index > 0 {
            app.selected_index -= 1;
        }
    });
}

/// Grid view — album art thumbnails in a wrapping grid.
fn grid_view(ui: &mut egui::Ui, app: &mut GuiApp, items: &[usize]) {
    let available_width = ui.available_width();
    let columns = ((available_width / CELL_WIDTH) as usize).max(1);

    // Pre-collect display data to avoid borrow conflicts
    let cells: Vec<(
        usize,
        usize,
        String,
        String,
        egui::Color32,
        uuid::Uuid,
        PathBuf,
    )> = items
        .iter()
        .enumerate()
        .filter_map(|(display_idx, &lib_idx)| {
            let item = app.library.library.items.get(lib_idx)?;
            let is_playing = app.current_playing_id == Some(item.id);
            let is_selected = display_idx == app.selected_index;
            let title = truncate_str(&item.title, 16);
            let artist = truncate_str(item.artist.as_deref().unwrap_or("Unknown"), 16);
            let color = if is_playing {
                theme::ACCENT
            } else if is_selected {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            };
            Some((
                display_idx,
                lib_idx,
                title,
                artist,
                color,
                item.id,
                item.path.clone(),
            ))
        })
        .collect();

    let ctx = ui.ctx().clone();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for chunk in cells.chunks(columns) {
            ui.horizontal(|ui| {
                for (display_idx, lib_idx, title, artist, color, item_id, path) in chunk {
                    let is_selected = *display_idx == app.selected_index;

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(CELL_WIDTH, CELL_HEIGHT),
                        egui::Sense::click(),
                    );

                    // Background highlight for selected cell
                    if is_selected {
                        ui.painter().rect_filled(
                            rect,
                            4.0,
                            egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
                        );
                    }

                    let art_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2((CELL_WIDTH - ART_SIZE) / 2.0, 4.0),
                        egui::vec2(ART_SIZE, ART_SIZE),
                    );

                    // Try to render album art
                    if let Some(tex) = app.art_cache.get(&ctx, *item_id, path) {
                        let uv =
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        ui.painter()
                            .image(tex.id(), art_rect, uv, egui::Color32::WHITE);
                    } else {
                        // Placeholder: dark rectangle with music note
                        ui.painter().rect_filled(
                            art_rect,
                            4.0,
                            egui::Color32::from_rgb(40, 40, 50),
                        );
                        ui.painter().text(
                            art_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "\u{266b}",
                            egui::FontId::proportional(32.0),
                            theme::TEXT_MUTED,
                        );
                    }

                    // Title text
                    let title_pos = egui::pos2(
                        rect.min.x + (CELL_WIDTH - ART_SIZE) / 2.0,
                        art_rect.max.y + 4.0,
                    );
                    ui.painter().text(
                        title_pos,
                        egui::Align2::LEFT_TOP,
                        title,
                        egui::FontId::proportional(11.0),
                        *color,
                    );

                    // Artist text
                    let artist_pos = egui::pos2(title_pos.x, title_pos.y + 14.0);
                    ui.painter().text(
                        artist_pos,
                        egui::Align2::LEFT_TOP,
                        artist,
                        egui::FontId::proportional(10.0),
                        theme::TEXT_MUTED,
                    );

                    if response.clicked() {
                        app.selected_index = *display_idx;
                    }
                    if response.double_clicked() {
                        app.play_item(*lib_idx);
                    }
                    if is_selected && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        app.play_item(*lib_idx);
                    }
                    if is_selected && ui.input(|i| i.key_pressed(egui::Key::A)) {
                        app.queue.enqueue(*item_id);
                    }
                }
            });
        }
    });

    // Arrow key navigation (grid: all four directions)
    let item_count = items.len();
    ui.input(|i| {
        if i.key_pressed(egui::Key::ArrowRight) && app.selected_index + 1 < item_count {
            app.selected_index += 1;
        }
        if i.key_pressed(egui::Key::ArrowLeft) && app.selected_index > 0 {
            app.selected_index -= 1;
        }
        if i.key_pressed(egui::Key::ArrowDown) {
            let next = app.selected_index + columns;
            if next < item_count {
                app.selected_index = next;
            }
        }
        if i.key_pressed(egui::Key::ArrowUp) && app.selected_index >= columns {
            app.selected_index -= columns;
        }
    });
}

/// Truncate a string to `max_len` characters, appending "..." if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jalwa_core::test_fixtures::make_media_item;

    fn test_app() -> crate::app::GuiApp {
        let plib = jalwa_core::db::PersistentLibrary::open(
            &std::env::temp_dir().join(format!("jalwa_gui_test_{}.db", uuid::Uuid::new_v4())),
        )
        .unwrap();
        let engine = jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());
        crate::app::GuiApp::new_headless(plib, engine)
    }

    #[test]
    fn library_view_empty() {
        let mut app = test_app();
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                library_view(ui, &mut app);
            });
        });
        assert_eq!(app.library.library.items.len(), 0);
    }

    #[test]
    fn library_view_with_items() {
        let mut app = test_app();
        app.library
            .library
            .items
            .push(make_media_item("Song A", "Artist A", 200));
        app.library
            .library
            .items
            .push(make_media_item("Song B", "Artist B", 180));
        app.library
            .library
            .items
            .push(make_media_item("Song C", "Artist C", 240));

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                library_view(ui, &mut app);
            });
        });
        assert_eq!(app.library.library.items.len(), 3);
    }

    #[test]
    fn library_view_grid_mode() {
        let mut app = test_app();
        app.library
            .library
            .items
            .push(make_media_item("Track 1", "Art", 120));
        app.library
            .library
            .items
            .push(make_media_item("Track 2", "Art", 130));
        app.library_view_mode = LibraryViewMode::Grid;

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                library_view(ui, &mut app);
            });
        });
        assert_eq!(app.library_view_mode, LibraryViewMode::Grid);
    }

    #[test]
    fn library_view_search_filters() {
        let mut app = test_app();
        app.library
            .library
            .items
            .push(make_media_item("Alpha Song", "Beatles", 200));
        app.library
            .library
            .items
            .push(make_media_item("Beta Track", "Stones", 180));
        app.library
            .library
            .items
            .push(make_media_item("Alpha Beat", "Zeppelin", 240));

        app.search_query = "alpha".to_string();
        app.update_search();
        assert_eq!(app.search_results.len(), 2);
        assert!(app.search_results.contains(&0));
        assert!(app.search_results.contains(&2));

        // Render with search active
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                library_view(ui, &mut app);
            });
        });
    }

    #[test]
    fn truncate_str_tests() {
        // Normal string within limit
        assert_eq!(truncate_str("hello", 10), "hello");

        // Empty string
        assert_eq!(truncate_str("", 5), "");

        // Exact length
        assert_eq!(truncate_str("abcde", 5), "abcde");

        // Long string gets truncated
        let result = truncate_str("abcdefghijklmnop", 5);
        assert_eq!(result, "abcd\u{2026}");

        // Unicode string
        let result = truncate_str("\u{00e9}\u{00e0}\u{00fc}\u{00f6}\u{00e4}\u{00df}", 4);
        assert_eq!(result, "\u{00e9}\u{00e0}\u{00fc}\u{2026}");
    }

    #[test]
    fn library_view_selection() {
        let mut app = test_app();
        app.library
            .library
            .items
            .push(make_media_item("Track A", "Art", 100));
        app.library
            .library
            .items
            .push(make_media_item("Track B", "Art", 200));
        app.library
            .library
            .items
            .push(make_media_item("Track C", "Art", 300));
        app.selected_index = 1;

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                library_view(ui, &mut app);
            });
        });
        assert_eq!(app.selected_index, 1);
    }
}
