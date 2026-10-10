use crate::{
    features::{
        library::{artwork::ArtworkCache, state::LibraryFeature},
        playback::Playback,
    },
    platform::persistence::settings::Settings,
    workbench::WorkbenchState,
};
use eframe::egui;

const MIN_PANEL_WIDTH: f32 = 260.0;
const MAX_PANEL_WIDTH: f32 = 520.0;

pub(crate) fn show(
    ctx: &egui::Context,
    workbench: &mut WorkbenchState,
    library_feature: &mut LibraryFeature,
    playback: &mut Playback,
    settings: &mut Settings,
    artwork_cache: &mut ArtworkCache,
) -> Option<String> {
    let colors = crate::shared::ui::theme::colors(ctx);
    if !workbench.show_album_details {
        return None;
    }

    let Some(album_index) = library_feature.view.selected_album else {
        return None;
    };
    if library_feature.model.albums.get(album_index).is_none() {
        return None;
    }

    let cover = library_feature
        .model
        .albums
        .get(album_index)
        .and_then(|album| {
            let bytes = album.art.as_deref()?;
            artwork_cache.texture(ctx, album_index, bytes)
        });
    let album = &library_feature.model.albums[album_index];
    let album_key = crate::features::library::sorting::album_sort_key(&album.artist, &album.title);
    let width = settings
        .right_panel_width
        .clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
    let mut changed_settings = false;
    let library = &library_feature.model;
    let selected_track = &mut library_feature.view.selected_track;

    let output = egui::SidePanel::right("album-tracklist-side-panel")
        .resizable(true)
        .width_range(MIN_PANEL_WIDTH..=MAX_PANEL_WIDTH)
        .default_width(width)
        .frame(egui::Frame::new().fill(colors.panel).inner_margin(0))
        .show(ctx, |ui| {
            let panel_rect = ui.max_rect();
            draw_track_panel_background(ui);
            let panel_height = panel_rect.height();
            egui::Frame::new()
                .fill(colors.panel)
                .inner_margin(egui::Margin::symmetric(20, 18))
                .show(ui, |ui| {
                    let content_height = (panel_height - 36.0).max(0.0);
                    let content_width = (panel_rect.width() - 40.0).max(0.0);
                    ui.set_width(content_width);
                    ui.set_min_height(content_height);
                    ui.set_max_height(content_height);

                    let release_year = album
                        .tracks
                        .iter()
                        .filter_map(|index| library.tracks.get(*index)?.release_year)
                        .min()
                        .map_or_else(|| "—".to_owned(), |year| year.to_string());
                    draw_track_panel_heading(
                        ui,
                        album.tracks.len(),
                        &release_year,
                        &mut workbench.show_album_details,
                    );
                    ui.add_space(15.0);
                    draw_album_summary(
                        ui,
                        cover.as_ref(),
                        &album.title,
                        &album.artist,
                        &album_key,
                        settings,
                        &mut changed_settings,
                    );
                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(7.0);
                    show_tracks(
                        ctx,
                        ui,
                        library,
                        playback,
                        selected_track,
                        album_index,
                        content_height,
                    );
                });
        });

    let mut save_error = None;
    if changed_settings {
        library_feature.view.album_sort_dirty = true;
        save_error = save_settings(settings);
    }

    let resize_id = egui::Id::new("album-tracklist-side-panel").with("__resize");
    let resized = ctx
        .read_response(resize_id)
        .is_some_and(|response| response.drag_stopped_by(egui::PointerButton::Primary));
    if resized {
        settings.right_panel_width = output
            .response
            .rect
            .width()
            .clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
        save_error = save_settings(settings);
    }
    save_error
}

fn draw_track_panel_background(ui: &egui::Ui) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, colors.panel);
    ui.painter().line_segment(
        [rect.left_top(), rect.left_bottom()],
        egui::Stroke::new(1.0_f32, colors.border),
    );
}

fn draw_track_panel_heading(
    ui: &mut egui::Ui,
    track_count: usize,
    release_year: &str,
    open: &mut bool,
) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("ALBUM TRACKS")
                    .size(10.0)
                    .strong()
                    .color(colors.subtle),
            );
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    egui::RichText::new(format!("{track_count} songs"))
                        .size(12.0)
                        .color(colors.muted),
                );
                ui.label(egui::RichText::new("·").size(12.0).color(colors.subtle));
                ui.label(
                    egui::RichText::new(release_year)
                        .size(12.0)
                        .color(colors.muted),
                );
            });
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
            draw_action_hover(ui, rect, response.hovered());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::X,
                egui::FontId::new(20.0, egui::FontFamily::Name("phosphor".into())),
                if response.hovered() {
                    colors.text
                } else {
                    colors.muted
                },
            );
            if response.on_hover_text("Close album tracks").clicked() {
                *open = false;
            }
        });
    });
}

fn draw_album_summary(
    ui: &mut egui::Ui,
    cover: Option<&egui::TextureHandle>,
    title: &str,
    artist: &str,
    key: &str,
    settings: &mut crate::platform::persistence::settings::Settings,
    changed: &mut bool,
) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    ui.horizontal(|ui| {
        if let Some(texture) = cover {
            let rect = ui.allocate_space(egui::vec2(80.0, 80.0)).1;
            paint_cover(ui, texture, rect);
            ui.add_space(12.0);
        }
        ui.vertical(|ui| {
            ui.add_space(12.0);
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.add(
                egui::Label::new(egui::RichText::new(title).size(18.0).color(colors.text))
                    .truncate(),
            );
            ui.add(
                egui::Label::new(egui::RichText::new(artist).size(13.0).color(colors.muted))
                    .truncate(),
            );
            ui.add_space(3.0);
            draw_album_preferences(ui, key, settings, changed);
        });
    });
}

fn draw_album_preferences(
    ui: &mut egui::Ui,
    key: &str,
    settings: &mut crate::platform::persistence::settings::Settings,
    changed: &mut bool,
) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    let rating = settings.album_ratings.get(key).copied().unwrap_or_default();
    let favorite = settings.favorite_albums.contains(key);
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        egui::vec2(width, 24.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let row = ui.max_rect();
            ui.spacing_mut().item_spacing.x = 1.0;
            for value in 1..=5 {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(22.0, 24.0), egui::Sense::click());
                let filled = value <= rating;
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if filled {
                        egui_phosphor::fill::STAR
                    } else {
                        egui_phosphor::regular::STAR
                    },
                    egui::FontId::new(
                        18.0,
                        egui::FontFamily::Name(
                            if filled { "phosphor-fill" } else { "phosphor" }.into(),
                        ),
                    ),
                    if filled {
                        colors.warning
                    } else {
                        colors.subtle
                    },
                );
                let tooltip = if value == rating {
                    "Clear rating".to_owned()
                } else {
                    format!("Rate {value} out of 5")
                };
                if response.on_hover_text(tooltip).clicked() {
                    if value == rating {
                        settings.album_ratings.remove(key);
                    } else {
                        settings.album_ratings.insert(key.to_owned(), value);
                    }
                    *changed = true;
                }
            }
            let heart = egui::Rect::from_center_size(
                egui::pos2(row.right() - 15.0, row.center().y),
                egui::vec2(30.0, 30.0),
            );
            let response = ui.interact(
                heart,
                ui.id().with("album-favorite-heart"),
                egui::Sense::click(),
            );
            draw_action_hover(ui, heart, response.hovered());
            ui.painter().text(
                heart.center(),
                egui::Align2::CENTER_CENTER,
                if favorite {
                    egui_phosphor::fill::HEART
                } else {
                    egui_phosphor::regular::HEART
                },
                egui::FontId::new(
                    20.0,
                    egui::FontFamily::Name(
                        if favorite {
                            "phosphor-fill"
                        } else {
                            "phosphor"
                        }
                        .into(),
                    ),
                ),
                if favorite {
                    colors.danger
                } else if response.hovered() {
                    colors.text
                } else {
                    colors.muted
                },
            );
            if response
                .on_hover_text(if favorite {
                    "Remove from favorites"
                } else {
                    "Add to favorites"
                })
                .clicked()
            {
                if favorite {
                    settings.favorite_albums.remove(key);
                } else {
                    settings.favorite_albums.insert(key.to_owned());
                }
                *changed = true;
            }
        },
    );
}

fn draw_action_hover(ui: &egui::Ui, rect: egui::Rect, hovered: bool) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    if hovered {
        ui.painter().rect_filled(rect, 7.0, colors.hover);
    }
}

fn save_settings(settings: &Settings) -> Option<String> {
    settings
        .save()
        .err()
        .map(|error| format!("Could not save settings: {error}"))
}

fn paint_cover(ui: &egui::Ui, texture: &egui::TextureHandle, rect: egui::Rect) {
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

fn show_tracks(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    library: &crate::features::library::Library,
    playback: &mut crate::features::playback::Playback,
    selected_track: &mut Option<usize>,
    album_index: usize,
    content_height: f32,
) {
    let Some(album) = library.albums.get(album_index) else {
        return;
    };
    let viewport = ui.available_rect_before_wrap();
    let pointer_inside = ctx.input(|input| {
        input
            .pointer
            .hover_pos()
            .is_some_and(|position| viewport.contains(position))
    });
    egui::ScrollArea::vertical()
        .id_salt("album-popout-tracks")
        .drag_to_scroll(false)
        .enable_scrolling(pointer_inside)
        .max_height((content_height - 190.0).max(100.0))
        .show(ui, |ui| {
            ui.add_space(12.0);
            for (position, &track_index) in album.tracks.iter().enumerate() {
                let Some(track) = library.tracks.get(track_index) else {
                    continue;
                };
                draw_track_row(
                    ui,
                    library,
                    playback,
                    selected_track,
                    album_index,
                    track_index,
                    position,
                    track,
                );
            }
            ui.add_space(18.0);
        });
}

fn draw_track_row(
    ui: &mut egui::Ui,
    library: &crate::features::library::Library,
    playback: &mut crate::features::playback::Playback,
    selected_track: &mut Option<usize>,
    album_index: usize,
    track_index: usize,
    position: usize,
    track: &crate::features::library::Track,
) {
    let colors = crate::shared::ui::theme::colors(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 50.0), egui::Sense::click());
    let highlight = egui::Rect::from_min_max(rect.min, rect.max - egui::vec2(12.0, 0.0));
    let playing = playback.current == Some(track_index);
    let selected = *selected_track == Some(track_index);
    if selected {
        ui.painter().rect_filled(highlight, 3.0, colors.selected);
    }

    let leading =
        egui::Rect::from_min_size(rect.min + egui::vec2(18.0, 14.0), egui::vec2(20.0, 20.0));
    let (glyph, color) = if playing {
        (
            if playback.is_playing() {
                egui_phosphor::regular::PAUSE
            } else {
                egui_phosphor::regular::PLAY
            },
            colors.accent,
        )
    } else {
        ("", colors.subtle)
    };
    ui.painter().text(
        leading.center(),
        egui::Align2::CENTER_CENTER,
        if playing {
            glyph.to_owned()
        } else {
            format!("{:02}", position + 1)
        },
        egui::FontId::new(
            if playing { 18.0 } else { 11.0 },
            if playing {
                egui::FontFamily::Name("phosphor".into())
            } else {
                egui::FontFamily::Proportional
            },
        ),
        color,
    );

    let text_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(50.0, 7.0),
        rect.max - egui::vec2(58.0, 5.0),
    );
    let mut text_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(text_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    text_ui.add(
        egui::Label::new(
            egui::RichText::new(&track.title)
                .size(12.0)
                .color(if selected { colors.accent } else { colors.text }),
        )
        .truncate(),
    );
    text_ui.add(
        egui::Label::new(
            egui::RichText::new(&track.artist)
                .size(10.0)
                .color(colors.muted),
        )
        .truncate(),
    );
    if let Some(duration) = track.duration_ms {
        ui.painter().text(
            highlight.right_center() - egui::vec2(10.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            format_duration(duration),
            egui::FontId::proportional(10.0),
            colors.subtle,
        );
    }
    if response.clicked() {
        if selected {
            if let Some(album) = library.albums.get(album_index) {
                playback.play_track_in_album(album, &library.tracks, track_index);
            }
        } else {
            *selected_track = Some(track_index);
        }
    }
}

fn format_duration(milliseconds: u64) -> String {
    let seconds = milliseconds / 1_000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
