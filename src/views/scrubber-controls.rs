use crate::{
    app::MusicApp,
    views::icons::{self, Icon},
};
use eframe::egui;
use std::time::Duration;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(155, 125, 255);

pub fn show(ctx: &egui::Context, app: &mut MusicApp) {
    egui::TopBottomPanel::bottom("scrubber-controls")
        .exact_height(86.0)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(19, 22, 30))
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    egui::Color32::from_rgb(39, 43, 54),
                ))
                .inner_margin(egui::Margin::symmetric(24, 8)),
        )
        .show(ctx, |ui| {
            let current = app.playback.current;
            let duration = current
                .and_then(|index| app.library.tracks.get(index))
                .and_then(|track| track.duration_ms)
                .map(Duration::from_millis);

            let row_width = ui.available_width();
            let (row_rect, _) =
                ui.allocate_exact_size(egui::vec2(row_width, 48.0), egui::Sense::hover());
            let info_rect = egui::Rect::from_min_size(
                row_rect.min,
                egui::vec2((row_width * 0.32).min(280.0), row_rect.height()),
            );
            let mut info_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(info_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            show_track_info(ctx, &mut info_ui, app, current);

            // Anchor the controls to the full bar rect; column sizing otherwise
            // shifts them when the track-info column has no content.
            let controls_rect = egui::Rect::from_center_size(
                row_rect.center(),
                egui::vec2(126.0, row_rect.height()),
            );
            let previous_rect = egui::Rect::from_min_size(
                controls_rect.min + egui::vec2(0.0, 8.0),
                egui::vec2(34.0, 32.0),
            );
            let play_rect = egui::Rect::from_min_size(
                controls_rect.min + egui::vec2(44.0, 5.0),
                egui::vec2(38.0, 38.0),
            );
            let next_rect = egui::Rect::from_min_size(
                controls_rect.min + egui::vec2(92.0, 8.0),
                egui::vec2(34.0, 32.0),
            );
            let previous = ui.interact(
                previous_rect,
                egui::Id::new("player-previous"),
                egui::Sense::click(),
            );
            let play = ui.interact(
                play_rect,
                egui::Id::new("player-play-pause"),
                egui::Sense::click(),
            );
            let next = ui.interact(
                next_rect,
                egui::Id::new("player-next"),
                egui::Sense::click(),
            );
            icons::draw(
                ui.painter(),
                previous_rect,
                Icon::Previous,
                if previous.hovered() {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_gray(190)
                },
            );
            ui.painter().circle_filled(play_rect.center(), 17.0, ACCENT);
            let icon = if app.playback.is_playing() {
                Icon::Pause
            } else {
                Icon::Play
            };
            icons::draw(ui.painter(), play_rect, icon, egui::Color32::WHITE);
            icons::draw(
                ui.painter(),
                next_rect,
                Icon::Next,
                if next.hovered() {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_gray(190)
                },
            );
            let previous = previous.on_hover_text("Previous track");
            let play = play.on_hover_text(if app.playback.is_playing() {
                "Pause"
            } else {
                "Play"
            });
            let next = next.on_hover_text("Next track");
            if previous.clicked() {
                app.playback.previous(&app.library.tracks);
            }
            if play.clicked() {
                if current.is_some() {
                    app.playback.toggle_pause(&app.library.tracks);
                } else if let Some(album_index) = app.selected_album {
                    if let Some(album) = app.library.albums.get(album_index) {
                        app.playback.play_album(album, &app.library.tracks);
                    }
                }
            }
            if next.clicked() {
                app.playback.skip_next(&app.library.tracks);
            }

            let volume_area = egui::Rect::from_min_size(
                row_rect.right_top() + egui::vec2(-176.0, 9.0),
                egui::vec2(176.0, 30.0),
            );
            let volume_icon = egui::Rect::from_min_size(
                volume_area.min + egui::vec2(0.0, 4.0),
                egui::vec2(22.0, 22.0),
            );
            icons::draw(
                ui.painter(),
                volume_icon,
                Icon::Volume,
                egui::Color32::from_gray(175),
            );
            let volume_track_area = egui::Rect::from_min_max(
                volume_area.min + egui::vec2(30.0, 0.0),
                volume_area.max - egui::vec2(46.0, 0.0),
            );
            let volume_response = ui.interact(
                volume_track_area,
                egui::Id::new("player-volume-slider"),
                egui::Sense::click_and_drag(),
            );
            let volume_track = egui::Rect::from_center_size(
                volume_track_area.center(),
                egui::vec2(volume_track_area.width(), 3.0),
            );
            ui.painter()
                .rect_filled(volume_track, 2.0, egui::Color32::from_rgb(54, 58, 69));
            let volume_fraction = app.settings.volume as f32 / 100.0;
            let volume_x = volume_track.left() + volume_track.width() * volume_fraction;
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    volume_track.min,
                    egui::pos2(volume_x, volume_track.bottom()),
                ),
                2.0,
                ACCENT,
            );
            ui.painter().circle_filled(
                egui::pos2(volume_x, volume_track.center().y),
                if volume_response.hovered() || volume_response.dragged() {
                    5.0
                } else {
                    3.5
                },
                egui::Color32::WHITE,
            );
            if volume_response.clicked() || volume_response.dragged() {
                if let Some(pointer) = volume_response.interact_pointer_pos() {
                    let fraction =
                        ((pointer.x - volume_track.left()) / volume_track.width()).clamp(0.0, 1.0);
                    app.settings.volume = (fraction * 100.0).round() as u8;
                }
            }
            if volume_response.dragged() {
                app.playback
                    .preview_volume(&app.library.tracks, app.settings.volume);
            }
            if volume_response.clicked() || volume_response.drag_stopped() {
                app.playback
                    .set_volume(&app.library.tracks, app.settings.volume);
                if let Err(error) = app.settings.save() {
                    app.error = Some(format!("Could not save settings: {error}"));
                }
            }

            let position = app.playback.position();
            let mut seconds = position.as_secs_f64();
            let max_seconds = duration.map_or(0.0, |duration| duration.as_secs_f64());
            let mut percentage_text_left = row_rect.left();
            ui.horizontal(|ui| {
                let (position_label, _) = ui.allocate_exact_size(
                    egui::vec2(38.0, 14.0),
                    egui::Sense::hover(),
                );
                ui.painter().text(
                    egui::pos2(position_label.left(), position_label.center().y),
                    egui::Align2::LEFT_CENTER,
                    format_time(position),
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_gray(135),
                );

                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2((ui.available_width() - 46.0).max(1.0), 14.0),
                    egui::Sense::click_and_drag(),
                );
                let track_color = egui::Color32::from_rgb(54, 58, 69);
                let track_rect =
                    egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), 3.0));
                ui.painter().rect_filled(track_rect, 2.0, track_color);

                let fraction = if max_seconds > 0.0 {
                    (seconds / max_seconds).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                let progress_rect = egui::Rect::from_min_max(
                    track_rect.min,
                    egui::pos2(
                        track_rect.left() + track_rect.width() * fraction,
                        track_rect.bottom(),
                    ),
                );
                ui.painter().rect_filled(progress_rect, 2.0, ACCENT);
                let knob_x = track_rect.left() + track_rect.width() * fraction;
                if current.is_some() && duration.is_some() {
                    ui.painter().circle_filled(
                        egui::pos2(knob_x, rect.center().y),
                        if response.hovered() || response.dragged() {
                            5.0
                        } else {
                            3.5
                        },
                        egui::Color32::WHITE,
                    );
                }

                if (response.clicked() || response.dragged()) && max_seconds > 0.0 {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let fraction =
                            ((pointer.x - track_rect.left()) / track_rect.width()).clamp(0.0, 1.0);
                        seconds = max_seconds * fraction as f64;
                        app.playback
                            .seek(&app.library.tracks, Duration::from_secs_f64(seconds));
                    }
                }

                let (duration_label, _) = ui.allocate_exact_size(
                    egui::vec2(38.0, 14.0),
                    egui::Sense::hover(),
                );
                percentage_text_left = duration_label.left();
                ui.painter().text(
                    egui::pos2(duration_label.left(), duration_label.center().y),
                    egui::Align2::LEFT_CENTER,
                    duration.map(format_time).unwrap_or_default(),
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_gray(135),
                );
            });
            ui.painter().text(
                egui::pos2(percentage_text_left, volume_area.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{}%", app.settings.volume),
                egui::FontId::proportional(11.0),
                egui::Color32::from_gray(145),
            );
            if app.playback.is_playing() {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        });
}

fn show_track_info(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    app: &mut MusicApp,
    current: Option<usize>,
) {
    if let Some(index) = current {
        let album_index = app.library.track_album.get(index).copied();
        let texture = album_index.and_then(|index| app.album_texture(ctx, index));
        let Some(track) = app.library.tracks.get(index) else {
            return;
        };
        let cover = ui.allocate_space(egui::vec2(48.0, 48.0)).1;
        if let Some(texture) = texture {
            ui.painter().image(
                texture.id(),
                cover,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        ui.add_space(9.0);
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&track.title)
                        .size(15.0)
                        .strong()
                        .color(egui::Color32::from_rgb(235, 237, 244)),
                )
                .truncate(),
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&track.artist)
                        .size(13.0)
                        .color(egui::Color32::from_gray(145)),
                )
                .truncate(),
            );
        });
    }
}

fn format_time(time: Duration) -> String {
    let total = time.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}
