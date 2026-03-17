//! Sidebar — left nav panel for view switching.

use crate::app::{GuiApp, View};
use crate::theme;

pub fn sidebar(ui: &mut egui::Ui, app: &mut GuiApp) {
    ui.add_space(8.0);

    let entries = [
        (View::Library, "Library", "\u{1F4DA}"),
        (View::NowPlaying, "Now Playing", "\u{266B}"),
        (View::Queue, "Queue", "\u{1F4CB}"),
        (View::Equalizer, "EQ", "\u{1F3DA}"),
    ];

    for (view, label, icon) in entries {
        let is_active = app.view == view;
        let text = format!("{icon} {label}");

        let color = if is_active {
            theme::ACCENT
        } else {
            theme::TEXT_SECONDARY
        };

        let response = ui.selectable_label(
            is_active,
            egui::RichText::new(text).color(color),
        );
        if response.clicked() {
            app.view = view;
            app.selected_index = 0;
        }
    }
}
