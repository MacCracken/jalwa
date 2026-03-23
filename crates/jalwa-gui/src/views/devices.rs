//! Devices view — shows detected USB storage and optical drives.

use crate::app::GuiApp;

pub fn devices_view(ui: &mut egui::Ui, app: &mut GuiApp) {
    ui.heading("Devices");
    ui.separator();

    #[cfg(feature = "yukti")]
    {
        // Show notifications
        if !app.hardware_notifications.is_empty() {
            for note in &app.hardware_notifications {
                ui.label(note);
            }
            ui.separator();
        }

        if let Some(ref hw) = app.hardware {
            let usb = hw.mounted_usb_devices();
            let optical = hw.optical_drives();

            if usb.is_empty() && optical.is_empty() {
                ui.label("No media devices detected.");
            } else {
                if !usb.is_empty() {
                    ui.label("USB Storage");
                    ui.separator();
                    for dev in &usb {
                        let mount = dev
                            .mount_point
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "not mounted".into());
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "{} — {} ({})",
                                dev.display_name(),
                                dev.size_display(),
                                mount,
                            ));
                        });
                    }
                    ui.add_space(8.0);
                }

                if !optical.is_empty() {
                    ui.label("Optical Drives");
                    ui.separator();
                    for dev in &optical {
                        ui.label(format!("{} — {}", dev.display_name(), dev.state));
                    }
                }
            }
        } else {
            ui.label("Hardware monitoring not available.");
        }
    }

    #[cfg(not(feature = "yukti"))]
    {
        ui.label("Device detection requires the 'yukti' feature.");
    }
}
