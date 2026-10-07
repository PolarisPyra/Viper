use crate::app::MusicApp;
use eframe::egui;

pub fn show_startup(ctx: &egui::Context, app: &mut MusicApp) {
    if !app.show_folder_dialog {
        return;
    }
    egui::Window::new("Welcome to musicplayer")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(350.0);
            ui.label(
                egui::RichText::new("Your music, beautifully arranged.")
                    .size(18.0)
                    .strong(),
            );
            ui.add_space(8.0);
            ui.label("Choose a music folder to scan for albums and tracks.");
            ui.add_space(12.0);
            if ui.button("Choose music folder…").clicked() {
                app.choose_folder();
                if app.settings.music_path.is_some() {
                    app.show_folder_dialog = false;
                }
            }
            if app.settings.music_path.is_some() && ui.button("Continue to library").clicked() {
                app.show_folder_dialog = false;
            }
        });
}

pub fn show_settings(ctx: &egui::Context, app: &mut MusicApp) {
    if !app.show_settings {
        return;
    }
    let mut open = app.show_settings;
    egui::Window::new("Settings")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label("Music library");
            ui.add_space(6.0);
            let folder = app
                .settings
                .music_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No music folder selected".to_owned());
            ui.label(egui::RichText::new(folder).color(egui::Color32::GRAY));
            ui.add_space(10.0);
            if ui.button("Choose music folder…").clicked() {
                app.choose_folder();
            }
        });
    app.show_settings = open;
}
