use crate::{
    app::{MusicApp, Page},
    storage::settings::StartupView,
    views::icons::{self, Icon},
};
use eframe::egui;

const SIDEBAR: egui::Color32 = egui::Color32::from_rgb(17, 20, 28);
const TEXT: egui::Color32 = egui::Color32::from_rgb(221, 225, 236);
const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 137, 153);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(155, 125, 255);

pub fn show(ctx: &egui::Context, app: &mut MusicApp) {
    let output = egui::SidePanel::left("library-sidebar")
        .resizable(true)
        .default_width(app.settings.left_panel_width)
        .width_range(170.0..=360.0)
        .frame(
            egui::Frame::new()
                .fill(SIDEBAR)
                .inner_margin(egui::Margin::symmetric(14, 18)),
        )
        .show(ctx, |ui| {
            ui.add_space(10.0);
            section_label(ui, "DISCOVER");
            ui.add_space(8.0);
            nav_item(ui, "Home", Icon::Home, Page::Home, app);
            nav_item(ui, "Albums", Icon::Albums, Page::Albums, app);

            ui.add_space(30.0);
            ui.horizontal(|ui| {
                section_label(ui, "YOUR LIBRARY");
                let album_count = app
                    .library
                    .albums
                    .iter()
                    .filter(|album| !album.tracks.is_empty())
                    .count();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{album_count}"))
                            .size(10.0)
                            .color(MUTED),
                    );
                });
            });
            ui.add_space(9.0);
            let stats_content_width = (ui.available_width() - 22.0).max(0.0);
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(24, 28, 38))
                .corner_radius(9.0)
                .inner_margin(egui::Margin::symmetric(11, 10))
                .show(ui, |ui| {
                    ui.set_min_width(stats_content_width);
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 7.0, egui::Color32::from_rgb(47, 42, 68));
                        icons::draw(ui.painter(), rect.shrink(6.0), Icon::Album, ACCENT);
                        ui.add_space(3.0);
                        ui.vertical(|ui| {
                            let album_count = app
                                .library
                                .albums
                                .iter()
                                .filter(|album| !album.tracks.is_empty())
                                .count();
                            ui.label(
                                egui::RichText::new(format!("{} tracks", app.library.tracks.len()))
                                    .size(12.0)
                                    .strong()
                                    .color(TEXT),
                            );
                            ui.label(
                                egui::RichText::new(format!("{} albums", album_count))
                                    .size(10.0)
                                    .color(MUTED),
                            );
                        });
                    });
                });
        });
    let width = output.response.rect.width();
    let resizing = ctx.input(|input| input.pointer.primary_down());
    if !resizing && (width - app.settings.left_panel_width).abs() > 0.5 {
        app.settings.left_panel_width = width;
        if let Err(error) = app.settings.save() {
            app.error = Some(format!("Could not save settings: {error}"));
        }
    }
}

fn section_label(ui: &mut egui::Ui, label: &str) {
    ui.label(egui::RichText::new(label).size(9.0).strong().color(MUTED));
}

fn nav_item(ui: &mut egui::Ui, label: &str, icon: Icon, page: Page, app: &mut MusicApp) {
    let selected = app.page == page;
    let size = egui::vec2(ui.available_width(), 40.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let fill = if selected {
        egui::Color32::from_rgb(37, 39, 53)
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    let tint = if selected { TEXT } else { MUTED };
    let icon_rect =
        egui::Rect::from_min_size(rect.min + egui::vec2(13.0, 11.0), egui::vec2(18.0, 18.0));
    icons::draw(ui.painter(), icon_rect, icon, tint);
    ui.painter().text(
        rect.min + egui::vec2(43.0, rect.height() * 0.5),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        tint,
    );
    if response.clicked_by(egui::PointerButton::Primary) {
        app.page = page;
    }
    response.context_menu(|ui| {
        if ui.button(format!("Set {label} as startup view")).clicked() {
            app.settings.startup_view = match page {
                Page::Home => StartupView::Home,
                Page::Albums => StartupView::Albums,
            };
            if let Err(error) = app.settings.save() {
                app.error = Some(format!("Could not save settings: {error}"));
            }
            ui.close_menu();
        }
        if app.settings.startup_view
            == match page {
                Page::Home => StartupView::Home,
                Page::Albums => StartupView::Albums,
            }
        {
            ui.label(
                egui::RichText::new("Current startup view")
                    .size(10.0)
                    .color(MUTED),
            );
        }
    });
}
