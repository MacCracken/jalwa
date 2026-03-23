//! Sidebar — left nav panel for view switching.

use crate::app::{GuiApp, View};
use crate::theme;

pub fn sidebar(ui: &mut egui::Ui, app: &mut GuiApp) {
    ui.add_space(8.0);

    let mut entries = vec![
        (View::Library, "Library", "\u{1F4DA}"),
        (View::NowPlaying, "Now Playing", "\u{266B}"),
        (View::Queue, "Queue", "\u{1F4CB}"),
        (View::Equalizer, "EQ", "\u{1F3DA}"),
    ];

    // Show Video entry when a video is loaded
    if app.engine.is_video() {
        entries.insert(2, (View::Video, "Video", "\u{1F3AC}"));
    }

    entries.push((View::Devices, "Devices", "\u{1F50C}"));

    for (view, label, icon) in entries {
        let is_active = app.view == view;
        let text = format!("{icon} {label}");

        let color = if is_active {
            theme::ACCENT
        } else {
            theme::TEXT_SECONDARY
        };

        let response = ui.selectable_label(is_active, egui::RichText::new(text).color(color));
        if response.clicked() {
            app.view = view;
            app.selected_index = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> crate::app::GuiApp {
        let plib = jalwa_core::db::PersistentLibrary::open(
            &std::env::temp_dir().join(format!("jalwa_gui_test_{}.db", uuid::Uuid::new_v4())),
        )
        .unwrap();
        let engine = jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());
        crate::app::GuiApp::new_headless(plib, engine)
    }

    #[test]
    fn sidebar_renders() {
        let mut app = test_app();
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                sidebar(ui, &mut app);
            });
        });
    }

    #[test]
    fn sidebar_highlights_current() {
        let mut app = test_app();

        // Default view is Library
        assert_eq!(app.view, View::Library);

        // Switch to Queue and render
        app.view = View::Queue;
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                sidebar(ui, &mut app);
            });
        });
        assert_eq!(app.view, View::Queue);

        // Switch to NowPlaying and render
        app.view = View::NowPlaying;
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                sidebar(ui, &mut app);
            });
        });
        assert_eq!(app.view, View::NowPlaying);
    }
}
