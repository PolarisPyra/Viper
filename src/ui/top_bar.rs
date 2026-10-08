use crate::app::{MusicApp, Page};
use crate::ui::icons::{self, Icon};
use eframe::egui;

pub fn show(ctx: &egui::Context, app: &mut MusicApp) {
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
                                ui.spacing_mut().button_padding = egui::vec2(12.0, 3.0);
                                if ui.button("Preferences").clicked() {
                                    app.show_preferences = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Quit").clicked() {
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
                                ui.spacing_mut().button_padding = egui::vec2(12.0, 3.0);
                                ui.menu_button("Add Library", |ui| {
                                    if ui.button("Choose Music Folder").clicked() {
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
                                ui.spacing_mut().button_padding = egui::vec2(12.0, 3.0);
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
                                    .add_enabled(can_play, egui::Button::new(play_label))
                                    .clicked()
                                {
                                    if has_current_track {
                                        app.playback.toggle_pause(&app.library.tracks);
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
                                        egui::Button::new("Previous Track"),
                                    )
                                    .clicked()
                                {
                                    app.playback.previous(&app.library.tracks);
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(has_current_track, egui::Button::new("Next Track"))
                                    .clicked()
                                {
                                    app.playback.skip_next(&app.library.tracks);
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(has_current_track, egui::Button::new("Stop"))
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
                                ui.spacing_mut().button_padding = egui::vec2(12.0, 3.0);
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

    egui::TopBottomPanel::top("library-topbar")
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(16, 18, 23))
                .inner_margin(egui::Margin::symmetric(20, 12)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(match app.page {
                        Page::Home => "Home",
                        Page::Albums => "Albums",
                    })
                    .size(28.0)
                    .strong()
                    .color(egui::Color32::WHITE),
                );
                if app.scanning {
                    let completed = app
                        .scan_progress
                        .completed
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let total = app
                        .scan_progress
                        .total
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let phase = app
                        .scan_progress
                        .phase
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let label = match phase {
                        0 => "Finding audio files…".to_owned(),
                        1 => format!("Reading track tags · {completed}/{total}"),
                        _ => format!("Loading album artwork · {completed}/{total}"),
                    };
                    let fraction = if total == 0 {
                        0.0
                    } else {
                        completed as f32 / total as f32
                    };
                    let progress_width = (ui.available_width() - 288.0).clamp(120.0, 270.0);
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(progress_width)
                            .text(label)
                            .animate(true),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(27, 31, 41))
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgb(43, 48, 61),
                        ))
                        .corner_radius(3.0)
                        .inner_margin(egui::Margin::symmetric(10, 5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(18.0, 20.0),
                                    egui::Sense::hover(),
                                );
                                icons::draw(
                                    ui.painter(),
                                    rect,
                                    Icon::Search,
                                    egui::Color32::from_gray(140),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.search)
                                        .hint_text("Search albums")
                                        .desired_width(230.0)
                                        .frame(false),
                                );
                            });
                        });
                });
            });
            if let Some(error) = &app.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        });
}
