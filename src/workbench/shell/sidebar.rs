use crate::{
    platform::persistence::settings::{Settings, StartupView},
    workbench::{Page, WorkbenchState},
};
use eframe::egui;

const COMPACT_WIDTH: f32 = 56.0;
const COMPACT_THRESHOLD: f32 = 140.0;
const MIN_EXPANDED_WIDTH: f32 = 170.0;
const MAX_WIDTH: f32 = 360.0;

#[derive(Clone, Copy)]
struct ResizeState {
    compact: bool,
    pending_save: bool,
}

pub fn show(
    ctx: &egui::Context,
    settings: &mut Settings,
    workbench: &mut WorkbenchState,
) -> Option<String> {
    let colors = crate::shared::ui::theme::colors(ctx);
    let mut error = None;
    let panel_id = egui::Id::new("library-sidebar");
    let resize_state_id = panel_id.with("mode");
    let mut resize_state = ctx
        .data(|data| data.get_temp::<ResizeState>(resize_state_id))
        .unwrap_or(ResizeState {
            compact: settings.compact_sidebar,
            pending_save: false,
        });

    // Preference changes reset the width; drag changes retain the same panel and drag handle.
    if resize_state.compact != settings.compact_sidebar {
        ctx.data_mut(|data| data.remove::<egui::containers::panel::PanelState>(panel_id));
        resize_state.compact = settings.compact_sidebar;
    }
    let panel_width = if settings.compact_sidebar {
        COMPACT_WIDTH
    } else {
        settings
            .left_panel_width
            .clamp(MIN_EXPANDED_WIDTH, MAX_WIDTH)
    };
    let output = egui::SidePanel::left(panel_id)
        .resizable(true)
        .default_width(panel_width)
        .width_range(COMPACT_WIDTH..=MAX_WIDTH)
        .frame(
            egui::Frame::new()
                .fill(colors.sidebar)
                .inner_margin(egui::Margin::symmetric(8, 10)),
        )
        .show(ctx, |ui| {
            let compact = ui.available_width() + 16.0 < COMPACT_THRESHOLD;
            nav_item(
                ui,
                "Home",
                egui_phosphor::regular::HOUSE_SIMPLE,
                Page::Home,
                compact,
                workbench,
                settings,
                &mut error,
            );
            nav_item(
                ui,
                "Albums",
                egui_phosphor::regular::FOLDERS,
                Page::Albums,
                compact,
                workbench,
                settings,
                &mut error,
            );
        });

    if !settings.left_panel_hidden {
        let width = output.response.rect.width();
        let compact = width < COMPACT_THRESHOLD;
        if compact != settings.compact_sidebar {
            settings.compact_sidebar = compact;
            resize_state.pending_save = true;
        }
        let dragging = ctx.input(|input| input.pointer.primary_down());
        if !dragging {
            // Snap to the icon rail on release while preserving the previous expanded width.
            let settled_width = if compact {
                COMPACT_WIDTH
            } else {
                width.clamp(MIN_EXPANDED_WIDTH, MAX_WIDTH)
            };
            if (settled_width - width).abs() > 0.5 {
                let mut rect = output.response.rect;
                rect.max.x = rect.min.x + settled_width;
                ctx.data_mut(|data| {
                    data.insert_persisted(panel_id, egui::containers::panel::PanelState { rect })
                });
                ctx.request_repaint();
            }
            if !compact && (settled_width - settings.left_panel_width).abs() > 0.5 {
                settings.left_panel_width = settled_width;
                resize_state.pending_save = true;
            }
            if resize_state.pending_save {
                if let Err(save_error) = settings.save() {
                    error = Some(format!("Could not save settings: {save_error}"));
                }
                resize_state.pending_save = false;
            }
        }
    }
    resize_state.compact = settings.compact_sidebar;
    ctx.data_mut(|data| data.insert_temp(resize_state_id, resize_state));

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
    compact: bool,
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
    let icon_center = if compact {
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
    let response = if compact {
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
