//! jalwa-gui — Desktop GUI for the Jalwa media player
//!
//! Uses egui/eframe with wgpu backend. Runs alongside (not replacing) the TUI.

mod app;
mod art_cache;
mod theme;
mod views;

use jalwa_core::db::PersistentLibrary;
use jalwa_playback::PlaybackEngine;

pub use app::GuiApp;

/// Launch the GUI window. Blocks until closed.
pub fn run(library: PersistentLibrary, engine: PlaybackEngine) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("jalwa")
            .with_inner_size([1024.0, 700.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native("jalwa", options, Box::new(|cc| {
        theme::apply(&cc.egui_ctx);
        Ok(Box::new(GuiApp::new(library, engine, cc)))
    }))
}
