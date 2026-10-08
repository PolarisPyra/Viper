use crate::app::MusicApp;
use crate::ui::preferences;
use eframe::egui;

pub fn show_preferences(ctx: &egui::Context, app: &mut MusicApp) {
    preferences::show(ctx, app);
}
