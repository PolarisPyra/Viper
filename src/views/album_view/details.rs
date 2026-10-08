use crate::app::MusicApp;
use eframe::egui;

pub(super) fn show(ctx: &egui::Context, app: &mut MusicApp) {
    if !app.show_album_details {
        return;
    }
    let Some(album_index) = app.selected_album else {
        return;
    };
    let cover = app.album_texture(ctx, album_index);
    let library = &app.library;
    let playback = &mut app.playback;
    let selected_track = &mut app.selected_track;
    let show_album_details = &mut app.show_album_details;
    let Some(album) = library.albums.get(album_index) else {
        return;
    };
    let album_key = crate::library::sorting::album_sort_key(&album.artist, &album.title);
    const MIN_PANEL_WIDTH: f32 = 260.0;
    const MAX_PANEL_WIDTH: f32 = 520.0;
    let panel_width = app
        .settings
        .right_panel_width
        .clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
    let settings = &mut app.settings;
    let mut settings_changed = false;
    let output = egui::SidePanel::right("album-tracklist-side-panel")
        .resizable(true)
        .width_range(MIN_PANEL_WIDTH..=MAX_PANEL_WIDTH)
        .default_width(panel_width)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(20, 23, 31))
                .inner_margin(0),
        )
        .show(ctx, |ui| {
            let panel_rect = ui.max_rect();
            let panel_height = panel_rect.height();
            let panel_bg = egui::Color32::from_rgb(20, 23, 31);
            ui.painter().rect_filled(panel_rect, 0.0, panel_bg);
            ui.painter().line_segment(
                [panel_rect.left_top(), panel_rect.left_bottom()],
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 49, 61)),
            );

            egui::Frame::new()
                .fill(panel_bg)
                .inner_margin(egui::Margin::symmetric(20, 18))
                .show(ui, |ui| {
                    let content_height = (panel_height - 36.0).max(0.0);
                    let content_width = (panel_rect.width() - 40.0).max(0.0);
                    ui.set_width(content_width);
                    ui.set_min_height(content_height);
                    ui.set_max_height(content_height);

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("ALBUM TRACKS")
                                    .size(10.0)
                                    .strong()
                                    .color(egui::Color32::from_gray(142)),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(format!("{} songs", album.tracks.len()))
                                    .size(12.0)
                                    .color(egui::Color32::from_gray(175)),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (close_rect, close_response) = ui
                                .allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
                            paint_album_panel_action_hover(
                                ui,
                                close_rect,
                                close_response.hovered(),
                            );
                            ui.painter().text(
                                close_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                egui_phosphor::regular::X,
                                egui::FontId::new(20.0, egui::FontFamily::Name("phosphor".into())),
                                if close_response.hovered() {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::from_gray(180)
                                },
                            );
                            if close_response.on_hover_text("Close album tracks").clicked() {
                                *show_album_details = false;
                            }
                        });
                    });

                    ui.add_space(15.0);
                    ui.horizontal(|ui| {
                        if let Some(texture) = &cover {
                            let art_rect = ui.allocate_space(egui::vec2(80.0, 80.0)).1;
                            ui.painter().image(
                                texture.id(),
                                art_rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                            ui.add_space(12.0);
                        }
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.set_width(ui.available_width());
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&album.title)
                                        .size(18.0)
                                        .color(egui::Color32::WHITE),
                                )
                                .truncate(),
                            );
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&album.artist)
                                        .size(13.0)
                                        .color(egui::Color32::from_gray(155)),
                                )
                                .truncate(),
                            );
                            ui.add_space(3.0);

                            let rating = settings
                                .album_ratings
                                .get(&album_key)
                                .copied()
                                .unwrap_or_default();
                            let is_favorite = settings.favorite_albums.contains(&album_key);
                            let rating_row_width = ui.available_width();
                            ui.allocate_ui_with_layout(
                                egui::vec2(rating_row_width, 24.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    let rating_row_rect = ui.max_rect();
                                    ui.spacing_mut().item_spacing.x = 1.0;
                                    for value in 1..=5 {
                                        let (rect, response) = ui.allocate_exact_size(
                                            egui::vec2(22.0, 24.0),
                                            egui::Sense::click(),
                                        );
                                        let (glyph, font_family) = if value <= rating {
                                            (
                                                egui_phosphor::fill::STAR,
                                                egui::FontFamily::Name("phosphor-fill".into()),
                                            )
                                        } else {
                                            (
                                                egui_phosphor::regular::STAR,
                                                egui::FontFamily::Proportional,
                                            )
                                        };
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            glyph,
                                            egui::FontId::new(18.0, font_family),
                                            if value <= rating {
                                                egui::Color32::from_rgb(255, 196, 74)
                                            } else {
                                                egui::Color32::from_rgb(116, 110, 94)
                                            },
                                        );
                                        let clear_rating = value == rating;
                                        let tooltip = if clear_rating {
                                            "Clear rating".to_owned()
                                        } else {
                                            format!("Rate {value} out of 5")
                                        };
                                        if response.on_hover_text(tooltip).clicked() {
                                            if clear_rating {
                                                settings.album_ratings.remove(&album_key);
                                            } else {
                                                settings
                                                    .album_ratings
                                                    .insert(album_key.clone(), value);
                                            }
                                            settings_changed = true;
                                        }
                                    }
                                    let favorite_rect = egui::Rect::from_center_size(
                                        egui::pos2(
                                            rating_row_rect.right() - 15.0,
                                            rating_row_rect.center().y,
                                        ),
                                        egui::vec2(30.0, 30.0),
                                    );
                                    let response = ui.interact(
                                        favorite_rect,
                                        ui.id().with("album-favorite-heart"),
                                        egui::Sense::click(),
                                    );
                                    paint_album_panel_action_hover(
                                        ui,
                                        favorite_rect,
                                        response.hovered(),
                                    );
                                    let (favorite_glyph, favorite_font) = if is_favorite {
                                        (
                                            egui_phosphor::fill::HEART,
                                            egui::FontFamily::Name("phosphor-fill".into()),
                                        )
                                    } else {
                                        (
                                            egui_phosphor::regular::HEART,
                                            egui::FontFamily::Proportional,
                                        )
                                    };
                                    ui.painter().text(
                                        favorite_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        favorite_glyph,
                                        egui::FontId::new(20.0, favorite_font),
                                        if is_favorite {
                                            egui::Color32::from_rgb(242, 83, 103)
                                        } else if response.hovered() {
                                            egui::Color32::WHITE
                                        } else {
                                            egui::Color32::from_gray(180)
                                        },
                                    );
                                    if response
                                        .on_hover_text(if is_favorite {
                                            "Remove from favorites"
                                        } else {
                                            "Add to favorites"
                                        })
                                        .clicked()
                                    {
                                        if is_favorite {
                                            settings.favorite_albums.remove(&album_key);
                                        } else {
                                            settings.favorite_albums.insert(album_key.clone());
                                        }
                                        settings_changed = true;
                                    }
                                },
                            );
                        });
                    });

                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(7.0);
                    let tracks_viewport = ui.available_rect_before_wrap();
                    let pointer_inside_tracks = ctx.input(|input| {
                        input
                            .pointer
                            .hover_pos()
                            .is_some_and(|position| tracks_viewport.contains(position))
                    });
                    egui::ScrollArea::vertical()
                        .id_salt("album-popout-tracks")
                        .drag_to_scroll(false)
                        .enable_scrolling(pointer_inside_tracks)
                        .max_height((content_height - 190.0).max(100.0))
                        .show(ui, |ui| {
                            ui.add_space(12.0);
                            for (position, track_index) in album.tracks.iter().copied().enumerate()
                            {
                                let Some(track) = library.tracks.get(track_index) else {
                                    continue;
                                };
                                let (row_rect, response) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 50.0),
                                    egui::Sense::click(),
                                );
                                let highlight_rect = egui::Rect::from_min_max(
                                    row_rect.min,
                                    row_rect.max - egui::vec2(12.0, 0.0),
                                );
                                let playing = playback.current == Some(track_index);
                                let selected = *selected_track == Some(track_index);
                                if selected {
                                    ui.painter().rect_filled(
                                        highlight_rect,
                                        3.0,
                                        egui::Color32::from_rgb(42, 39, 59),
                                    );
                                }

                                let number_or_icon = egui::Rect::from_min_size(
                                    row_rect.min + egui::vec2(18.0, 14.0),
                                    egui::vec2(20.0, 20.0),
                                );
                                if playing {
                                    ui.painter().text(
                                        number_or_icon.center(),
                                        egui::Align2::CENTER_CENTER,
                                        if playback.is_playing() {
                                            egui_phosphor::regular::PAUSE
                                        } else {
                                            egui_phosphor::regular::PLAY
                                        },
                                        egui::FontId::new(
                                            18.0,
                                            egui::FontFamily::Name("phosphor".into()),
                                        ),
                                        egui::Color32::from_rgb(174, 149, 255),
                                    );
                                } else {
                                    ui.painter().text(
                                        number_or_icon.center(),
                                        egui::Align2::CENTER_CENTER,
                                        format!("{:02}", position + 1),
                                        egui::FontId::proportional(11.0),
                                        egui::Color32::from_gray(130),
                                    );
                                }

                                let text_rect = egui::Rect::from_min_max(
                                    row_rect.min + egui::vec2(50.0, 7.0),
                                    row_rect.max - egui::vec2(58.0, 5.0),
                                );
                                let mut text_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(text_rect)
                                        .layout(egui::Layout::top_down(egui::Align::Min)),
                                );
                                text_ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&track.title).size(12.0).color(
                                            if selected {
                                                egui::Color32::from_rgb(194, 176, 255)
                                            } else {
                                                egui::Color32::from_rgb(230, 232, 239)
                                            },
                                        ),
                                    )
                                    .truncate(),
                                );
                                text_ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&track.artist)
                                            .size(10.0)
                                            .color(egui::Color32::from_gray(145)),
                                    )
                                    .truncate(),
                                );
                                if let Some(duration) = track.duration_ms {
                                    ui.painter().text(
                                        highlight_rect.right_center() - egui::vec2(10.0, 0.0),
                                        egui::Align2::RIGHT_CENTER,
                                        format_duration(duration),
                                        egui::FontId::proportional(10.0),
                                        egui::Color32::from_gray(135),
                                    );
                                }
                                if response.clicked() {
                                    if *selected_track == Some(track_index) {
                                        playback.play_track_in_album(
                                            album,
                                            &library.tracks,
                                            track_index,
                                        );
                                    } else {
                                        *selected_track = Some(track_index);
                                    }
                                }
                            }
                            ui.add_space(18.0);
                        });
                });
        });
    if settings_changed {
        app.album_sort_dirty = true;
        if let Err(error) = app.settings.save() {
            app.error = Some(format!("Could not save settings: {error}"));
        }
    }
    let resize_id = egui::Id::new("album-tracklist-side-panel").with("__resize");
    let resize_finished = ctx
        .read_response(resize_id)
        .is_some_and(|response| response.drag_stopped_by(egui::PointerButton::Primary));
    if resize_finished {
        let width = output
            .response
            .rect
            .width()
            .clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
        app.settings.right_panel_width = width;
        if let Err(error) = app.settings.save() {
            app.error = Some(format!("Could not save settings: {error}"));
        }
    }
}

fn paint_album_panel_action_hover(ui: &egui::Ui, rect: egui::Rect, hovered: bool) {
    if hovered {
        ui.painter()
            .rect_filled(rect, 7.0, egui::Color32::from_rgb(48, 55, 70));
    }
}

fn format_duration(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
