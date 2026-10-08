use crate::app::ViperApp;
use eframe::egui;

const PANEL: egui::Color32 = egui::Color32::from_rgb(22, 25, 34);
const PANEL_BORDER: egui::Color32 = egui::Color32::from_rgb(51, 57, 72);
const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 235, 244);
const MUTED: egui::Color32 = egui::Color32::from_rgb(148, 155, 173);

pub fn show(ctx: &egui::Context, app: &mut ViperApp) {
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
                .stroke(egui::Stroke::new(1.0_f32, PANEL_BORDER))
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
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::X,
                        egui::FontId::new(16.0, egui::FontFamily::Name("phosphor".into())),
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
                        Category::General => show_general(ui, app),
                        Category::Connections => show_connections(ui, app),
                        Category::Audio
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

fn category_tabs(ui: &mut egui::Ui, app: &mut ViperApp) {
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

const BUTTON: egui::Color32 = egui::Color32::from_rgb(37, 40, 53);
const BORDER: egui::Color32 = egui::Color32::from_rgb(67, 72, 91);
const ROW_HOVERED: egui::Color32 = egui::Color32::from_rgb(32, 36, 47);
const ROW_SELECTED: egui::Color32 = egui::Color32::from_rgb(34, 38, 49);

fn show_general(ui: &mut egui::Ui, app: &mut crate::app::ViperApp) {
    ui.set_min_width(420.0);
    ui.add_space(8.0);
    section_title(ui, "Music library");
    ui.label("Choose the folder that contains your music.");
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.set_width(ui.available_width());
        if let Some(path) = &app.settings.music_path {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(path.display().to_string())
                        .size(12.0)
                        .color(ui.visuals().widgets.inactive.text_color()),
                )
                .sense(egui::Sense::hover())
                .truncate(),
            );
        } else {
            ui.label(
                egui::RichText::new("No music folder selected")
                    .size(12.0)
                    .color(ui.visuals().widgets.inactive.text_color()),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = egui::Button::new(
                egui::RichText::new("Choose folder")
                    .size(12.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(BUTTON)
            .stroke(egui::Stroke::new(1.0_f32, BORDER))
            .corner_radius(7.0)
            .min_size(egui::vec2(112.0, 34.0));
            if ui.add(button).clicked() {
                app.choose_folder();
            }
        });
    });

    ui.add_space(28.0);
    section_title(ui, "Startup");
    ui.label("Choose which page opens when the player starts.");
    ui.add_space(12.0);

    let startup_view = app.settings.startup_view;
    let mut new_startup_view = startup_view;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        for (page, label) in [
            (crate::storage::settings::StartupView::Home, "Home"),
            (crate::storage::settings::StartupView::Albums, "Albums"),
        ] {
            let is_startup = new_startup_view == page;
            let (rect, response) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::click());
            let fill = if response.hovered() {
                ROW_HOVERED
            } else if is_startup {
                ROW_SELECTED
            } else {
                BUTTON
            };
            ui.painter().rect(
                rect,
                7.0,
                fill,
                egui::Stroke::new(1.0_f32, BORDER),
                egui::StrokeKind::Inside,
            );

            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.left() + 18.0, rect.center().y),
                egui::vec2(16.0, 16.0),
            );
            ui.painter().text(
                icon_rect.center(),
                egui::Align2::CENTER_CENTER,
                if is_startup {
                    egui_phosphor::regular::RADIO_BUTTON
                } else {
                    egui_phosphor::regular::CIRCLE
                },
                egui::FontId::new(16.0, egui::FontFamily::Name("phosphor".into())),
                if is_startup {
                    egui::Color32::from_gray(220)
                } else {
                    egui::Color32::from_gray(130)
                },
            );
            ui.painter().text(
                egui::pos2(rect.left() + 36.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(13.0),
                egui::Color32::from_gray(if is_startup { 235 } else { 195 }),
            );
            if is_startup {
                ui.painter().text(
                    egui::pos2(rect.right() - 12.0, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    "Current startup page",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_gray(145),
                );
            }
            if response.clicked() {
                new_startup_view = page;
            }
        }
    });
    if new_startup_view != startup_view {
        app.settings.startup_view = new_startup_view;
        if let Err(error) = app.settings.save() {
            app.error = Some(format!("Could not save settings: {error}"));
        }
    }
}

fn show_connections(ui: &mut egui::Ui, app: &mut crate::app::ViperApp) {
    ui.set_min_width(420.0);
    ui.add_space(8.0);
    section_title(ui, "Discord Rich Presence");
    ui.label("Show the song currently playing in your Discord profile.");
    ui.add_space(10.0);
    ui.label("Discord Application ID");
    ui.add(
        egui::TextEdit::singleline(&mut app.discord_application_id_draft)
            .hint_text("Enter your Discord application ID")
            .desired_width(ui.available_width()),
    );
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Create an application in the Discord Developer Portal and copy its Application ID here.")
            .size(11.0)
            .color(MUTED),
    );
    ui.add_space(12.0);
    if ui.button("Save Discord settings").clicked() {
        app.settings.discord_application_id = match app.discord_application_id_draft.trim() {
            "" => None,
            id => Some(id.to_owned()),
        };
        app.save_settings();
    }
}

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(
        eframe::egui::RichText::new(title)
            .size(15.0)
            .strong()
            .color(ui.visuals().widgets.active.text_color()),
    );
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(8.0);
}
