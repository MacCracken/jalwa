//! Now Playing view — album art + track details.

use crate::app::GuiApp;
use crate::theme;
use jalwa_playback::format_duration;

pub fn now_playing_view(ui: &mut egui::Ui, ctx: &egui::Context, app: &mut GuiApp) {
    let now_playing = app
        .engine
        .current_path()
        .and_then(|p| app.library.library.find_by_path(p));

    let Some(item) = now_playing else {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new(
                    "Nothing playing\nSelect a track from the Library and press Enter",
                )
                .color(theme::TEXT_MUTED),
            );
        });
        return;
    };

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);

        // Album art
        let art_tex = app.art_cache.get(ctx, item.id, &item.path);
        if let Some(tex) = art_tex {
            let max_size = 300.0;
            let size = tex.size_vec2();
            let scale = (max_size / size.x.max(size.y)).min(1.0);
            let display_size = egui::vec2(size.x * scale, size.y * scale);
            ui.image(egui::load::SizedTexture::new(tex.id(), display_size));
        } else {
            // Placeholder
            let (rect, _) = ui.allocate_exact_size(egui::vec2(200.0, 200.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 8.0, theme::BG_WIDGET);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{266B}",
                egui::FontId::proportional(64.0),
                theme::TEXT_MUTED,
            );
        }

        ui.add_space(16.0);

        // Title
        ui.label(
            egui::RichText::new(&item.title)
                .size(22.0)
                .color(theme::TEXT_PRIMARY)
                .strong(),
        );

        // Artist
        let artist = item.artist.as_deref().unwrap_or("Unknown Artist");
        ui.label(
            egui::RichText::new(artist)
                .size(16.0)
                .color(theme::TEXT_SECONDARY),
        );

        // Album
        let album = item.album.as_deref().unwrap_or("Unknown Album");
        ui.label(
            egui::RichText::new(album)
                .size(14.0)
                .color(theme::TEXT_MUTED),
        );

        ui.add_space(12.0);

        // Details
        let duration = item
            .duration
            .map(format_duration)
            .unwrap_or_else(|| "?:??".to_string());
        let codec = item
            .audio_codec
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string());

        ui.label(
            egui::RichText::new(format!(
                "Duration: {duration}  \u{2022}  Codec: {codec}  \u{2022}  Format: {}",
                item.format
            ))
            .color(theme::TEXT_MUTED),
        );

        ui.label(
            egui::RichText::new(format!(
                "Plays: {}  \u{2022}  Rating: {}",
                item.play_count,
                item.rating
                    .map(|r| format!("{r}/5"))
                    .unwrap_or_else(|| "-".to_string())
            ))
            .color(theme::TEXT_MUTED),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use jalwa_core::test_fixtures::{make_media_item, make_test_wav};

    fn test_app() -> crate::app::GuiApp {
        let plib = jalwa_core::db::PersistentLibrary::open(
            &std::env::temp_dir().join(format!("jalwa_gui_test_{}.db", uuid::Uuid::new_v4())),
        )
        .unwrap();
        let engine = jalwa_playback::PlaybackEngine::new(jalwa_playback::EngineConfig::default());
        crate::app::GuiApp::new_headless(plib, engine)
    }

    #[test]
    fn now_playing_no_track() {
        let mut app = test_app();
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                now_playing_view(ui, ctx, &mut app);
            });
        });
        // Should render "Nothing playing" without panic
    }

    #[test]
    fn now_playing_with_track() {
        let mut app = test_app();

        // Create a real wav on disk so the engine can open it
        let wav_data = make_test_wav(44100, 44100);
        let wav_path =
            std::env::temp_dir().join(format!("jalwa_np_test_{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&wav_path, &wav_data).unwrap();

        let mut item = make_media_item("Now Playing Track", "Test Artist", 1);
        item.path = wav_path.clone();
        let id = item.id;
        app.library.library.items.push(item);

        // Open and play so engine has a current path
        let _ = app.engine.open(&wav_path);
        let _ = app.engine.play();
        app.current_playing_id = Some(id);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                now_playing_view(ui, ctx, &mut app);
            });
        });

        let _ = std::fs::remove_file(&wav_path);
    }

    #[test]
    fn now_playing_with_metadata() {
        let mut app = test_app();

        let wav_data = make_test_wav(44100, 44100);
        let wav_path =
            std::env::temp_dir().join(format!("jalwa_np_meta_{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&wav_path, &wav_data).unwrap();

        let mut item = make_media_item("Rich Track", "Best Artist", 245);
        item.path = wav_path.clone();
        item.album = Some("Great Album".to_string());
        item.rating = Some(4);
        item.play_count = 7;
        let id = item.id;
        app.library.library.items.push(item);

        let _ = app.engine.open(&wav_path);
        let _ = app.engine.play();
        app.current_playing_id = Some(id);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                now_playing_view(ui, ctx, &mut app);
            });
        });

        let _ = std::fs::remove_file(&wav_path);
    }

    #[test]
    fn now_playing_progress_display() {
        let mut app = test_app();

        let wav_data = make_test_wav(88200, 44100); // 2 seconds
        let wav_path =
            std::env::temp_dir().join(format!("jalwa_np_prog_{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&wav_path, &wav_data).unwrap();

        let mut item = make_media_item("Progress Track", "Prog Artist", 120);
        item.path = wav_path.clone();
        item.duration = Some(std::time::Duration::from_secs(120));
        item.audio_codec = Some(jalwa_core::AudioCodec::Flac);
        item.artist = None; // exercises "Unknown Artist" branch
        item.album = None; // exercises "Unknown Album" branch
        item.rating = None; // exercises "-" rating branch
        let id = item.id;
        app.library.library.items.push(item);

        let _ = app.engine.open(&wav_path);
        let _ = app.engine.play();
        app.current_playing_id = Some(id);

        let ctx = egui::Context::default();
        // Render twice to exercise with some state
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    now_playing_view(ui, ctx, &mut app);
                });
            });
        }

        let _ = std::fs::remove_file(&wav_path);
    }
}
