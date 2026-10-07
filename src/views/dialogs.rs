use crate::app::MusicApp;
use eframe::egui;

pub fn show_settings(ctx: &egui::Context, app: &mut MusicApp) {
    if !app.show_settings {
        return;
    }
    let mut open = app.show_settings;
    egui::Window::new("Settings")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .fixed_size([520.0, 320.0])
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |_| {});
    app.show_settings = open;
}
