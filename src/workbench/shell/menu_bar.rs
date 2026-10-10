use crate::{
    features::{library::state::LibraryFeature, playback::Playback},
    workbench::{Page, WorkbenchState},
};
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuBarAction {
    ChooseMusicFolder,
}

pub fn show(
    ctx: &egui::Context,
    workbench: &mut WorkbenchState,
    library: &LibraryFeature,
    playback: &mut Playback,
) -> Option<MenuBarAction> {
    let colors = crate::shared::ui::theme::colors(ctx);
    let mut action = None;
    let original_style = ctx.style().as_ref().clone();
    let mut menu_style = original_style.clone();
    menu_style.spacing.menu_spacing = 6.0;
    style_menu_visuals(&mut menu_style.visuals, colors);
    ctx.set_style(menu_style);

    egui::TopBottomPanel::top("app-menu-bar")
        .exact_height(34.0)
        .frame(
            egui::Frame::new()
                .fill(colors.canvas)
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 18,
                    top: 2,
                    bottom: 2,
                }),
        )
        .show(ctx, |ui| {
            let menu_size = ui.available_size();
            ui.allocate_ui_with_layout(
                menu_size,
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new(egui_phosphor::regular::DISC)
                            .font(egui::FontId::new(
                                18.0,
                                egui::FontFamily::Name("phosphor".into()),
                            ))
                            .color(colors.muted),
                    );
                    ui.add_space(4.0);
                    ui.scope(|ui| {
                        // Keep the menu triggers visually quiet; popup styling is
                        // applied inside each menu closure to match album sort.
                        crate::shared::ui::theme::style_dropdown(ui);

                        let visuals = &mut ui.visuals_mut().widgets;
                        visuals.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        visuals.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                        visuals.inactive.bg_stroke = egui::Stroke::NONE;
                        visuals.hovered.bg_fill = colors.hover;
                        visuals.hovered.weak_bg_fill = colors.hover;
                        visuals.hovered.bg_stroke = egui::Stroke::NONE;
                        visuals.hovered.corner_radius = egui::CornerRadius::same(5);
                        visuals.active.bg_fill = colors.hover;
                        visuals.active.weak_bg_fill = colors.hover;
                        visuals.active.bg_stroke = egui::Stroke::NONE;
                        visuals.open.bg_fill = colors.hover;
                        visuals.open.weak_bg_fill = colors.hover;
                        visuals.open.bg_stroke = egui::Stroke::NONE;
                        visuals.open.corner_radius = egui::CornerRadius::same(5);
                        visuals.inactive.corner_radius = egui::CornerRadius::same(5);
                        visuals.active.corner_radius = egui::CornerRadius::same(5);
                        ui.spacing_mut().button_padding = egui::vec2(8.0, 2.0);
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.menu_button(
                            egui::RichText::new("File").size(12.0).color(colors.muted),
                            |ui| {
                                style_menu_popup(ui);
                                if menu_item(ui, "Preferences").clicked() {
                                    workbench.show_preferences = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if menu_item(ui, "Quit").clicked() {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                    ui.close_menu();
                                }
                            },
                        );
                        ui.menu_button(
                            egui::RichText::new("Library")
                                .size(12.0)
                                .color(colors.muted),
                            |ui| {
                                style_menu_popup(ui);
                                ui.menu_button("Add Library", |ui| {
                                    style_menu_popup(ui);
                                    if menu_item(ui, "Choose Music Folder").clicked() {
                                        ui.close_menu();
                                        action = Some(MenuBarAction::ChooseMusicFolder);
                                    }
                                });
                            },
                        );
                        ui.menu_button(
                            egui::RichText::new("Playback")
                                .size(12.0)
                                .color(colors.muted),
                            |ui| {
                                style_menu_popup(ui);
                                let has_current_track = playback.current.is_some();
                                let can_play =
                                    has_current_track || library.view.selected_album.is_some();
                                let play_label = if has_current_track && playback.is_paused() {
                                    "Resume"
                                } else if has_current_track {
                                    "Pause"
                                } else {
                                    "Play Selected Album"
                                };
                                if ui
                                    .add_enabled(
                                        can_play,
                                        egui::SelectableLabel::new(
                                            false,
                                            egui::RichText::new(play_label).color(colors.text),
                                        ),
                                    )
                                    .clicked()
                                {
                                    if has_current_track {
                                        playback.toggle_pause();
                                    } else if let Some(album_index) = library.view.selected_album {
                                        if let Some(album) = library.model.albums.get(album_index) {
                                            playback.play_album(album, &library.model.tracks);
                                        }
                                    }
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .add_enabled(
                                        has_current_track,
                                        egui::SelectableLabel::new(
                                            false,
                                            egui::RichText::new("Previous Track")
                                                .color(colors.text),
                                        ),
                                    )
                                    .clicked()
                                {
                                    playback.previous(&library.model.tracks);
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(
                                        has_current_track,
                                        egui::SelectableLabel::new(
                                            false,
                                            egui::RichText::new("Next Track").color(colors.text),
                                        ),
                                    )
                                    .clicked()
                                {
                                    playback.skip_next(&library.model.tracks);
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(
                                        has_current_track,
                                        egui::SelectableLabel::new(
                                            false,
                                            egui::RichText::new("Stop").color(colors.text),
                                        ),
                                    )
                                    .clicked()
                                {
                                    playback.stop();
                                    ui.close_menu();
                                }
                            },
                        );
                        ui.menu_button(
                            egui::RichText::new("View").size(12.0).color(colors.muted),
                            |ui| {
                                style_menu_popup(ui);
                                if ui
                                    .selectable_label(workbench.page == Page::Home, "Home")
                                    .clicked()
                                {
                                    workbench.page = Page::Home;
                                    ui.close_menu();
                                }
                                if ui
                                    .selectable_label(workbench.page == Page::Albums, "Albums")
                                    .clicked()
                                {
                                    workbench.page = Page::Albums;
                                    ui.close_menu();
                                }
                            },
                        );
                    });
                },
            );
        });

    ctx.set_style(original_style);
    action
}

fn style_menu_popup(ui: &mut egui::Ui) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    crate::shared::ui::theme::style_dropdown(ui);
    ui.set_min_width(220.0);
    ui.spacing_mut().menu_margin = egui::Margin::same(8);
    ui.spacing_mut().button_padding = egui::vec2(14.0, 5.0);
    ui.spacing_mut().item_spacing.y = 4.0;
    let widgets = &mut ui.visuals_mut().widgets;
    widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    widgets.inactive.bg_stroke = egui::Stroke::NONE;
    // Match the album sort popup's native hovered row visuals.
    widgets.hovered.bg_fill = colors.hover;
    widgets.hovered.weak_bg_fill = colors.hover;
    widgets.active.bg_fill = colors.hover;
    widgets.active.weak_bg_fill = colors.hover;
    widgets.active.bg_stroke = egui::Stroke::NONE;
    widgets.open.bg_fill = colors.hover;
    widgets.open.weak_bg_fill = colors.hover;
    widgets.open.bg_stroke = egui::Stroke::NONE;
}

fn style_menu_visuals(visuals: &mut egui::Visuals, colors: crate::shared::ui::theme::Palette) {
    let fill = colors.panel;
    let selected = colors.selected;
    let border = egui::Stroke::new(1.0_f32, colors.border);

    visuals.window_fill = fill;
    visuals.window_stroke = border;
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    visuals.selection.bg_fill = selected;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, colors.text);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = border;
    }
    visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.hovered.bg_fill = colors.hover;
    visuals.widgets.hovered.weak_bg_fill = colors.hover;
    visuals.widgets.open.bg_fill = colors.hover;
    visuals.widgets.open.weak_bg_fill = colors.hover;
    visuals.widgets.open.bg_stroke = egui::Stroke::NONE;
}

fn menu_item(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    ui.selectable_label(false, egui::RichText::new(label).color(colors.text))
}
