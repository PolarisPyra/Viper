pub(crate) mod toolbar;
pub(crate) mod track_list;
use crate::{
    features::{
        library::{artwork::ArtworkCache, state::LibraryFeature},
        playback::Playback,
    },
    platform::persistence::settings::{AlbumLayout, Settings},
    workbench::WorkbenchState,
};
use eframe::egui;

mod grid;
mod list;

const CANVAS: egui::Color32 = egui::Color32::from_rgb(16, 18, 23);

pub(crate) struct AlbumScreen<'a> {
    pub(crate) workbench: &'a mut WorkbenchState,
    pub(crate) library: &'a mut LibraryFeature,
    pub(crate) playback: &'a mut Playback,
    pub(crate) settings: &'a mut Settings,
    pub(crate) artwork_cache: &'a mut ArtworkCache,
    pub(crate) error: &'a mut Option<String>,
}

impl AlbumScreen<'_> {
    pub(crate) fn album_texture(
        &mut self,
        ctx: &egui::Context,
        album_index: usize,
    ) -> Option<egui::TextureHandle> {
        let bytes = self.library.model.albums.get(album_index)?.art.as_deref()?;
        self.artwork_cache.texture(ctx, album_index, bytes)
    }
}

pub(crate) fn show(ctx: &egui::Context, mut screen: AlbumScreen<'_>) {
    crate::features::albums::toolbar::show(
        ctx,
        screen.settings,
        &mut screen.library.view,
        screen.error,
    );
    let layout = screen.settings.album_layout;
    let albums = screen.library.filtered_album_indices(screen.settings);
    let page = crate::shared::ui::pagination::show(
        ctx,
        albums.len(),
        crate::shared::ui::pagination::ALBUMS_PER_PAGE,
        (
            matches!(layout, AlbumLayout::Grid),
            screen.library.view.search.as_str(),
            screen.settings.album_sort.label(),
            screen.settings.sort_ascending,
        ),
    );
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(CANVAS).inner_margin(0))
        .show(ctx, |ui| show_grid(ctx, ui, &mut screen, &albums, page));
}

fn show_grid(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    screen: &mut AlbumScreen<'_>,
    albums: &[usize],
    page: std::ops::Range<usize>,
) {
    ui.spacing_mut().scroll.bar_outer_margin = 10.0;
    ui.spacing_mut().scroll.dormant_background_opacity = 0.0;
    ui.spacing_mut().scroll.active_background_opacity = 0.0;
    ui.spacing_mut().scroll.interact_background_opacity = 0.0;

    let layout = screen.settings.album_layout;
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
                show_empty(ctx, ui, screen, width);
                return;
            }
            match layout {
                AlbumLayout::Grid => {
                    grid::show_album_rows(ctx, ui, screen, &albums[page.clone()], viewport, width)
                }
                AlbumLayout::List => {
                    list::show_album_list(ctx, ui, screen, &albums[page.clone()], viewport, width)
                }
            }
        });
}

fn show_empty(ctx: &egui::Context, ui: &mut egui::Ui, screen: &AlbumScreen<'_>, width: f32) {
    if screen.library.scanning
        && screen.library.model.albums.is_empty()
        && screen.library.view.search.trim().is_empty()
    {
        match screen.settings.album_layout {
            AlbumLayout::Grid => grid::show_skeleton_albums(ctx, ui, width),
            AlbumLayout::List => list::show_skeleton_list(ctx, ui),
        }
        return;
    }
    ui.add_space(70.0);
    let message = if screen.library.model.albums.is_empty() {
        "Your albums will appear here"
    } else if screen.library.scanning && screen.library.view.search.trim().is_empty() {
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

pub(super) fn open_album(ctx: &egui::Context, screen: &mut AlbumScreen<'_>, index: usize) {
    if screen.library.view.selected_album == Some(index) {
        if let Some(album) = screen.library.model.albums.get(index).cloned() {
            screen
                .playback
                .play_album(&album, &screen.library.model.tracks);
        }
    } else {
        screen.library.view.selected_track = None;
        screen.library.view.selected_album = Some(index);
    }
    screen.workbench.show_album_details = true;
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
