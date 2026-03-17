//! Transport bar — play/pause/stop, volume, track info, and progress seek.

use crate::app::GuiApp;
use crate::theme;
use jalwa_core::PlaybackState;
use jalwa_playback::format_duration;

/// Top bar with transport controls and track info.
pub fn top_bar(ctx: &egui::Context, app: &mut GuiApp) {
    egui::TopBottomPanel::top("transport").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            // Previous
            if ui.button("\u{23EE}").clicked() {
                if let Some(prev_id) = app.queue.go_back()
                    && let Some(item) = app.library.library.find_by_id(prev_id)
                {
                    let path = item.path.clone();
                    app.current_playing_id = Some(prev_id);
                    let _ = app.engine.open(&path);
                    let _ = app.engine.play();
                }
            }

            // Play/Pause
            let play_label = match app.engine.state() {
                PlaybackState::Playing => "\u{23F8}",
                _ => "\u{25B6}",
            };
            if ui.button(play_label).clicked() {
                let _ = app.engine.toggle();
            }

            // Stop
            if ui.button("\u{23F9}").clicked() {
                app.engine.stop();
            }

            // Next
            if ui.button("\u{23ED}").clicked() {
                if let Some(next_id) = app.queue.advance()
                    && let Some(item) = app.library.library.find_by_id(next_id)
                {
                    let path = item.path.clone();
                    app.current_playing_id = Some(next_id);
                    let _ = app.engine.open(&path);
                    let _ = app.engine.play();
                }
            }

            ui.separator();

            // Track info
            let now_playing = app
                .engine
                .current_path()
                .and_then(|p| app.library.library.find_by_path(p));

            let title_text = match now_playing {
                Some(item) => {
                    let artist = item.artist.as_deref().unwrap_or("");
                    if artist.is_empty() {
                        item.title.clone()
                    } else {
                        format!("{artist} \u{2013} {}", item.title)
                    }
                }
                None => "No media loaded".to_string(),
            };

            ui.label(
                egui::RichText::new(title_text)
                    .color(theme::TEXT_PRIMARY)
                    .strong(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Volume
                let mut vol = app.engine.volume();
                let vol_pct = format!("{}%", (vol * 100.0) as u8);
                ui.label(
                    egui::RichText::new(vol_pct).color(theme::TEXT_SECONDARY),
                );

                let vol_slider = ui.add(
                    egui::Slider::new(&mut vol, 0.0..=1.0)
                        .show_value(false)
                        .custom_formatter(|_, _| String::new()),
                );
                if vol_slider.changed() {
                    app.engine.set_volume(vol);
                }

                // Mute button
                let mute_icon = if app.engine.muted() {
                    "\u{1F507}"
                } else {
                    "\u{1F50A}"
                };
                if ui.button(mute_icon).clicked() {
                    app.engine.toggle_mute();
                }
            });
        });
    });
}

/// Bottom progress/seek bar.
pub fn progress_bar(ui: &mut egui::Ui, app: &mut GuiApp) {
    let status = app.engine.status();
    let position = status.position;
    let duration = status.duration.unwrap_or(std::time::Duration::ZERO);

    ui.horizontal(|ui| {
        let pos_str = format_duration(position);
        let dur_str = if duration.is_zero() {
            "--:--".to_string()
        } else {
            format_duration(duration)
        };

        ui.label(
            egui::RichText::new(&pos_str)
                .color(theme::TEXT_SECONDARY)
                .monospace(),
        );

        let progress = if duration.is_zero() {
            0.0
        } else {
            position.as_secs_f32() / duration.as_secs_f32()
        };

        let mut seek_val = progress;
        let slider = ui.add(
            egui::Slider::new(&mut seek_val, 0.0..=1.0)
                .show_value(false)
                .custom_formatter(|_, _| String::new()),
        );
        if slider.drag_stopped() && !duration.is_zero() {
            let target = std::time::Duration::from_secs_f32(seek_val * duration.as_secs_f32());
            let _ = app.engine.seek(target);
        }

        ui.label(
            egui::RichText::new(&dur_str)
                .color(theme::TEXT_SECONDARY)
                .monospace(),
        );
    });
}
