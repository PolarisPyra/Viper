use crate::app::MusicApp;
use crate::storage::settings::AlbumSort;
use crate::views::icons::{self, Icon};
use eframe::egui;

pub fn show(ctx: &egui::Context, app: &mut MusicApp) {
    egui::TopBottomPanel::top("album-sort-toolbar")
        .exact_height(48.0)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(16, 18, 23))
                .inner_margin(egui::Margin {
                    left: 20,
                    right: 20,
                    top: 4,
                    bottom: 12,
                }),
        )
        .show(ctx, |ui| {
            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let active_sort = app.settings.album_sort;
                    let mut selected_sort = None;
                    let sort_rect = ui
                        .scope(|ui| {
                            let widgets = &mut ui.visuals_mut().widgets;
                            widgets.inactive.bg_fill = egui::Color32::from_rgb(27, 31, 41);
                            widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(27, 31, 41);
                            let border =
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 48, 61));
                            widgets.inactive.bg_stroke = border;
                            widgets.hovered.bg_fill = egui::Color32::from_rgb(27, 31, 41);
                            widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(27, 31, 41);
                            widgets.hovered.bg_stroke = border;
                            widgets.active.bg_fill = egui::Color32::from_rgb(27, 31, 41);
                            widgets.active.weak_bg_fill = egui::Color32::from_rgb(27, 31, 41);
                            widgets.active.bg_stroke = border;
                            widgets.open.bg_fill = egui::Color32::from_rgb(27, 31, 41);
                            widgets.open.weak_bg_fill = egui::Color32::from_rgb(27, 31, 41);
                            widgets.open.bg_stroke = border;
                            ui.spacing_mut().button_padding = egui::vec2(10.0, 5.0);
                            egui::ComboBox::from_id_salt("album-sort")
                                .selected_text(
                                    egui::RichText::new(active_sort.label())
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(221, 225, 236)),
                                )
                                .width(140.0)
                                .show_ui(ui, |ui| {
                                    for sort in AlbumSort::ALL {
                                        if ui
                                            .selectable_label(active_sort == sort, sort.label())
                                            .clicked()
                                        {
                                            selected_sort = Some(sort);
                                            ui.close_menu();
                                        }
                                    }
                                })
                                .response
                                .rect
                        })
                        .inner;
                    if let Some(sort) = selected_sort {
                        app.album_sort_changed(sort);
                    }
                    let direction = app.settings.sort_ascending;
                    let (allocated_rect, _) =
                        ui.allocate_exact_size(egui::vec2(34.0, 32.0), egui::Sense::hover());
                    let rect = egui::Rect::from_center_size(
                        egui::pos2(allocated_rect.center().x, sort_rect.center().y),
                        allocated_rect.size(),
                    );
                    let response = ui.interact(
                        rect,
                        ui.id().with("album-sort-direction"),
                        egui::Sense::click(),
                    );
                    if response.hovered() {
                        ui.painter()
                            .rect_filled(rect, 6.0, egui::Color32::from_rgb(35, 39, 50));
                    }
                    icons::draw(
                        ui.painter(),
                        rect.shrink(7.0),
                        if direction {
                            Icon::SortAscending
                        } else {
                            Icon::SortDescending
                        },
                        if response.hovered() {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_gray(190)
                        },
                    );
                    if response
                        .on_hover_text(if direction { "Ascending" } else { "Descending" })
                        .clicked()
                    {
                        app.toggle_sort_direction();
                    }
                },
            );
        });

    let dark = egui::Color32::from_rgb(16, 18, 23);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(dark).inner_margin(0))
        .show(ctx, |ui| show_grid(ctx, ui, app));
}

fn show_grid(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut MusicApp) {
    ui.spacing_mut().scroll.bar_outer_margin = 10.0;
    ui.spacing_mut().scroll.dormant_background_opacity = 0.0;
    ui.spacing_mut().scroll.active_background_opacity = 0.0;
    ui.spacing_mut().scroll.interact_background_opacity = 0.0;
    let panel_rect = ui.max_rect();
    let scrollbar_rect = panel_rect.shrink2(egui::vec2(0.0, 14.0));
    let filtered = app.filtered_album_indices();
    egui::ScrollArea::vertical()
        .id_salt("album-grid")
        .auto_shrink([false, false])
        .drag_to_scroll(false)
        .scroll_bar_rect(scrollbar_rect)
        .show_viewport(ui, |ui, viewport| {
            let available_width = ui.available_width();
            let (columns, card_width, art_size, gap, grid_left) = grid_metrics(available_width);
            if filtered.is_empty() {
                if app.scanning && app.library.albums.is_empty() && app.search.trim().is_empty() {
                    show_skeleton_albums(ctx, ui, available_width);
                } else {
                    ui.add_space(70.0);
                    let message = if app.library.albums.is_empty() {
                        "Your albums will appear here"
                    } else if app.scanning && app.search.trim().is_empty() {
                        "Scanning your music folder…"
                    } else {
                        "No albums match your search"
                    };
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(message)
                                .size(18.0)
                                .color(egui::Color32::from_gray(120)),
                        );
                    });
                }
                return;
            }

            const TOP_PADDING: f32 = 16.0;
            const ROW_GAP: f32 = 10.0;
            let row_height = art_size + 81.0;
            let row_step = row_height + ROW_GAP;
            let row_count = filtered.len().div_ceil(columns);
            let content_top = ui.max_rect().top();
            let content_left = ui.max_rect().left();
            let content_width = ui.available_width();
            ui.set_height(TOP_PADDING + row_count as f32 * row_step);
            let first_row = (((viewport.min.y - TOP_PADDING).max(0.0) / row_step).floor() as usize)
                .min(row_count);
            let last_row = (((viewport.max.y - TOP_PADDING).max(0.0) / row_step).ceil() as usize
                + 1)
            .min(row_count);

            for row_index in first_row..last_row {
                let y = content_top + TOP_PADDING + row_index as f32 * row_step;
                let row_rect = egui::Rect::from_min_size(
                    egui::pos2(content_left, y),
                    egui::vec2(content_width, row_height),
                );
                let mut row_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(row_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Min)),
                );
                row_ui.add_space(grid_left);
                row_ui.spacing_mut().item_spacing.x = gap;
                let start = row_index * columns;
                let end = (start + columns).min(filtered.len());
                for &album_index in &filtered[start..end] {
                    if app.library.albums.get(album_index).is_none() {
                        continue;
                    }
                    let texture = app.album_texture(ctx, album_index);
                    let album = &app.library.albums[album_index];
                    let track_count = album.tracks.len();
                    let selected = app.selected_album == Some(album_index);
                    let response = row_ui.allocate_ui_with_layout(
                        egui::vec2(card_width, row_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::Frame::new()
                                .fill(if selected {
                                    egui::Color32::from_rgb(38, 39, 51)
                                } else {
                                    egui::Color32::TRANSPARENT
                                })
                                .inner_margin(egui::Margin::same(6))
                                .corner_radius(3)
                                .show(ui, |ui| {
                                    let art_rect =
                                        ui.allocate_space(egui::vec2(art_size, art_size)).1;
                                    if ui.is_rect_visible(art_rect) {
                                        if let Some(texture) = &texture {
                                            ui.painter().image(
                                                texture.id(),
                                                art_rect,
                                                egui::Rect::from_min_max(
                                                    egui::pos2(0.0, 0.0),
                                                    egui::pos2(1.0, 1.0),
                                                ),
                                                egui::Color32::WHITE,
                                            );
                                        }
                                    }
                                    ui.add_space(9.0);
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&album.title)
                                                .color(egui::Color32::WHITE)
                                                .size(14.0),
                                        )
                                        .truncate(),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format!(
                                                "{} · {track_count} tracks",
                                                album.artist
                                            ))
                                            .color(egui::Color32::from_gray(145))
                                            .size(12.0),
                                        )
                                        .truncate(),
                                    );
                                });
                        },
                    );
                    let card = response.response.interact(egui::Sense::click());
                    if card.clicked() {
                        let already_selected = app.selected_album == Some(album_index);
                        if !already_selected {
                            app.selected_track = None;
                            app.selected_album = Some(album_index);
                        } else {
                            let album = app.library.albums[album_index].clone();
                            app.playback.play_album(&album, &app.library.tracks);
                        }
                        app.show_album_details = true;
                        ctx.request_repaint();
                    }
                }
            }
        });
}

fn show_skeleton_albums(ctx: &egui::Context, ui: &mut egui::Ui, available_width: f32) {
    let (columns, card_width, art_size, gap, grid_left) = grid_metrics(available_width);
    let card_height = art_size + 81.0;
    let pulse = ((ctx.input(|input| input.time) * 2.2).sin() * 0.5 + 0.5) as f32;
    let cover_color = egui::Color32::from_gray((34.0 + pulse * 12.0) as u8);
    let line_color = egui::Color32::from_gray((43.0 + pulse * 10.0) as u8);

    ui.add_space(16.0);
    for _ in 0..3 {
        ui.horizontal(|ui| {
            ui.add_space(grid_left);
            ui.spacing_mut().item_spacing.x = gap;
            for _ in 0..columns {
                ui.allocate_ui_with_layout(
                    egui::vec2(card_width, card_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::Frame::new()
                            .inner_margin(egui::Margin::same(6))
                            .corner_radius(10.0)
                            .show(ui, |ui| {
                                let cover = ui.allocate_space(egui::vec2(art_size, art_size)).1;
                                ui.painter().rect_filled(cover, 8.0, cover_color);
                                ui.add_space(10.0);
                                let title = ui
                                    .allocate_space(egui::vec2((card_width - 12.0).max(24.0), 12.0))
                                    .1;
                                ui.painter().rect_filled(title, 4.0, line_color);
                                ui.add_space(7.0);
                                let artist = ui.allocate_space(egui::vec2(82.0, 10.0)).1;
                                ui.painter().rect_filled(artist, 4.0, line_color);
                            });
                    },
                );
            }
        });
        ui.add_space(10.0);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}

fn grid_metrics(available_width: f32) -> (usize, f32, f32, f32, f32) {
    const LEFT_EDGE_PADDING: f32 = 12.0;
    const RIGHT_EDGE_PADDING: f32 = 18.0;
    const MIN_GAP: f32 = 12.0;
    const BASE_CARD_WIDTH: f32 = 166.0;
    const MIN_COLUMNS: usize = 3;
    let cards_area = (available_width - LEFT_EDGE_PADDING - RIGHT_EDGE_PADDING).max(0.0);
    let natural_columns = ((cards_area + MIN_GAP) / (BASE_CARD_WIDTH + MIN_GAP)).floor() as usize;
    let columns = natural_columns.max(MIN_COLUMNS);
    let card_width = if natural_columns < MIN_COLUMNS {
        ((cards_area - (columns - 1) as f32 * MIN_GAP) / columns as f32).max(48.0)
    } else {
        BASE_CARD_WIDTH
    };
    let art_size = (card_width - 12.0).max(36.0);
    let gap = if columns > 1 {
        ((cards_area - columns as f32 * card_width) / (columns - 1) as f32).max(MIN_GAP)
    } else {
        MIN_GAP
    };
    (columns, card_width, art_size, gap, LEFT_EDGE_PADDING)
}

pub fn show_details(ctx: &egui::Context, app: &mut MusicApp) {
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
    let album_key = crate::app::album_sort_key(&album.artist, &album.title);
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
                            crate::views::icons::draw(
                                ui.painter(),
                                close_rect.shrink(2.0),
                                crate::views::icons::Icon::Close,
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
                                        crate::views::icons::draw(
                                            ui.painter(),
                                            rect.shrink(2.0),
                                            if value <= rating {
                                                Icon::StarFilled
                                            } else {
                                                Icon::Star
                                            },
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
                                    crate::views::icons::draw(
                                        ui.painter(),
                                        favorite_rect.shrink(2.0),
                                        if is_favorite {
                                            Icon::HeartFilled
                                        } else {
                                            Icon::Heart
                                        },
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
                                    crate::views::icons::draw(
                                        ui.painter(),
                                        number_or_icon,
                                        if playback.is_playing() {
                                            crate::views::icons::Icon::Pause
                                        } else {
                                            crate::views::icons::Icon::Play
                                        },
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
