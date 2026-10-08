use crate::{app::ViperApp, storage::settings::AlbumSort};
use eframe::egui;

use crate::components::album_card;

const CANVAS: egui::Color32 = egui::Color32::from_rgb(16, 18, 23);

pub fn show(ctx: &egui::Context, app: &mut ViperApp) {
    show_album_browser(ctx, app);
}

fn show_album_browser(ctx: &egui::Context, app: &mut ViperApp) {
    show_sort_toolbar(ctx, app);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(CANVAS).inner_margin(0))
        .show(ctx, |ui| show_grid(ctx, ui, app));
}

fn show_sort_toolbar(ctx: &egui::Context, app: &mut ViperApp) {
    egui::TopBottomPanel::top("album-sort-toolbar")
        .exact_height(48.0)
        .frame(egui::Frame::new().fill(CANVAS).inner_margin(egui::Margin {
            left: 20,
            right: 20,
            top: 4,
            bottom: 12,
        }))
        .show(ctx, |ui| {
            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let active = app.settings.album_sort;
                    let mut changed = None;
                    let sort_rect = ui
                        .scope(|ui| {
                            style_sort_menu(ui);
                            ui.spacing_mut().button_padding = egui::vec2(10.0, 5.0);
                            egui::ComboBox::from_id_salt("album-sort")
                                .selected_text(
                                    egui::RichText::new(active.label())
                                        .size(13.0)
                                        .color(egui::Color32::from_rgb(221, 225, 236)),
                                )
                                .width(140.0)
                                .show_ui(ui, |ui| {
                                    for sort in AlbumSort::ALL {
                                        if ui
                                            .selectable_label(active == sort, sort.label())
                                            .clicked()
                                        {
                                            changed = Some(sort);
                                            ui.close_menu();
                                        }
                                    }
                                })
                                .response
                                .rect
                        })
                        .inner;
                    if let Some(sort) = changed {
                        app.album_sort_changed(sort);
                    }
                    let (slot, _) =
                        ui.allocate_exact_size(egui::vec2(34.0, 32.0), egui::Sense::hover());
                    let rect = egui::Rect::from_center_size(
                        egui::pos2(slot.center().x, sort_rect.center().y),
                        slot.size(),
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
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        if app.settings.sort_ascending {
                            egui_phosphor::regular::SORT_ASCENDING
                        } else {
                            egui_phosphor::regular::SORT_DESCENDING
                        },
                        egui::FontId::new(18.0, egui::FontFamily::Name("phosphor".into())),
                        if response.hovered() {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_gray(190)
                        },
                    );
                    if response
                        .on_hover_text(if app.settings.sort_ascending {
                            "Ascending"
                        } else {
                            "Descending"
                        })
                        .clicked()
                    {
                        app.toggle_sort_direction();
                    }
                },
            );
        });
}

fn style_sort_menu(ui: &mut egui::Ui) {
    let widgets = &mut ui.visuals_mut().widgets;
    let fill = egui::Color32::from_rgb(27, 31, 41);
    let border = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(43, 48, 61));
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
}

fn show_grid(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut ViperApp) {
    ui.spacing_mut().scroll.bar_outer_margin = 10.0;
    ui.spacing_mut().scroll.dormant_background_opacity = 0.0;
    ui.spacing_mut().scroll.active_background_opacity = 0.0;
    ui.spacing_mut().scroll.interact_background_opacity = 0.0;
    let bounds = ui.max_rect();
    egui::ScrollArea::vertical()
        .id_salt("album-grid")
        .auto_shrink([false, false])
        .drag_to_scroll(false)
        .scroll_bar_rect(bounds.shrink2(egui::vec2(0.0, 14.0)))
        .show_viewport(ui, |ui, viewport| {
            let width = ui.available_width();
            let albums = app.filtered_album_indices();
            if albums.is_empty() {
                show_empty_grid(ctx, ui, app, width);
                return;
            }
            show_album_rows(ctx, ui, app, &albums, viewport, width);
        });
}

fn show_empty_grid(ctx: &egui::Context, ui: &mut egui::Ui, app: &ViperApp, width: f32) {
    if app.scanning && app.library.albums.is_empty() && app.search.trim().is_empty() {
        show_skeleton_albums(ctx, ui, width);
        return;
    }
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
        )
    });
}

fn show_album_rows(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    app: &mut ViperApp,
    albums: &[usize],
    viewport: egui::Rect,
    width: f32,
) {
    const TOP_PADDING: f32 = 16.0;
    const ROW_GAP: f32 = 10.0;
    let (columns, card_width, art_size, gap, left) = grid_metrics(width);
    let row_height = art_size + 81.0;
    let row_step = row_height + ROW_GAP;
    let rows = albums.len().div_ceil(columns);
    let content = ui.max_rect();
    ui.set_height(TOP_PADDING + rows as f32 * row_step);
    let first = (((viewport.min.y - TOP_PADDING).max(0.0) / row_step).floor() as usize).min(rows);
    let last =
        ((((viewport.max.y - TOP_PADDING).max(0.0) / row_step).ceil() as usize) + 1).min(rows);
    for row in first..last {
        let row_rect = egui::Rect::from_min_size(
            egui::pos2(
                content.left(),
                content.top() + TOP_PADDING + row as f32 * row_step,
            ),
            egui::vec2(ui.available_width(), row_height),
        );
        let mut row_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Min)),
        );
        row_ui.add_space(left);
        row_ui.spacing_mut().item_spacing.x = gap;
        let start = row * columns;
        for &album_index in albums.iter().skip(start).take(columns) {
            album_card::show(
                ctx,
                &mut row_ui,
                app,
                album_index,
                card_width,
                art_size,
                row_height,
            );
        }
    }
}

fn show_skeleton_albums(ctx: &egui::Context, ui: &mut egui::Ui, width: f32) {
    let (columns, card_width, art_size, gap, left) = grid_metrics(width);
    let card_height = art_size + 81.0;
    let pulse = ((ctx.input(|input| input.time) * 2.2).sin() * 0.5 + 0.5) as f32;
    let cover_color = egui::Color32::from_gray((34.0 + pulse * 12.0) as u8);
    let line_color = egui::Color32::from_gray((43.0 + pulse * 10.0) as u8);
    ui.add_space(16.0);
    for _ in 0..3 {
        ui.horizontal(|ui| {
            ui.add_space(left);
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
                                let art = ui.allocate_space(egui::vec2(art_size, art_size)).1;
                                ui.painter().rect_filled(art, 8.0, cover_color);
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

fn grid_metrics(width: f32) -> (usize, f32, f32, f32, f32) {
    const LEFT_PADDING: f32 = 12.0;
    const RIGHT_PADDING: f32 = 18.0;
    const MIN_GAP: f32 = 12.0;
    const CARD_WIDTH: f32 = 166.0;
    const MIN_COLUMNS: usize = 3;
    let area = (width - LEFT_PADDING - RIGHT_PADDING).max(0.0);
    let natural_columns = ((area + MIN_GAP) / (CARD_WIDTH + MIN_GAP)).floor() as usize;
    let columns = natural_columns.max(MIN_COLUMNS);
    let card_width = if natural_columns < MIN_COLUMNS {
        ((area - (columns - 1) as f32 * MIN_GAP) / columns as f32).max(48.0)
    } else {
        CARD_WIDTH
    };
    let art_size = (card_width - 12.0).max(36.0);
    let gap = ((area - columns as f32 * card_width) / (columns - 1) as f32).max(MIN_GAP);
    (columns, card_width, art_size, gap, LEFT_PADDING)
}
