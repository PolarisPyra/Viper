use crate::app::MusicApp;
use crate::components::icons::{self, Icon};
use crate::storage::settings::AlbumSort;
use eframe::egui;

pub(super) fn show(ctx: &egui::Context, app: &mut MusicApp) {
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
