use super::general;
use crate::app::MusicApp;
use eframe::egui;

const PANEL: egui::Color32 = egui::Color32::from_rgb(22, 25, 34);
const BORDER: egui::Color32 = egui::Color32::from_rgb(51, 57, 72);
const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 235, 244);
const MUTED: egui::Color32 = egui::Color32::from_rgb(148, 155, 173);

pub fn show(ctx: &egui::Context, app: &mut MusicApp) {
    if !app.show_preferences {
        return;
    }

    let screen_size = ctx.screen_rect().size();
    let margin = 24.0_f32.min(screen_size.x * 0.05).min(screen_size.y * 0.05);
    let modal_size = egui::vec2(
        (screen_size.x - margin * 2.0).min(760.0).max(0.0),
        (screen_size.y - margin * 2.0).max(0.0),
    );
    let content_size = (modal_size - egui::vec2(36.0, 36.0)).max(egui::Vec2::ZERO);
    let mut close_requested = false;

    let response = egui::Modal::new(egui::Id::new("preferences-modal"))
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .stroke(egui::Stroke::new(1.0_f32, BORDER))
                .corner_radius(14.0)
                .inner_margin(egui::Margin::symmetric(18, 18)),
        )
        .backdrop_color(egui::Color32::from_black_alpha(165))
        .show(ctx, |ui| {
            ui.set_min_size(content_size);
            ui.set_max_size(content_size);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Preferences")
                            .size(23.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(
                        egui::RichText::new("Manage your library and startup experience")
                            .size(12.0)
                            .color(MUTED),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                    if response.hovered() {
                        ui.painter()
                            .rect_filled(rect, 8.0, egui::Color32::from_rgb(47, 52, 67));
                    }
                    crate::components::icons::draw(
                        ui.painter(),
                        rect.shrink(8.0),
                        crate::components::icons::Icon::Close,
                        if response.hovered() { TEXT } else { MUTED },
                    );
                    if response.on_hover_text("Close preferences").clicked() {
                        close_requested = true;
                    }
                });
            });

            ui.add_space(18.0);
            category_tabs(ui, app);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            egui::ScrollArea::vertical()
                .id_salt("preferences-content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match app.preferences_category.unwrap_or(Category::General) {
                        Category::General => general::show(ui, app),
                        Category::Audio
                        | Category::Connections
                        | Category::Theme
                        | Category::View
                        | Category::Services
                        | Category::About => {}
                    }
                });
        });

    if close_requested || response.should_close() {
        app.show_preferences = false;
    }
}

fn category_tabs(ui: &mut egui::Ui, app: &mut MusicApp) {
    let mut selected = app.preferences_category.unwrap_or(Category::General);
    egui::ScrollArea::horizontal()
        .id_salt("preferences-category-tabs")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for category in Category::ALL {
                    let is_selected = selected == category;
                    let response = ui
                        .scope(|ui| {
                            let widgets = &mut ui.visuals_mut().widgets;
                            widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                            widgets.inactive.bg_stroke = egui::Stroke::NONE;
                            widgets.hovered.bg_fill = egui::Color32::from_rgb(37, 40, 53);
                            widgets.hovered.bg_stroke = egui::Stroke::NONE;
                            widgets.active.bg_fill = egui::Color32::from_rgb(37, 40, 53);
                            widgets.active.bg_stroke = egui::Stroke::NONE;
                            ui.spacing_mut().button_padding = egui::vec2(11.0, 7.0);
                            ui.selectable_label(
                                is_selected,
                                egui::RichText::new(category.label())
                                    .size(12.0)
                                    .color(if is_selected { TEXT } else { MUTED }),
                            )
                        })
                        .inner;
                    if response.clicked() {
                        selected = category;
                    }
                }
            });
        });
    app.preferences_category = Some(selected);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Category {
    General,
    Audio,
    Connections,
    Theme,
    View,
    Services,
    About,
}

impl Category {
    const ALL: [Self; 7] = [
        Self::General,
        Self::Audio,
        Self::Connections,
        Self::Theme,
        Self::View,
        Self::Services,
        Self::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Audio => "Audio",
            Self::Connections => "Connections",
            Self::Theme => "Theme",
            Self::View => "View",
            Self::Services => "Services",
            Self::About => "About",
        }
    }
}
