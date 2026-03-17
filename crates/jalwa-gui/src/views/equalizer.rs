//! Equalizer view — 10-band sliders + presets + normalize toggle.

use crate::app::GuiApp;
use crate::theme;
use jalwa_playback::EqSettings;

pub fn equalizer_view(ui: &mut egui::Ui, app: &mut GuiApp) {
    ui.horizontal(|ui| {
        ui.heading("Equalizer");

        // EQ toggle
        let eq_enabled = app.engine.eq_settings().enabled;
        let eq_label = if eq_enabled { "EQ: ON" } else { "EQ: OFF" };
        if ui.button(eq_label).clicked() {
            app.engine.toggle_eq();
        }

        ui.separator();

        // Normalize toggle
        let norm = app.engine.normalize_enabled();
        let norm_label = if norm { "Normalize: ON" } else { "Normalize: OFF" };
        if ui.button(norm_label).clicked() {
            app.engine.toggle_normalize();
        }

        ui.separator();

        // Preset selector
        let names = EqSettings::preset_names();
        if ui.button("Preset \u{25BC}").clicked() {
            // Cycle through presets
            let current = &app.engine.eq_settings().bands;
            let mut next_idx = 0;
            for (i, name) in names.iter().enumerate() {
                let preset = EqSettings::preset(name);
                if preset.bands == *current {
                    next_idx = (i + 1) % names.len();
                    break;
                }
            }
            let preset_name = names[next_idx];
            let enabled = app.engine.eq_settings().enabled;
            let mut settings = if preset_name == "flat" {
                EqSettings::flat()
            } else {
                EqSettings::preset(preset_name)
            };
            settings.enabled = enabled;
            app.engine.set_eq_settings(settings);
        }

        // Show current preset name
        let current_bands = &app.engine.eq_settings().bands;
        let preset_name = names
            .iter()
            .find(|&&name| {
                let p = EqSettings::preset(name);
                p.bands == *current_bands
            })
            .copied()
            .unwrap_or("custom");
        ui.label(
            egui::RichText::new(preset_name).color(theme::TEXT_MUTED),
        );
    });

    ui.separator();

    // Band sliders — vertical layout
    ui.columns(10, |columns| {
        for (band, col) in columns.iter_mut().enumerate() {
            let name = EqSettings::band_name(band);
            let mut gain = app.engine.eq_settings().bands[band];

            col.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!("{gain:+.1}"))
                        .color(theme::TEXT_SECONDARY)
                        .small(),
                );

                let slider = ui.add(
                    egui::Slider::new(&mut gain, -12.0..=12.0)
                        .vertical()
                        .show_value(false)
                        .custom_formatter(|v, _| format!("{v:+.1}")),
                );
                if slider.changed() {
                    app.engine.set_eq_band(band, gain);
                }

                ui.label(
                    egui::RichText::new(name)
                        .color(theme::TEXT_MUTED)
                        .small(),
                );
            });
        }
    });
}
