use crate::{
    platform::persistence::settings::{Settings, StartupView},
    workbench::{Page, WorkbenchState},
};
use eframe::egui;

const SIDEBAR: egui::Color32 = egui::Color32::from_rgb(17, 20, 28);
const TEXT: egui::Color32 = egui::Color32::from_rgb(221, 225, 236);
const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 137, 153);

pub fn show(
    ctx: &egui::Context,
    settings: &mut Settings,
    workbench: &mut WorkbenchState,
) -> Option<String> {
    let mut error = None;
    let panel_width = if settings.left_panel_hidden {
        0.0
    } else {
        settings.left_panel_width
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
            nav_item(
                ui,
                "Home",
                egui_phosphor::regular::HOUSE_SIMPLE,
                Page::Home,
                workbench,
                settings,
                &mut error,
            );
            nav_item(
                ui,
                "Albums",
                egui_phosphor::regular::FOLDERS,
                Page::Albums,
                workbench,
                settings,
                &mut error,
            );
        });

    // Only save width if panel is not hidden
    if !settings.left_panel_hidden {
        let width = output.response.rect.width();
        let resizing = ctx.input(|input| input.pointer.primary_down());
        if !resizing && (width - settings.left_panel_width).abs() > 0.5 {
            settings.left_panel_width = width;
            if let Err(save_error) = settings.save() {
                error = Some(format!("Could not save settings: {save_error}"));
            }
        }
    }

    // If album details were closed and panel was hidden, show it again
    if settings.left_panel_hidden && !workbench.show_album_details {
        settings.left_panel_hidden = false;
        if let Err(save_error) = settings.save() {
            error = Some(format!("Could not save settings: {save_error}"));
        }
    }
    error
}

fn nav_item(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    page: Page,
    workbench: &mut WorkbenchState,
    settings: &mut Settings,
    error: &mut Option<String>,
) {
    let selected = workbench.page == page;
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
    ui.painter().text(
        icon_rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::new(18.0, egui::FontFamily::Name("phosphor".into())),
        icon_tint,
    );
    ui.painter().text(
        rect.min + egui::vec2(43.0, rect.height() * 0.5),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        TEXT,
    );
    if response.clicked_by(egui::PointerButton::Primary) {
        workbench.page = page;
    }
    response.context_menu(|ui| {
        if ui.button(format!("Set {label} as startup view")).clicked() {
            settings.startup_view = match page {
                Page::Home => StartupView::Home,
                Page::Albums => StartupView::Albums,
            };
            if let Err(save_error) = settings.save() {
                *error = Some(format!("Could not save settings: {save_error}"));
            }
            ui.close_menu();
        }
        if settings.startup_view
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
