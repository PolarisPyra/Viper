use crate::app::MusicApp;
use crate::views::settings;
use eframe::egui;

pub fn show_settings(ctx: &egui::Context, app: &mut MusicApp) {
    settings::show(ctx, app);
}
