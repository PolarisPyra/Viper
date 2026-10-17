use super::ViperApp;
use eframe::egui;
use std::time::Duration;

impl Default for ViperApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for ViperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let playback_handle = std::sync::Arc::clone(&self.playback);
        let mut playback = playback_handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.persist_window_size(ctx);
        crate::shared::ui::theme::apply(ctx, self.settings.theme);
        ctx.style_mut(|style| {
            style.spacing.scroll.dormant_background_opacity = 0.0;
            style.spacing.scroll.active_background_opacity = 0.0;
            style.spacing.scroll.interact_background_opacity = 0.0;
        });
        ctx.style_mut(|style| style.interaction.selectable_labels = false);
        let popup_open = ctx.memory(|memory| memory.any_popup_open());
        let escape_pressed = !self.workbench.show_preferences
            && (popup_open
                || self.workbench.show_queue
                || self.workbench.show_shortcuts
                || self.workbench.show_album_details)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if escape_pressed {
            if popup_open {
                ctx.memory_mut(|memory| memory.close_popup());
            } else if self.workbench.show_shortcuts {
                self.workbench.show_shortcuts = false;
            } else if self.workbench.show_queue {
                self.workbench.show_queue = false;
            } else {
                self.workbench.show_album_details = false;
            }
        }
        let search_pressed = !self.workbench.show_preferences
            && !self.workbench.show_shortcuts
            && !ctx.wants_keyboard_input()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F));
        if search_pressed {
            self.workbench.page = crate::workbench::Page::Albums;
            self.workbench.show_album_details = false;
            ctx.memory_mut(|memory| {
                memory.request_focus(egui::Id::new("library-search-field"));
            });
        }
        let queue_pressed = !self.workbench.show_preferences
            && !self.workbench.show_shortcuts
            && !ctx.wants_keyboard_input()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Q));
        if queue_pressed {
            self.workbench.show_queue = !self.workbench.show_queue;
        }
        let shuffle_pressed = !self.workbench.show_preferences
            && !self.workbench.show_shortcuts
            && !ctx.wants_keyboard_input()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F5));
        if shuffle_pressed {
            playback.play_random_track(&self.library.model.tracks);
        }
        if !self.workbench.show_album_details
            && !self.workbench.show_preferences
            && self.settings.left_panel_hidden
        {
            self.settings.left_panel_hidden = false;
            if let Err(error) = self.settings.save() {
                self.error = Some(format!("Could not save settings: {error}"));
            }
        }
        let space_pressed = !self.workbench.show_preferences
            && !ctx.wants_keyboard_input()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Space));
        if space_pressed {
            if playback.current.is_some() {
                playback.toggle_pause();
            } else if let Some(album_index) = self.library.view.selected_album {
                if let Some(album) = self.library.model.albums.get(album_index) {
                    if let Some(track_index) = self
                        .library
                        .view
                        .selected_track
                        .filter(|index| album.tracks.contains(index))
                    {
                        playback.play_track_in_album(
                            album,
                            &self.library.model.tracks,
                            track_index,
                        );
                    } else {
                        playback.play_album(album, &self.library.model.tracks);
                    }
                }
            } else if playback.shuffle_enabled {
                playback.play_random_track(&self.library.model.tracks);
            }
        }
        drop(playback);
        if self.poll_scan() {
            self.artwork_cache.clear();
        }
        #[cfg(target_os = "linux")]
        let mut artwork_updated = false;
        for (album_index, artwork) in self.library.background.poll_artwork(ctx) {
            if let Some(album) = self.library.model.albums.get_mut(album_index) {
                album.art = Some(artwork);
                #[cfg(target_os = "linux")]
                {
                    artwork_updated = true;
                }
            }
        }
        #[cfg(target_os = "linux")]
        if artwork_updated {
            self.mpris.set_library(
                std::sync::Arc::clone(&self.library.model.tracks),
                std::sync::Arc::clone(&self.library.model.track_album),
                &self.library.model.albums,
            );
        }
        self.poll_file_watcher(ctx);
        self.artwork_cache.poll(ctx);
        self.artwork_cache.begin_frame();
        let mut playback = playback_handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.library.scanning {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if playback.has_playback_work() {
            if playback.current.is_some() && !playback.is_paused() {
                playback.advance_if_finished(&self.library.model.tracks);
            }
            let repeat_one = playback.repeat_mode == crate::features::playback::RepeatMode::One;
            playback.update_gapless(
                &self.library.model.tracks,
                self.settings.gapless_playback && !repeat_one,
            );
            if playback.has_playback_work() {
                let repaint_interval = if playback.gapless_preparation_pending() {
                    Duration::from_millis(16)
                } else if self.settings.gapless_playback && playback.has_next() && !repeat_one {
                    Duration::from_millis(100)
                } else {
                    Duration::from_millis(400)
                };
                ctx.request_repaint_after(repaint_interval);
            }
        }
        self.observe_playback_track(&playback);
        if self.lastfm_service.poll(&mut self.lastfm_settings) {
            if let Err(error) =
                crate::platform::persistence::settings::save_lastfm_settings(&self.lastfm_settings)
            {
                self.error = Some(format!("Could not save Last.fm session: {error}"));
            }
        }
        let current_track_index = playback.current;
        let current_track =
            current_track_index.and_then(|index| self.library.model.tracks.get(index));
        self.lastfm_service.update_playback(
            self.settings.lastfm_scrobbling_enabled,
            &self.lastfm_settings,
            current_track_index,
            current_track,
            playback.position(),
        );

        let current_track = playback.current.and_then(|index| {
            let track = self.library.model.tracks.get(index)?;
            Some((
                index,
                track.title.as_str(),
                track.artist.as_str(),
                track.album.as_str(),
                track.duration_ms.map(Duration::from_millis),
            ))
        });
        self.discord_presence.update(
            if self.settings.discord_rich_presence_enabled {
                self.settings.discord_application_id.as_deref()
            } else {
                None
            },
            current_track,
            playback.is_paused(),
            playback.position(),
        );

        let actions = {
            let mut context = crate::workbench::WorkbenchContext {
                state: &mut self.workbench,
                library: &mut self.library,
                playback: &mut playback,
                lyrics: &mut self.lyrics,
                settings: &mut self.settings,
                preferences: &mut self.preferences,
                lastfm_service: &mut self.lastfm_service,
                lastfm_settings: &mut self.lastfm_settings,
                artwork_cache: &mut self.artwork_cache,
                error: &mut self.error,
            };
            crate::workbench::show(ctx, &mut context)
        };
        drop(playback);
        self.artwork_cache.finish_frame(ctx);
        for action in actions {
            match action {
                crate::workbench::WorkbenchAction::ChooseMusicFolder => self.choose_folder(),
                crate::workbench::WorkbenchAction::SaveSettings => self.save_settings(),
                crate::workbench::WorkbenchAction::ConnectSmbShare => {
                    #[cfg(target_os = "linux")]
                    self.connect_smb_share();
                }
                crate::workbench::WorkbenchAction::SaveLastFmSettings => {
                    self.save_lastfm_settings_from_preferences();
                }
                crate::workbench::WorkbenchAction::StartLastFmAuth => {
                    self.save_lastfm_settings_from_preferences();
                    self.lastfm_service.start_auth(&self.lastfm_settings);
                }
                crate::workbench::WorkbenchAction::FinishLastFmAuth => {
                    self.lastfm_service.finish_auth(&self.lastfm_settings);
                }
                crate::workbench::WorkbenchAction::DisconnectLastFm => {
                    self.disconnect_lastfm();
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let Some(size) = self.last_window_size else {
            return;
        };
        if self.settings.window_size == Some(size) {
            return;
        }

        self.settings.window_size = Some(size);
        if let Err(error) = self.settings.save() {
            eprintln!("Could not save window size: {error}");
        }
    }
}

impl Drop for ViperApp {
    fn drop(&mut self) {
        self.library.background.cancel_all();
    }
}
