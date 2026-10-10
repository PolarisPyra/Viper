mod state;
pub(crate) use state::PreferencesState;

use crate::{platform::persistence::settings::Settings, workbench::WorkbenchState};
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreferencesAction {
    ChooseMusicFolder,
    ConnectSmbShare,
    SaveSettings,
}

pub(crate) fn show(
    ctx: &egui::Context,
    workbench: &mut WorkbenchState,
    preferences: &mut PreferencesState,
    settings: &mut Settings,
    error: &mut Option<String>,
) -> Option<PreferencesAction> {
    let colors = crate::shared::ui::theme::colors(ctx);
    if !workbench.show_preferences {
        return None;
    }
    let mut action = None;

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
                .fill(colors.panel)
                .stroke(egui::Stroke::new(1.0_f32, colors.border))
                .corner_radius(14.0)
                .inner_margin(egui::Margin::symmetric(18, 18)),
        )
        .backdrop_color(colors.backdrop)
        .show(ctx, |ui| {
            ui.set_min_size(content_size);
            ui.set_max_size(content_size);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Preferences")
                            .size(23.0)
                            .strong()
                            .color(colors.text),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Manage your library, connections, and startup experience",
                        )
                        .size(12.0)
                        .color(colors.muted),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                    if response.hovered() {
                        ui.painter().rect_filled(rect, 8.0, colors.hover);
                    }
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        egui_phosphor::regular::X,
                        egui::FontId::new(16.0, egui::FontFamily::Name("phosphor".into())),
                        if response.hovered() {
                            colors.text
                        } else {
                            colors.muted
                        },
                    );
                    if response.on_hover_text("Close preferences").clicked() {
                        close_requested = true;
                    }
                });
            });

            ui.add_space(18.0);
            category_tabs(ui, preferences);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            egui::ScrollArea::vertical()
                .id_salt("preferences-content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match preferences.category.unwrap_or(Category::General) {
                        Category::General => show_general(ui, settings, error, &mut action),
                        Category::Connections => {
                            show_connections(ui, preferences, settings, error, &mut action)
                        }
                        Category::View => show_view(ui, settings, &mut action),
                        Category::Theme => show_theme(ui, settings, &mut action),
                        Category::Audio | Category::Services | Category::About => {}
                    }
                });
        });

    if close_requested || response.should_close() {
        workbench.show_preferences = false;
    }
    action
}

fn category_tabs(ui: &mut egui::Ui, preferences: &mut PreferencesState) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    let mut selected = preferences.category.unwrap_or(Category::General);
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
                            widgets.hovered.bg_fill = colors.surface;
                            widgets.hovered.bg_stroke = egui::Stroke::NONE;
                            widgets.active.bg_fill = colors.surface;
                            widgets.active.bg_stroke = egui::Stroke::NONE;
                            ui.spacing_mut().button_padding = egui::vec2(11.0, 7.0);
                            ui.selectable_label(
                                is_selected,
                                egui::RichText::new(category.label()).size(12.0).color(
                                    if is_selected {
                                        colors.text
                                    } else {
                                        colors.muted
                                    },
                                ),
                            )
                        })
                        .inner;
                    if response.clicked() {
                        selected = category;
                    }
                }
            });
        });
    preferences.category = Some(selected);
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

fn show_general(
    ui: &mut egui::Ui,
    settings: &mut Settings,
    error: &mut Option<String>,
    action: &mut Option<PreferencesAction>,
) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    ui.set_min_width(420.0);
    ui.add_space(8.0);
    section_title(ui, "Music library");
    ui.label("Choose the folder that contains your music.");
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.set_width(ui.available_width());
        if let Some(path) = &settings.music_path {
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
                    .color(colors.text),
            )
            .fill(colors.surface)
            .stroke(egui::Stroke::new(1.0_f32, colors.border))
            .corner_radius(7.0)
            .min_size(egui::vec2(112.0, 34.0));
            if ui.add(button).clicked() {
                *action = Some(PreferencesAction::ChooseMusicFolder);
            }
        });
    });

    ui.add_space(28.0);
    section_title(ui, "Startup");
    ui.label("Choose which page opens when the player starts.");
    ui.add_space(12.0);

    let startup_view = settings.startup_view;
    let mut new_startup_view = startup_view;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        for (page, label) in [
            (
                crate::platform::persistence::settings::StartupView::Home,
                "Home",
            ),
            (
                crate::platform::persistence::settings::StartupView::Albums,
                "Albums",
            ),
        ] {
            if choice_row(ui, label, new_startup_view == page, "Current startup page").clicked() {
                new_startup_view = page;
            }
        }
    });
    if new_startup_view != startup_view {
        settings.startup_view = new_startup_view;
        if let Err(save_error) = settings.save() {
            *error = Some(format!("Could not save settings: {save_error}"));
        }
    }
}

fn show_theme(ui: &mut egui::Ui, settings: &mut Settings, action: &mut Option<PreferencesAction>) {
    ui.add_space(8.0);
    section_title(ui, "Themes");
    ui.label("Choose a theme for the player.");
    ui.add_space(12.0);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        for theme in crate::shared::ui::theme::ThemeId::ALL {
            if choice_row(ui, theme.label(), settings.theme == theme, "Current theme").clicked()
                && settings.theme != theme
            {
                settings.theme = theme;
                crate::shared::ui::theme::apply(ui.ctx(), theme);
                *action = Some(PreferencesAction::SaveSettings);
            }
        }
    });
}

fn choice_row(ui: &mut egui::Ui, label: &str, selected: bool, status: &str) -> egui::Response {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    let fill = if response.hovered() {
        colors.hover
    } else if selected {
        colors.selected
    } else {
        colors.surface
    };
    ui.painter().rect(
        rect,
        7.0,
        fill,
        egui::Stroke::new(1.0_f32, colors.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 18.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        if selected {
            egui_phosphor::regular::RADIO_BUTTON
        } else {
            egui_phosphor::regular::CIRCLE
        },
        egui::FontId::new(16.0, egui::FontFamily::Name("phosphor".into())),
        if selected {
            colors.accent
        } else {
            colors.subtle
        },
    );
    // Reserve status space only when there is room; long names are clipped to the row.
    let show_status = selected && rect.width() >= 390.0;
    let label_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 36.0, rect.top()),
        egui::pos2(
            rect.right() - if show_status { 150.0 } else { 12.0 },
            rect.bottom(),
        ),
    );
    ui.painter().with_clip_rect(label_rect).text(
        egui::pos2(label_rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        colors.text,
    );
    if show_status {
        ui.painter().text(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            status,
            egui::FontId::proportional(11.0),
            colors.muted,
        );
    }
    response
}

fn show_view(ui: &mut egui::Ui, settings: &mut Settings, action: &mut Option<PreferencesAction>) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    ui.add_space(8.0);
    section_title(ui, "Sidebar");
    if ui
        .checkbox(&mut settings.compact_sidebar, "Compact sidebar")
        .changed()
    {
        *action = Some(PreferencesAction::SaveSettings);
    }
    ui.label(
        egui::RichText::new(
            "Show only icons in a narrow sidebar. Hover over an icon to see its name.",
        )
        .size(12.0)
        .color(colors.muted),
    );
}

fn show_connections(
    ui: &mut egui::Ui,
    preferences: &mut PreferencesState,
    settings: &mut Settings,
    _error: &mut Option<String>,
    action: &mut Option<PreferencesAction>,
) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    ui.set_min_width(420.0);
    ui.add_space(8.0);
    #[cfg(target_os = "linux")]
    {
        section_title(ui, "Samba / SMB share");
        ui.label(
            "Connect directly to a network share. Connection details are stored in the local SQLite database for future access.",
        );
        ui.add_space(10.0);
        ui.label("Share folder URL");
        connection_text_edit(
            ui,
            &mut preferences.smb_url_draft,
            "smb://server/Music/Albums",
            false,
        );
        ui.add_space(8.0);
        ui.columns(2, |columns| {
            columns[0].vertical(|ui| {
                ui.label("Username");
                connection_text_edit(ui, &mut preferences.smb_username, "Optional", false);
            });
            columns[1].vertical(|ui| {
                ui.label("Workgroup");
                connection_text_edit(ui, &mut preferences.smb_workgroup, "Optional", false);
            });
        });
        ui.add_space(8.0);
        ui.label("Password");
        connection_text_edit(ui, &mut preferences.smb_password, "Optional", true);
        ui.add_space(8.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 38.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                if left_action_button(ui, "Connect and scan").clicked() {
                    *action = Some(PreferencesAction::ConnectSmbShare);
                }
            },
        );
        ui.add_space(28.0);
    }
    section_title(ui, "Discord Rich Presence");
    ui.label("Show the song currently playing in your Discord profile.");
    ui.add_space(10.0);
    ui.label("Discord Application ID");
    connection_text_edit(
        ui,
        &mut preferences.discord_application_id_draft,
        "Enter your Discord application ID",
        false,
    );
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new("Create an application in the Discord Developer Portal and copy its Application ID here.")
            .size(11.0)
            .color(colors.muted),
    );
    ui.add_space(12.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 38.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            if left_action_button(ui, "Save").clicked() {
                settings.discord_application_id =
                    match preferences.discord_application_id_draft.trim() {
                        "" => None,
                        id => Some(id.to_owned()),
                    };
                *action = Some(PreferencesAction::SaveSettings);
            }
        },
    );
}

fn left_action_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    let font = egui::FontId::proportional(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), colors.text);
    let size = egui::vec2(galley.size().x + 24.0, 34.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        colors.selected
    } else if response.hovered() {
        colors.hover
    } else {
        colors.surface
    };
    ui.painter().rect(
        rect,
        7.0,
        fill,
        egui::Stroke::new(1.0_f32, colors.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        colors.text,
    );
    response
}

fn connection_text_edit(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    password: bool,
) -> egui::Response {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    ui.scope(|ui| {
        let input_fill = colors.input;
        let input_border = colors.border;
        let visuals = &mut ui.visuals_mut().widgets;
        visuals.inactive.bg_fill = input_fill;
        visuals.inactive.bg_stroke = egui::Stroke::new(1.0_f32, input_border);
        visuals.hovered.bg_fill = colors.surface;
        visuals.hovered.bg_stroke = egui::Stroke::new(1.0_f32, colors.border);
        visuals.active.bg_fill = input_fill;
        visuals.active.bg_stroke = egui::Stroke::new(1.0_f32, colors.accent);
        let mut edit = egui::TextEdit::singleline(value)
            .hint_text(egui::RichText::new(hint).color(colors.muted))
            .text_color(colors.text)
            .background_color(input_fill)
            .margin(egui::vec2(8.0, 6.0))
            .desired_width(ui.available_width());
        if password {
            edit = edit.password(true);
        }
        ui.add(edit)
    })
    .inner
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
