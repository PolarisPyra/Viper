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
        self.persist_window_size(ctx);
        crate::shared::ui::theme::apply(ctx, self.settings.theme);
        ctx.style_mut(|style| {
            style.spacing.scroll.dormant_background_opacity = 0.0;
            style.spacing.scroll.active_background_opacity = 0.0;
            style.spacing.scroll.interact_background_opacity = 0.0;
        });
        ctx.style_mut(|style| style.interaction.selectable_labels = false);
        let escape_pressed = !self.workbench.show_preferences
            && self.workbench.show_album_details
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if escape_pressed {
            self.workbench.show_album_details = false;
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
            if self.playback.current.is_some() {
                self.playback.toggle_pause();
            } else if let Some(album_index) = self.library.view.selected_album {
                if let Some(album) = self.library.model.albums.get(album_index) {
                    self.playback.play_album(album, &self.library.model.tracks);
                }
            }
        }
        if self.poll_scan() {
            self.artwork_cache.clear();
        }
        self.poll_file_watcher(ctx);
        self.artwork_cache.poll(ctx);
        self.artwork_cache.begin_frame();
        if self.library.scanning {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.playback.current.is_some() && !self.playback.is_paused() {
            self.playback
                .advance_if_finished(&self.library.model.tracks);
            if self.playback.current.is_some() {
                ctx.request_repaint_after(Duration::from_millis(400));
            }
        }
        self.observe_playback_track();

        let current_track = self.playback.current.and_then(|index| {
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
            self.settings.discord_application_id.as_deref(),
            current_track,
            self.playback.is_paused(),
            self.playback.position(),
        );

        let actions = {
            let mut context = crate::workbench::WorkbenchContext {
                state: &mut self.workbench,
                library: &mut self.library,
                playback: &mut self.playback,
                settings: &mut self.settings,
                preferences: &mut self.preferences,
                artwork_cache: &mut self.artwork_cache,
                error: &mut self.error,
                dismissed_notice_signature: &mut self.dismissed_notice_signature,
            };
            crate::workbench::show(ctx, &mut context)
        };
        for action in actions {
            match action {
                crate::workbench::WorkbenchAction::ChooseMusicFolder => self.choose_folder(),
                crate::workbench::WorkbenchAction::SaveSettings => self.save_settings(),
                crate::workbench::WorkbenchAction::ConnectSmbShare => {
                    #[cfg(target_os = "linux")]
                    self.connect_smb_share();
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
