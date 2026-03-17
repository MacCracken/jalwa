//! Queue view — play queue with current-track highlight.

use crate::app::GuiApp;
use crate::theme;
use jalwa_core::RepeatMode;

pub fn queue_view(ui: &mut egui::Ui, app: &mut GuiApp) {
    // Header with repeat/shuffle status
    ui.horizontal(|ui| {
        ui.heading("Queue");
        ui.label(
            egui::RichText::new(format!("{} items", app.queue.len()))
                .color(theme::TEXT_MUTED),
        );
        ui.separator();

        // Repeat toggle
        let repeat_label = match app.queue.repeat_mode {
            RepeatMode::Off => "Repeat: Off",
            RepeatMode::One => "Repeat: One",
            RepeatMode::All => "Repeat: All",
        };
        if ui.button(repeat_label).clicked() {
            app.queue.repeat_mode = app.queue.repeat_mode.cycle();
        }

        // Shuffle toggle
        let shuffle_label = if app.queue.shuffle {
            "Shuffle: On"
        } else {
            "Shuffle: Off"
        };
        if ui.button(shuffle_label).clicked() {
            app.queue.shuffle = !app.queue.shuffle;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Clear").clicked() {
                app.queue.clear();
                app.selected_index = 0;
            }
        });
    });

    ui.separator();

    if app.queue.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Queue is empty\nPress 'a' in Library view to enqueue tracks")
                    .color(theme::TEXT_MUTED),
            );
        });
        return;
    }

    // Queue list
    let mut remove_idx = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (i, id) in app.queue.items.iter().enumerate() {
            let is_current = app.queue.position == Some(i);
            let is_selected = i == app.selected_index;

            let title = app
                .library
                .library
                .find_by_id(*id)
                .map(|item| {
                    let artist = item.artist.as_deref().unwrap_or("Unknown");
                    format!("{:>3}. {artist} \u{2013} {}", i + 1, item.title)
                })
                .unwrap_or_else(|| format!("{:>3}. (unknown)", i + 1));

            let prefix = if is_current { "\u{25B6} " } else { "  " };
            let text = format!("{prefix}{title}");

            let color = if is_current {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            };

            let response = ui.selectable_label(
                is_selected,
                egui::RichText::new(&text).color(color),
            );

            if response.clicked() {
                app.selected_index = i;
            }
            if response.double_clicked() {
                // Jump to this position in queue
                app.queue.position = Some(i);
                if let Some(id) = app.queue.current()
                    && let Some(item) = app.library.library.find_by_id(id)
                {
                    let path = item.path.clone();
                    app.current_playing_id = Some(id);
                    let _ = app.engine.open(&path);
                    let _ = app.engine.play();
                }
            }

            // Context: remove with 'd'
            if is_selected && ui.input(|i| i.key_pressed(egui::Key::D)) {
                remove_idx = Some(i);
            }
        }
    });

    if let Some(idx) = remove_idx {
        app.queue.items.remove(idx);
        if app.selected_index >= app.queue.len() && !app.queue.is_empty() {
            app.selected_index = app.queue.len() - 1;
        }
        if let Some(pos) = app.queue.position
            && idx <= pos
            && pos > 0
        {
            app.queue.position = Some(pos - 1);
        }
    }

    // Arrow key navigation
    ui.input(|i| {
        if i.key_pressed(egui::Key::ArrowDown) && app.selected_index + 1 < app.queue.len() {
            app.selected_index += 1;
        }
        if i.key_pressed(egui::Key::ArrowUp) && app.selected_index > 0 {
            app.selected_index -= 1;
        }
    });
}
