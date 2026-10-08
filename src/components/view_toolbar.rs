use crate::{
    app::ViperApp,
    storage::settings::{AlbumLayout, AlbumSort},
};
use eframe::egui;

const CANVAS: egui::Color32 = egui::Color32::from_rgb(16, 18, 23);

pub(crate) fn show(ctx: &egui::Context, app: &mut ViperApp) {
    egui::TopBottomPanel::top("view-toolbar")
        .exact_height(48.0)
        .frame(
            egui::Frame::new()
                .fill(CANVAS)
                .inner_margin(egui::Margin::symmetric(20, 8)),
        )
        .show(ctx, |ui| {
            style_sort_menu(ui);
            ui.spacing_mut().menu_margin = egui::Margin::same(8);
            ui.spacing_mut().interact_size.y = 32.0;
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let active = app.settings.album_sort;
                let mut changed = None;
                let popup_id = ui.make_persistent_id("album-sort-popup");
                let (sort_rect, sort_response) =
                    ui.allocate_exact_size(egui::vec2(152.0, 32.0), egui::Sense::click());
                paint_sort_dropdown(ui, sort_rect, &sort_response, active.label(), popup_id);
                if sort_response.clicked() {
                    ui.memory_mut(|memory| memory.toggle_popup(popup_id));
                }
                let popup_anchor = egui::Response {
                    rect: sort_response.rect.translate(egui::vec2(0.0, 6.0)),
                    ..sort_response.clone()
                };
                egui::popup_below_widget(
                    ui,
                    popup_id,
                    &popup_anchor,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |popup_ui| {
                        popup_ui.set_min_width(sort_rect.width());
                        popup_ui.spacing_mut().button_padding = egui::vec2(14.0, 5.0);
                        popup_ui.spacing_mut().item_spacing.y = 4.0;
                        for sort in AlbumSort::ALL {
                            if popup_ui
                                .selectable_label(
                                    active == sort,
                                    egui::RichText::new(sort.label()).color(egui::Color32::WHITE),
                                )
                                .clicked()
                            {
                                changed = Some(sort);
                                popup_ui.memory_mut(|memory| memory.close_popup());
                            }
                        }
                    },
                );
                if let Some(sort) = changed {
                    app.album_sort_changed(sort);
                }

                let sort_direction = view_button(
                    ui,
                    false,
                    if app.settings.sort_ascending {
                        egui_phosphor::regular::SORT_ASCENDING
                    } else {
                        egui_phosphor::regular::SORT_DESCENDING
                    },
                    if app.settings.sort_ascending {
                        "Ascending"
                    } else {
                        "Descending"
                    },
                );
                if sort_direction {
                    app.toggle_sort_direction();
                }

                let mut layout_changed = None;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for (layout, icon, label) in [
                        (
                            AlbumLayout::List,
                            egui_phosphor::regular::LIST_BULLETS,
                            "List view",
                        ),
                        (
                            AlbumLayout::Grid,
                            egui_phosphor::regular::GRID_FOUR,
                            "Grid view",
                        ),
                    ] {
                        if view_button(ui, app.settings.album_layout == layout, icon, label) {
                            layout_changed = Some(layout);
                        }
                    }
                });
                if let Some(layout) = layout_changed {
                    app.settings.album_layout = layout;
                    app.save_settings();
                }
            });
        });
}

fn paint_sort_dropdown(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    label: &str,
    popup_id: egui::Id,
) {
    let open = ui.memory(|memory| memory.is_popup_open(popup_id));
    let fill = if open || response.hovered() {
        egui::Color32::from_rgb(35, 39, 50)
    } else {
        egui::Color32::from_rgb(27, 31, 41)
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 48, 61)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(221, 225, 236),
    );
    ui.painter().text(
        egui::pos2(rect.right() - 14.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        egui_phosphor::regular::CARET_DOWN,
        egui::FontId::new(15.0, egui::FontFamily::Name("phosphor".into())),
        egui::Color32::from_rgb(190, 196, 211),
    );
}

fn view_button(ui: &mut egui::Ui, selected: bool, icon: &str, label: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 32.0), egui::Sense::click());
    paint_icon_button(ui, rect, &response, icon, selected);
    response.on_hover_text(label).clicked()
}

fn paint_icon_button(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    icon: &str,
    selected: bool,
) {
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            6.0,
            if selected {
                egui::Color32::from_rgb(48, 52, 68)
            } else {
                egui::Color32::from_rgb(35, 39, 50)
            },
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::new(18.0, egui::FontFamily::Name("phosphor".into())),
        if selected || response.hovered() {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_gray(175)
        },
    );
}

fn style_sort_menu(ui: &mut egui::Ui) {
    let fill = egui::Color32::from_rgb(27, 31, 41);
    let hover = egui::Color32::from_rgb(35, 39, 50);
    let selected = egui::Color32::from_rgb(48, 52, 68);
    let border = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 48, 61));

    {
        let visuals = ui.visuals_mut();
        visuals.window_fill = fill;
        visuals.window_stroke = border;
        visuals.menu_corner_radius = egui::CornerRadius::same(6);
        visuals.selection.bg_fill = selected;
        visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);

        let widgets = &mut visuals.widgets;
        for widget in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.bg_stroke = border;
        }
        widgets.hovered.bg_fill = hover;
        widgets.hovered.weak_bg_fill = hover;
    }
}
