use crate::{
    app::{MusicApp, Page},
    library::sorting::album_sort_key,
    library::Album,
};
use eframe::egui;
use std::collections::BTreeMap;

const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(16, 18, 23);
const PANEL: egui::Color32 = egui::Color32::from_rgb(25, 29, 39);
const TEXT: egui::Color32 = egui::Color32::from_rgb(231, 233, 241);
const MUTED: egui::Color32 = egui::Color32::from_rgb(145, 152, 169);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(170, 145, 255);

pub fn show(ctx: &egui::Context, app: &mut MusicApp) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(BACKGROUND)
                .inner_margin(egui::Margin::symmetric(24, 0)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("home-content")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new("Your library")
                            .size(30.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Pick up your current track or browse albums.")
                            .size(14.0)
                            .color(MUTED),
                    );
                    ui.add_space(24.0);

                    if app.settings.music_path.is_none() {
                        onboarding(ui, app);
                        return;
                    }

                    let filtered = app.filtered_album_indices();
                    if !app.search.trim().is_empty() && filtered.is_empty() {
                        empty_state(
                            ui,
                            "No albums found",
                            "Try another search, or clear it to see your library.",
                            "Clear search",
                            |app| app.search.clear(),
                            app,
                        );
                        return;
                    }

                    if app.library.albums.is_empty() {
                        if app.scanning {
                            empty_state(
                                ui,
                                "Finding your music",
                                "Your albums will show up here as the folder is scanned.",
                                "Browse albums",
                                |app| app.page = Page::Albums,
                                app,
                            );
                        } else {
                            empty_state(
                                ui,
                                "Your library is empty",
                                "Choose a folder with supported audio files to get started.",
                                "Choose Music Folder",
                                MusicApp::choose_folder,
                                app,
                            );
                        }
                        return;
                    }

                    let recently_added = albums_by_timestamp(
                        &filtered,
                        &app.library.albums,
                        &app.settings.album_added,
                    );
                    let recently_played = albums_by_timestamp(
                        &filtered,
                        &app.library.albums,
                        &app.settings.album_last_played,
                    );

                    let mut favorites: Vec<_> = filtered
                        .iter()
                        .copied()
                        .filter(|&index| {
                            app.library.albums.get(index).is_some_and(|album| {
                                app.settings
                                    .favorite_albums
                                    .contains(&album_sort_key(&album.artist, &album.title))
                            })
                        })
                        .collect();
                    favorites.sort_by_key(|&index| {
                        app.library
                            .albums
                            .get(index)
                            .map(|album| (album.title.to_lowercase(), album.artist.to_lowercase()))
                    });

                    show_continue_listening(ui, app);
                    if !recently_added.is_empty() {
                        album_section(ui, ctx, app, "Recently added", &recently_added);
                    }
                    if !recently_played.is_empty() {
                        album_section(ui, ctx, app, "Recently played", &recently_played);
                    }
                    if favorites.is_empty() {
                        section_heading(ui, "Favorites");
                        ui.label(
                            egui::RichText::new(
                                "Browse albums and mark a favorite to keep it close at hand.",
                            )
                            .size(13.0)
                            .color(MUTED),
                        );
                        ui.add_space(10.0);
                        if ui.small_button("Browse albums").clicked() {
                            app.page = Page::Albums;
                        }
                    } else {
                        album_section(ui, ctx, app, "Favorites", &favorites);
                    }
                    ui.add_space(24.0);
                });
        });
}

fn albums_by_timestamp(
    indices: &[usize],
    albums: &[Album],
    timestamps: &BTreeMap<String, u64>,
) -> Vec<usize> {
    let mut albums_with_timestamps: Vec<_> = indices
        .iter()
        .filter_map(|&index| {
            let album = albums.get(index)?;
            let key = album_sort_key(&album.artist, &album.title);
            Some((index, *timestamps.get(&key)?))
        })
        .collect();

    albums_with_timestamps.sort_by_key(|(_, timestamp)| std::cmp::Reverse(*timestamp));
    albums_with_timestamps
        .into_iter()
        .map(|(index, _)| index)
        .collect()
}

fn onboarding(ui: &mut egui::Ui, app: &mut MusicApp) {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(14.0)
        .inner_margin(egui::Margin::same(24))
        .show(ui, |ui| {
            ui.set_max_width(720.0);
            ui.set_min_height(185.0);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(72.0, 72.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 14.0, egui::Color32::from_rgb(48, 42, 72));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    egui_phosphor::regular::MUSIC_NOTE,
                    egui::FontId::new(34.0, egui::FontFamily::Name("phosphor".into())),
                    ACCENT,
                );
                ui.add_space(20.0);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Start with your music folder")
                            .size(20.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.add_space(7.0);
                    ui.label(
                        egui::RichText::new(
                            "Choose where your music is stored. Your albums and artwork will appear here.",
                        )
                        .size(13.0)
                        .color(MUTED),
                    );
                    ui.add_space(14.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Choose Music Folder")
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(111, 83, 205))
                            .corner_radius(8.0)
                            .min_size(egui::vec2(190.0, 38.0)),
                        )
                        .clicked()
                    {
                        app.choose_folder();
                    }
                });
            });
        });
}

fn empty_state(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    action: &str,
    on_click: impl FnOnce(&mut MusicApp),
    app: &mut MusicApp,
) {
    egui::Frame::new()
        .fill(PANEL)
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(24))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).size(19.0).strong().color(TEXT));
            ui.add_space(5.0);
            ui.label(egui::RichText::new(description).size(13.0).color(MUTED));
            ui.add_space(12.0);
            if ui.button(action).clicked() {
                on_click(app);
            }
        });
}

fn show_continue_listening(ui: &mut egui::Ui, app: &mut MusicApp) {
    section_heading(ui, "Continue listening");
    if let Some(track_index) = app.playback.current {
        let Some(track) = app.library.tracks.get(track_index) else {
            return;
        };
        let title = track.title.clone();
        let artist = track.artist.clone();
        let album_index = app.library.track_album.get(track_index).copied();
        let album_name = album_index
            .and_then(|index| app.library.albums.get(index))
            .map(|album| album.title.clone())
            .unwrap_or_default();

        egui::Frame::new()
            .fill(PANEL)
            .corner_radius(12.0)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(index) = album_index {
                        if let Some(texture) = app.album_texture(ui.ctx(), index) {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(84.0, 84.0), egui::Sense::hover());
                            ui.painter().image(
                                texture.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(title).size(17.0).strong().color(TEXT));
                        ui.label(
                            egui::RichText::new(format!("{artist} · {album_name}"))
                                .size(13.0)
                                .color(MUTED),
                        );
                        ui.add_space(8.0);
                        let label = if app.playback.is_paused() {
                            "Resume"
                        } else {
                            "Pause"
                        };
                        if ui.button(label).clicked() {
                            app.playback.toggle_pause(&app.library.tracks);
                        }
                    });
                });
            });
    } else {
        ui.label(
            egui::RichText::new("Nothing is playing. Browse albums to start playback.")
                .size(13.0)
                .color(MUTED),
        );
        ui.add_space(8.0);
        if ui.small_button("Browse albums").clicked() {
            app.page = Page::Albums;
        }
    }
    ui.add_space(25.0);
}

fn album_section(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app: &mut MusicApp,
    title: &str,
    indices: &[usize],
) {
    section_heading(ui, title);
    egui::ScrollArea::horizontal()
        .id_salt(("home-albums", title))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                for &index in indices {
                    let Some(album) = app.library.albums.get(index) else {
                        continue;
                    };
                    let title = album.title.clone();
                    let artist = album.artist.clone();
                    let texture = app.album_texture(ctx, index);
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(150.0, 207.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, 9.0, PANEL);
                    let art_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2(7.0, 7.0),
                        egui::vec2(136.0, 136.0),
                    );
                    if let Some(texture) = texture {
                        ui.painter().image(
                            texture.id(),
                            art_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    } else {
                        ui.painter().rect_filled(
                            art_rect,
                            7.0,
                            egui::Color32::from_rgb(39, 43, 56),
                        );
                        ui.painter().text(
                            art_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            egui_phosphor::regular::DISC,
                            egui::FontId::new(32.0, egui::FontFamily::Name("phosphor".into())),
                            MUTED,
                        );
                    }
                    ui.painter().with_clip_rect(rect.shrink(7.0)).text(
                        rect.min + egui::vec2(9.0, 153.0),
                        egui::Align2::LEFT_TOP,
                        title,
                        egui::FontId::proportional(13.0),
                        TEXT,
                    );
                    ui.painter().with_clip_rect(rect.shrink(7.0)).text(
                        rect.min + egui::vec2(9.0, 174.0),
                        egui::Align2::LEFT_TOP,
                        artist,
                        egui::FontId::proportional(11.0),
                        MUTED,
                    );
                    if response.hovered() {
                        ui.painter().rect_stroke(
                            rect,
                            9.0,
                            egui::Stroke::new(1.0_f32, ACCENT),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if response.clicked() {
                        app.selected_track = None;
                        app.selected_album = Some(index);
                        app.show_album_details = true;
                        app.page = Page::Albums;
                        ctx.request_repaint();
                    }
                }
            });
        });
    ui.add_space(22.0);
}

fn section_heading(ui: &mut egui::Ui, title: &str) {
    ui.add_space(5.0);
    ui.label(egui::RichText::new(title).size(19.0).strong().color(TEXT));
    ui.add_space(11.0);
}
