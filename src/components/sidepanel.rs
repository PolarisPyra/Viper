use crate::{
    app::{MusicApp, Page},
    components::icons::{self, Icon},
    storage::settings::StartupView,
};
use eframe::egui;

const SIDEBAR: egui::Color32 = egui::Color32::from_rgb(17, 20, 28);
const TEXT: egui::Color32 = egui::Color32::from_rgb(221, 225, 236);
const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 137, 153);

pub fn show(ctx: &egui::Context, app: &mut MusicApp) {
    let panel_width = if app.settings.left_panel_hidden {
        0.0
    } else {
        app.settings.left_panel_width
    };

    let output = egui::SidePanel::left("library-sidebar")
        .resizable(true)
        .default_width(panel_width)
        .width_range(170.0..=360.0)
        .frame(
            egui::Frame::new()
                .fill(SIDEBAR)
                .inner_margin(egui::Margin::symmetric(14, 10)),
        )
        .show(ctx, |ui| {
            nav_item(ui, "Home", Icon::Home, Page::Home, app);
            nav_item(ui, "Albums", Icon::Albums, Page::Albums, app);
        });

    // Only save width if panel is not hidden
    if !app.settings.left_panel_hidden {
        let width = output.response.rect.width();
        let resizing = ctx.input(|input| input.pointer.primary_down());
        if !resizing && (width - app.settings.left_panel_width).abs() > 0.5 {
            app.settings.left_panel_width = width;
            if let Err(error) = app.settings.save() {
                app.error = Some(format!("Could not save settings: {error}"));
            }
        }
    }

    // If album details were closed and panel was hidden, show it again
    if app.settings.left_panel_hidden && !app.show_album_details {
        app.settings.left_panel_hidden = false;
        if let Err(error) = app.settings.save() {
            app.error = Some(format!("Could not save settings: {error}"));
        }
    }
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
    ui.painter().rect_filled(rect, 3.0, fill);
    let icon_tint = if selected { TEXT } else { MUTED };
    let icon_rect =
        egui::Rect::from_min_size(rect.min + egui::vec2(13.0, 11.0), egui::vec2(18.0, 18.0));
    icons::draw(ui.painter(), icon_rect, icon, icon_tint);
    ui.painter().text(
        rect.min + egui::vec2(43.0, rect.height() * 0.5),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        TEXT,
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
