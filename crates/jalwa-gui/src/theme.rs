//! Dark theme for the jalwa GUI.

use egui::{Color32, CornerRadius, Stroke, Style, Visuals};

/// AGNOS-standard dark palette.
pub const BG_DARK: Color32 = Color32::from_rgb(24, 24, 28);
pub const BG_PANEL: Color32 = Color32::from_rgb(32, 32, 38);
pub const BG_WIDGET: Color32 = Color32::from_rgb(44, 44, 52);
pub const ACCENT: Color32 = Color32::from_rgb(0, 188, 212); // cyan-ish
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0, 131, 148);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(230, 230, 235);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 170);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 110);
#[allow(dead_code)]
pub const HIGHLIGHT: Color32 = Color32::from_rgb(0, 188, 212);
#[allow(dead_code)]
pub const ERROR: Color32 = Color32::from_rgb(220, 60, 60);

pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();

    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_DARK;
    visuals.extreme_bg_color = BG_DARK;
    visuals.faint_bg_color = BG_WIDGET;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.selection.bg_fill = ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_fill = BG_WIDGET;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.inactive.bg_fill = BG_WIDGET;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 65);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    style.visuals = visuals;
    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_apply() {
        let ctx = egui::Context::default();
        apply(&ctx);
        // Verify the theme was applied by checking panel fill
        let style = ctx.style();
        assert_eq!(style.visuals.panel_fill, BG_PANEL);
        assert_eq!(style.visuals.window_fill, BG_DARK);
    }

    #[test]
    fn theme_constants() {
        // Verify color constants have expected non-zero RGB values
        assert_ne!(BG_DARK, Color32::TRANSPARENT);
        assert_ne!(BG_PANEL, Color32::TRANSPARENT);
        assert_ne!(BG_WIDGET, Color32::TRANSPARENT);
        assert_ne!(ACCENT, Color32::TRANSPARENT);
        assert_ne!(ACCENT_DIM, Color32::TRANSPARENT);
        assert_ne!(TEXT_PRIMARY, Color32::TRANSPARENT);
        assert_ne!(TEXT_SECONDARY, Color32::TRANSPARENT);
        assert_ne!(TEXT_MUTED, Color32::TRANSPARENT);
        assert_ne!(HIGHLIGHT, Color32::TRANSPARENT);
        assert_ne!(ERROR, Color32::TRANSPARENT);

        // All colors should be fully opaque (alpha = 255)
        assert_eq!(BG_DARK.a(), 255);
        assert_eq!(ACCENT.a(), 255);
        assert_eq!(TEXT_PRIMARY.a(), 255);
        assert_eq!(ERROR.a(), 255);
    }
}
