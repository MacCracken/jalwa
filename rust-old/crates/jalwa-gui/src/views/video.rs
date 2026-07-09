//! Video view — renders decoded video frames as egui textures.

use crate::app::GuiApp;
use crate::theme;
use jalwa_playback::format_duration;

/// Video playback surface. Polls the engine for decoded frames and uploads
/// them as egui textures, maintaining aspect ratio.
pub fn video_view(ui: &mut egui::Ui, ctx: &egui::Context, app: &mut GuiApp) {
    // Try to get the latest frame from the engine
    if let Some(frame) = app.engine.take_video_frame() {
        let color_image =
            egui::ColorImage::from_rgb([frame.width as usize, frame.height as usize], &frame.data);

        // Always create/replace texture — egui handles GPU upload efficiently
        let tex = ctx.load_texture("jalwa-video", color_image, egui::TextureOptions::LINEAR);
        app.video_texture = Some(tex);
    }

    // Render the texture or a placeholder
    if let Some(ref tex) = app.video_texture {
        let available = ui.available_size();
        let tex_size = tex.size_vec2();

        // Maintain aspect ratio within available space
        let scale_x = available.x / tex_size.x;
        let scale_y = available.y / tex_size.y;
        let scale = scale_x.min(scale_y).min(1.0); // Don't upscale beyond native
        let display_size = egui::vec2(tex_size.x * scale, tex_size.y * scale);

        ui.vertical_centered(|ui| {
            // Center vertically
            let v_padding = (available.y - display_size.y) / 2.0;
            if v_padding > 0.0 {
                ui.add_space(v_padding);
            }

            ui.image(egui::load::SizedTexture::new(tex.id(), display_size));

            // Show position/duration below
            ui.add_space(8.0);
            let pos = format_duration(app.engine.position());
            let dur = app
                .engine
                .duration()
                .map(format_duration)
                .unwrap_or_else(|| "?:??".to_string());
            ui.label(
                egui::RichText::new(format!("{pos} / {dur}"))
                    .color(theme::TEXT_SECONDARY)
                    .size(14.0),
            );
        });
    } else {
        // No frame yet — show loading placeholder
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Loading video...")
                    .color(theme::TEXT_MUTED)
                    .size(18.0),
            );
        });
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
    fn video_view_no_frame() {
        let mut app = test_app();
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                video_view(ui, ctx, &mut app);
            });
        });
        // Should show "Loading video..." without panic
        assert!(app.video_texture.is_none());
    }

    #[test]
    fn video_view_with_texture() {
        let mut app = test_app();
        let ctx = egui::Context::default();

        // Pre-load a texture to simulate a decoded frame
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let color = egui::ColorImage::new([64, 48], vec![egui::Color32::BLACK; 64 * 48]);
            let tex = ctx.load_texture("test-video", color, egui::TextureOptions::LINEAR);
            app.video_texture = Some(tex);

            egui::CentralPanel::default().show(ctx, |ui| {
                video_view(ui, ctx, &mut app);
            });
        });

        assert!(app.video_texture.is_some());
    }
}
