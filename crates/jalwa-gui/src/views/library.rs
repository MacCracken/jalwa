//! Library view — scrollable track list with search.

use crate::app::GuiApp;
use crate::theme;
use jalwa_playback::format_duration;

pub fn library_view(ui: &mut egui::Ui, app: &mut GuiApp) {
    // Search bar
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
    });

    ui.separator();

    // Track list
    let items: Vec<usize> = if !app.search_query.is_empty() {
        app.search_results.clone()
    } else {
        (0..app.library.library.items.len()).collect()
    };

    if items.is_empty() {
        ui.centered_and_justified(|ui| {
            if app.library.library.items.is_empty() {
                ui.label(
                    egui::RichText::new("Library is empty. Use 'jalwa scan <dir>' to add files.")
                        .color(theme::TEXT_MUTED),
                );
            } else {
                ui.label(
                    egui::RichText::new("No matches")
                        .color(theme::TEXT_MUTED),
                );
            }
        });
        return;
    }

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

    // Arrow key navigation
    ui.input(|i| {
        if i.key_pressed(egui::Key::ArrowDown) && app.selected_index + 1 < items.len() {
            app.selected_index += 1;
        }
        if i.key_pressed(egui::Key::ArrowUp) && app.selected_index > 0 {
            app.selected_index -= 1;
        }
    });
}
