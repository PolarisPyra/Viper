use eframe::egui;

pub(crate) fn style_dropdown(ui: &mut egui::Ui) {
    let fill = egui::Color32::from_rgb(27, 31, 41);
    let hover = egui::Color32::from_rgb(35, 39, 50);
    let selected = egui::Color32::from_rgb(48, 52, 68);
    let border = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 48, 61));

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
