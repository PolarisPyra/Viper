use crate::app::{Page, ViperApp};
use eframe::egui;

pub fn show(ctx: &egui::Context, app: &mut ViperApp) {
    let original_style = ctx.style().as_ref().clone();
    let mut menu_style = original_style.clone();
    menu_style.spacing.menu_spacing = 6.0;
    style_menu_visuals(&mut menu_style.visuals);
    ctx.set_style(menu_style);

    egui::TopBottomPanel::top("app-menu-bar")
        .exact_height(34.0)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(16, 18, 23))
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
                    ui.scope(|ui| {
                        // Keep the menu triggers visually quiet; popup styling is
                        // applied inside each menu closure to match album sort.
                        crate::components::view_toolbar::style_sort_menu(ui);

                        let visuals = &mut ui.visuals_mut().widgets;
                        visuals.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        visuals.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                        visuals.inactive.bg_stroke = egui::Stroke::NONE;
                        visuals.hovered.bg_fill = egui::Color32::from_rgb(35, 39, 50);
                        visuals.hovered.bg_stroke = egui::Stroke::NONE;
                        visuals.active.bg_fill = egui::Color32::from_rgb(42, 46, 59);
                        visuals.active.bg_stroke = egui::Stroke::NONE;
                        ui.spacing_mut().button_padding = egui::vec2(8.0, 2.0);
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.menu_button(
                            egui::RichText::new("File")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(190, 196, 211)),
                            |ui| {
                                style_menu_popup(ui);
                                if menu_item(ui, "Preferences").clicked() {
                                    app.show_preferences = true;
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
                                .color(egui::Color32::from_rgb(190, 196, 211)),
                            |ui| {
                                style_menu_popup(ui);
                                ui.menu_button("Add Library", |ui| {
                                    style_menu_popup(ui);
                                    if menu_item(ui, "Choose Music Folder").clicked() {
                                        ui.close_menu();
                                        app.choose_folder();
                                    }
                                });
                            },
                        );
                        ui.menu_button(
                            egui::RichText::new("Playback")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(190, 196, 211)),
                            |ui| {
                                style_menu_popup(ui);
                                let has_current_track = app.playback.current.is_some();
                                let can_play = has_current_track || app.selected_album.is_some();
                                let play_label = if has_current_track && app.playback.is_paused() {
                                    "Resume"
                                } else if has_current_track {
                                    "Pause"
                                } else {
                                    "Play Selected Album"
                                };
                                if ui
                                    .add_enabled(can_play, egui::SelectableLabel::new(false, egui::RichText::new(play_label).color(egui::Color32::WHITE)))
                                    .clicked()
                                {
                                    if has_current_track {
                                        app.playback.toggle_pause();
                                    } else if let Some(album_index) = app.selected_album {
                                        if let Some(album) = app.library.albums.get(album_index) {
                                            app.playback.play_album(album, &app.library.tracks);
                                        }
                                    }
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .add_enabled(
                                        has_current_track,
                                        egui::SelectableLabel::new(false, egui::RichText::new("Previous Track").color(egui::Color32::WHITE)),
                                    )
                                    .clicked()
                                {
                                    app.playback.previous(&app.library.tracks);
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(has_current_track, egui::SelectableLabel::new(false, egui::RichText::new("Next Track").color(egui::Color32::WHITE)))
                                    .clicked()
                                {
                                    app.playback.skip_next(&app.library.tracks);
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(has_current_track, egui::SelectableLabel::new(false, egui::RichText::new("Stop").color(egui::Color32::WHITE)))
                                    .clicked()
                                {
                                    app.playback.stop();
                                    ui.close_menu();
                                }
                            },
                        );
                        ui.menu_button(
                            egui::RichText::new("View")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(190, 196, 211)),
                            |ui| {
                                style_menu_popup(ui);
                                if ui
                                    .selectable_label(app.page == Page::Home, "Home")
                                    .clicked()
                                {
                                    app.page = Page::Home;
                                    ui.close_menu();
                                }
                                if ui
                                    .selectable_label(app.page == Page::Albums, "Albums")
                                    .clicked()
                                {
                                    app.page = Page::Albums;
                                    ui.close_menu();
                                }
                            },
                        );
                    });
                },
            );
        });

    ctx.set_style(original_style);
}

fn style_menu_popup(ui: &mut egui::Ui) {
    crate::components::view_toolbar::style_sort_menu(ui);
    ui.set_min_width(220.0);
    ui.spacing_mut().menu_margin = egui::Margin::same(8);
    ui.spacing_mut().button_padding = egui::vec2(14.0, 5.0);
    ui.spacing_mut().item_spacing.y = 4.0;
    let widgets = &mut ui.visuals_mut().widgets;
    widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    widgets.inactive.bg_stroke = egui::Stroke::NONE;
    // Match the album sort popup's native hovered row visuals.
    widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 39, 50);
    widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(35, 39, 50);
    widgets.active.bg_fill = egui::Color32::from_rgb(42, 46, 59);
    widgets.active.weak_bg_fill = egui::Color32::from_rgb(42, 46, 59);
    widgets.active.bg_stroke = egui::Stroke::NONE;
    widgets.open.bg_fill = egui::Color32::from_rgb(35, 39, 50);
    widgets.open.weak_bg_fill = egui::Color32::from_rgb(35, 39, 50);
    widgets.open.bg_stroke = egui::Stroke::NONE;
}

fn style_menu_visuals(visuals: &mut egui::Visuals) {
    let fill = egui::Color32::from_rgb(27, 31, 41);
    let selected = egui::Color32::from_rgb(48, 52, 68);
    let border = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 48, 61));

    visuals.window_fill = fill;
    visuals.window_stroke = border;
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    visuals.selection.bg_fill = selected;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
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
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 39, 50);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(35, 39, 50);
    visuals.widgets.open.bg_fill = egui::Color32::from_rgb(35, 39, 50);
    visuals.widgets.open.weak_bg_fill = egui::Color32::from_rgb(35, 39, 50);
    visuals.widgets.open.bg_stroke = egui::Stroke::NONE;
}


fn menu_item(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.selectable_label(
        false,
        egui::RichText::new(label).color(egui::Color32::WHITE),
    )
}
