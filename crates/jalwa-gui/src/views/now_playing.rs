//! Now Playing view — album art + track details.

use crate::app::GuiApp;
use crate::theme;
use jalwa_playback::format_duration;

pub fn now_playing_view(ui: &mut egui::Ui, ctx: &egui::Context, app: &mut GuiApp) {
    let now_playing = app
        .engine
        .current_path()
        .and_then(|p| app.library.library.find_by_path(p))
        .cloned();

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
