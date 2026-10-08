use crate::app::ViperApp;
use eframe::egui;

pub(crate) fn show(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    app: &mut ViperApp,
    index: usize,
    card_width: f32,
    art_size: f32,
    card_height: f32,
) {
    if app.library.albums.get(index).is_none() {
        return;
    }
    let cover = app.album_texture(ctx, index);
    let album = &app.library.albums[index];
    let selected = app.selected_album == Some(index);
    let response = ui.allocate_ui_with_layout(
        egui::vec2(card_width, card_height),
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
                    let art = ui.allocate_space(egui::vec2(art_size, art_size)).1;
                    if ui.is_rect_visible(art) {
                        if let Some(cover) = &cover {
                            paint_cover(ui, cover, art);
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
                                "{} · {} tracks",
                                album.artist,
                                album.tracks.len()
                            ))
                            .color(egui::Color32::from_gray(145))
                            .size(12.0),
                        )
                        .truncate(),
                    );
                });
        },
    );
    if response.response.interact(egui::Sense::click()).clicked() {
        if app.selected_album == Some(index) {
            let album = app.library.albums[index].clone();
            app.playback.play_album(&album, &app.library.tracks);
        } else {
            app.selected_track = None;
            app.selected_album = Some(index);
        }
        app.show_album_details = true;
        ctx.request_repaint();
    }
}

fn paint_cover(ui: &egui::Ui, texture: &egui::TextureHandle, rect: egui::Rect) {
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}
