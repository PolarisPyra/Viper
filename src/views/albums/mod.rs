use crate::{app::ViperApp, storage::settings::AlbumLayout};
use eframe::egui;

mod grid;
mod list;

const CANVAS: egui::Color32 = egui::Color32::from_rgb(16, 18, 23);

pub fn show(ctx: &egui::Context, app: &mut ViperApp) {
    crate::components::view_toolbar::show(ctx, app);
    let layout = app.settings.album_layout;
    let albums = app.filtered_album_indices();
    let page = crate::components::pagination::show(
        ctx,
        albums.len(),
        match layout {
            AlbumLayout::Grid => 25,
            AlbumLayout::List => crate::components::pagination::ALBUMS_PER_PAGE,
        },
        (
            matches!(layout, AlbumLayout::Grid),
            app.search.as_str(),
            app.settings.album_sort.label(),
            app.settings.sort_ascending,
        ),
    );
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(CANVAS).inner_margin(0))
        .show(ctx, |ui| show_grid(ctx, ui, app, &albums, page));
}

fn show_grid(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    app: &mut ViperApp,
    albums: &[usize],
    page: std::ops::Range<usize>,
) {
    ui.spacing_mut().scroll.bar_outer_margin = 10.0;
    ui.spacing_mut().scroll.dormant_background_opacity = 0.0;
    ui.spacing_mut().scroll.active_background_opacity = 0.0;
    ui.spacing_mut().scroll.interact_background_opacity = 0.0;

    let layout = app.settings.album_layout;
    let width = ui.available_width();
    if layout == AlbumLayout::List {
        list::show_header(ui, width);
    }

    let bounds = ui.max_rect();
    egui::ScrollArea::vertical()
        .id_salt(match layout {
            AlbumLayout::Grid => ("album-grid-scroll", page.start),
            AlbumLayout::List => ("album-list-scroll", page.start),
        })
        .auto_shrink([false, false])
        .drag_to_scroll(false)
        .scroll_bar_rect(bounds.shrink2(egui::vec2(0.0, 14.0)))
        .show_viewport(ui, |ui, viewport| {
            let width = ui.available_width();
            if albums.is_empty() {
                show_empty(ctx, ui, app, width);
                return;
            }
            match layout {
                AlbumLayout::Grid => grid::show_album_rows(
                    ctx,
                    ui,
                    app,
                    &albums[page.clone()],
                    viewport,
                    width,
                ),
                AlbumLayout::List => {
                    list::show_album_list(ctx, ui, app, &albums[page.clone()], viewport, width)
                }
            }
        });
}

fn show_empty(ctx: &egui::Context, ui: &mut egui::Ui, app: &ViperApp, width: f32) {
    if app.scanning && app.library.albums.is_empty() && app.search.trim().is_empty() {
        match app.settings.album_layout {
            AlbumLayout::Grid => grid::show_skeleton_albums(ctx, ui, width),
            AlbumLayout::List => list::show_skeleton_list(ctx, ui),
        }
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

pub(super) fn open_album(ctx: &egui::Context, app: &mut ViperApp, index: usize) {
    if app.selected_album == Some(index) {
        if let Some(album) = app.library.albums.get(index).cloned() {
            app.playback.play_album(&album, &app.library.tracks);
        }
    } else {
        app.selected_track = None;
        app.selected_album = Some(index);
    }
    app.show_album_details = true;
    ctx.request_repaint();
}

pub(super) fn paint_cover(ui: &egui::Ui, texture: &egui::TextureHandle, rect: egui::Rect) {
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}
