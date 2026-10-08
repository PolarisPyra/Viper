use crate::app::MusicApp;
use eframe::egui;

pub fn show(ctx: &egui::Context, app: &mut MusicApp) {
    super::details::show(ctx, app);
    super::grid::show(ctx, app);
}
