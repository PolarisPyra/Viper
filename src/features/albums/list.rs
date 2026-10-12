use eframe::egui;

use super::{
    open_album, paint_cover_with_opacity, show_cover_placeholder_with_opacity, show_shimmer_rect,
    AlbumScreen,
};

pub(super) fn show_header(ui: &mut egui::Ui, width: f32) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    const HEADER_HEIGHT: f32 = 32.0;
    let header = ui
        .allocate_exact_size(egui::vec2(width, HEADER_HEIGHT), egui::Sense::hover())
        .0;
    let columns = egui::Rect::from_min_max(
        egui::pos2(header.left() + 8.0, header.top()),
        egui::pos2(header.right() - 8.0, header.bottom()),
    );
    for (x, label) in [
        (columns.left() + 82.0, "Title"),
        (columns.left() + width * 0.53, "Artist"),
        (columns.right() - 82.0, "Date"),
    ] {
        ui.painter().text(
            egui::pos2(x, header.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            colors.muted,
        );
    }
    ui.painter().line_segment(
        [header.left_bottom(), header.right_bottom()],
        egui::Stroke::new(1.0_f32, colors.border),
    );
}

pub(super) fn show_album_list(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    screen: &mut AlbumScreen<'_>,
    albums: &[usize],
    page_start: usize,
    viewport: egui::Rect,
    width: f32,
) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    const ROW_HEIGHT: f32 = 72.0;
    const ROW_GAP: f32 = 1.0;
    let row_step = ROW_HEIGHT + ROW_GAP;
    let rows = albums.len();
    let content = ui.max_rect();
    let rows_top = content.top();
    ui.set_height(rows as f32 * row_step);
    let first = ((viewport.min.y.max(0.0) / row_step).floor() as usize).min(rows);
    let last = ((viewport.max.y.max(0.0) / row_step).ceil() as usize + 1).min(rows);
    let entry_opacity = screen.album_entry_opacity(ctx, page_start);

    for row in first..last {
        let album_index = albums[row];
        let rect = egui::Rect::from_min_size(
            egui::pos2(content.left() + 8.0, rows_top + row as f32 * row_step),
            egui::vec2((width - 16.0).max(0.0), ROW_HEIGHT),
        );
        let selected = screen.library.view.selected_album == Some(album_index);
        let response = ui.interact(
            rect,
            ui.id().with(("album-list-row", album_index)),
            egui::Sense::click(),
        );
        if selected || response.hovered() {
            ui.painter().rect_filled(
                rect.shrink2(egui::vec2(0.0, 2.0)),
                0.0,
                if selected {
                    colors.selected.gamma_multiply(entry_opacity)
                } else {
                    colors.hover.gamma_multiply(entry_opacity)
                },
            );
        }

        let cover = screen.album_texture(ctx, album_index);
        let cover_opacity = screen.album_texture_opacity(ctx, album_index);
        let Some(album) = screen.library.model.albums.get(album_index) else {
            continue;
        };
        let cover_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 10.0, rect.center().y - 28.0),
            egui::vec2(56.0, 56.0),
        );
        if let Some(cover) = &cover {
            paint_cover_with_opacity(ui, cover, cover_rect, cover_opacity * entry_opacity);
        } else {
            show_cover_placeholder_with_opacity(ui, cover_rect, 6.0, entry_opacity);
            ui.painter().text(
                cover_rect.center(),
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::DISC,
                egui::FontId::new(20.0, egui::FontFamily::Name("phosphor".into())),
                colors.muted.gamma_multiply(entry_opacity),
            );
        }

        let title_x = rect.left() + 82.0;
        let artist_x = rect.left() + width * 0.53;
        let date_x = rect.right() - 82.0;
        let title_width = (artist_x - title_x - 12.0).max(0.0);
        let artist_width = (date_x - artist_x - 12.0).max(0.0);
        let date_width = (rect.right() - date_x - 8.0).max(0.0);
        let year = album
            .tracks
            .iter()
            .filter_map(|index| screen.library.model.tracks.get(*index)?.release_year)
            .min()
            .map_or_else(|| "—".to_owned(), |year| year.to_string());

        for (x, column_width, text, color) in [
            (
                title_x,
                title_width,
                album.title.as_str(),
                colors.text.gamma_multiply(entry_opacity),
            ),
            (
                artist_x,
                artist_width,
                album.artist.as_str(),
                colors.muted.gamma_multiply(entry_opacity),
            ),
            (
                date_x,
                date_width,
                year.as_str(),
                colors.muted.gamma_multiply(entry_opacity),
            ),
        ] {
            let column_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.top()),
                egui::vec2(column_width, ROW_HEIGHT),
            );
            ui.painter().with_clip_rect(column_rect).text(
                egui::pos2(x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(13.0),
                color,
            );
        }
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0_f32, colors.border.gamma_multiply(entry_opacity)),
        );
        if response.clicked() {
            open_album(ctx, screen, album_index);
        }
    }
}

pub(super) fn show_skeleton_list(ctx: &egui::Context, ui: &mut egui::Ui) {
    for _ in 0..6 {
        let (row, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 72.0), egui::Sense::hover());
        show_shimmer_rect(
            ui,
            egui::Rect::from_min_size(row.min + egui::vec2(10.0, 8.0), egui::vec2(56.0, 56.0)),
            6.0,
        );
        let title = egui::Rect::from_min_size(
            row.min + egui::vec2(82.0, 12.0),
            egui::vec2((row.width() * 0.35).max(100.0), 10.0),
        );
        show_shimmer_rect(ui, title, 4.0);
        let artist = egui::Rect::from_min_size(
            row.min + egui::vec2(row.width() * 0.53, 12.0),
            egui::vec2((row.width() * 0.2).max(60.0), 10.0),
        );
        show_shimmer_rect(ui, artist, 4.0);
        ui.add_space(1.0);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}
