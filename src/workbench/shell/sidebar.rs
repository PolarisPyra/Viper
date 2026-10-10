use crate::{
    platform::persistence::settings::{Settings, StartupView},
    workbench::{Page, WorkbenchState},
};
use eframe::egui;

pub fn show(
    ctx: &egui::Context,
    settings: &mut Settings,
    workbench: &mut WorkbenchState,
) -> Option<String> {
    let colors = crate::shared::ui::theme::colors(ctx);
    let mut error = None;
    let panel_width = if settings.left_panel_hidden {
        0.0
    } else {
        settings.left_panel_width
    };

    let compact = settings.compact_sidebar;
    // Separate panel state preserves the expanded width when toggling compact mode.
    let panel = if compact {
        egui::SidePanel::left("library-sidebar-compact")
            .resizable(false)
            .exact_width(56.0)
    } else {
        egui::SidePanel::left("library-sidebar")
            .resizable(true)
            .default_width(panel_width)
            .width_range(170.0..=360.0)
    };
    let output = panel
        .frame(
            egui::Frame::new()
                .fill(colors.sidebar)
                .inner_margin(egui::Margin::symmetric(if compact { 8 } else { 14 }, 10)),
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

    // Compact mode must not overwrite the preferred expanded width.
    if !settings.left_panel_hidden && !compact {
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
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    let selected = workbench.page == page;
    let size = egui::vec2(ui.available_width(), 40.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let fill = if selected {
        colors.selected
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    let icon_tint = if selected { colors.text } else { colors.subtle };
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    let icon_center = if settings.compact_sidebar {
        rect.center()
    } else {
        rect.min + egui::vec2(22.0, rect.height() * 0.5)
    };
    ui.painter().text(
        icon_center,
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::new(18.0, egui::FontFamily::Name("phosphor".into())),
        icon_tint,
    );
    let response = if settings.compact_sidebar {
        response.on_hover_text(label)
    } else {
        ui.painter().text(
            rect.min + egui::vec2(43.0, rect.height() * 0.5),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            colors.text,
        );
        response
    };
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
                    .color(colors.subtle),
            );
        }
    });
}
