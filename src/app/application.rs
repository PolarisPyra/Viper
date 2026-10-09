use crate::platform::presence::discord::DiscordPresence;
use crate::{
    features::library::artwork::ArtworkCache,
    features::playback::Playback,
    features::{library::state::LibraryFeature, preferences::PreferencesState},
    platform::persistence::settings::{Settings, StartupView},
    workbench::{Page, WorkbenchState},
};
use eframe::egui;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "library_loader.rs"]
mod library_loader;
#[path = "library_watcher.rs"]
mod library_watcher;
#[path = "runtime.rs"]
mod runtime;

pub struct ViperApp {
    pub(crate) workbench: WorkbenchState,
    pub(crate) library: LibraryFeature,
    pub(crate) playback: Playback,
    pub(crate) settings: Settings,
    pub(crate) preferences: PreferencesState,
    discord_presence: DiscordPresence,
    last_observed_track: Option<usize>,
    pub(crate) error: Option<String>,
    pub(crate) artwork_cache: ArtworkCache,
    pub(crate) dismissed_notice_signature: Option<String>,
    last_window_size: Option<[f32; 2]>,
    #[cfg(target_os = "linux")]
    smb_auth: Option<crate::features::library::smb::SmbAuth>,
}

impl ViperApp {
    pub fn new() -> Self {
        Self::with_settings_result(Settings::load())
    }

    pub(crate) fn with_settings_result(settings_result: std::io::Result<Settings>) -> Self {
        let (settings, settings_error) = match settings_result {
            Ok(settings) => (settings, None),
            Err(error) => (
                Settings::default(),
                Some(format!("Could not read settings: {error}")),
            ),
        };
        let saved_path = settings.music_path.clone();
        #[cfg(target_os = "linux")]
        let saved_samba = crate::platform::persistence::settings::load_samba_settings()
            .ok()
            .flatten();
        let mut preferences =
            PreferencesState::new(settings.discord_application_id.clone().unwrap_or_default());
        #[cfg(target_os = "linux")]
        {
            preferences.smb_url_draft = saved_samba
                .as_ref()
                .map(|samba| samba.share_url.clone())
                .or_else(|| {
                    saved_path
                        .as_ref()
                        .filter(|path| crate::features::library::smb::is_smb_path(path))
                        .map(|path| path.to_string_lossy().into_owned())
                })
                .unwrap_or_default();
            preferences.smb_username = saved_samba
                .as_ref()
                .map(|samba| samba.username.clone())
                .unwrap_or_default();
            preferences.smb_password = saved_samba
                .as_ref()
                .map(|samba| samba.password.clone())
                .unwrap_or_default();
            preferences.smb_workgroup = saved_samba
                .as_ref()
                .map(|samba| samba.workgroup.clone())
                .unwrap_or_default();
        }
        let startup_page = match settings.startup_view {
            StartupView::Home => Page::Home,
            StartupView::Albums => Page::Albums,
        };
        let random_sort_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos() as u64);
        let mut playback = Playback::default();
        playback.volume = settings.volume;
        let mut app = Self {
            workbench: WorkbenchState::new(startup_page),
            library: LibraryFeature::new(random_sort_seed),
            playback,
            settings,
            preferences,
            discord_presence: DiscordPresence::default(),
            last_observed_track: None,
            error: settings_error,
            artwork_cache: ArtworkCache::new(),
            dismissed_notice_signature: None,
            last_window_size: None,
            #[cfg(target_os = "linux")]
            smb_auth: saved_samba
                .as_ref()
                .map(|samba| crate::features::library::smb::SmbAuth {
                    username: samba.username.clone(),
                    password: samba.password.clone(),
                    workgroup: samba.workgroup.clone(),
                }),
        };
        if let Some(path) = saved_path {
            #[cfg(target_os = "linux")]
            if crate::features::library::smb::is_smb_path(&path) {
                if let Some(auth) = app.smb_auth.clone() {
                    app.playback.set_smb_auth(auth);
                    app.start_scan(path);
                } else {
                    app.error = Some("Reconnect to your SMB share in Preferences".into());
                }
            } else {
                app.start_scan(path);
            }
            #[cfg(not(target_os = "linux"))]
            app.start_scan(path);
        }
        app
    }

    pub(crate) fn choose_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Choose your music folder")
            .pick_folder()
        {
            self.settings.music_path = Some(path.clone());
            let settings_error = self
                .settings
                .save()
                .err()
                .map(|error| format!("Could not save settings: {error}"));
            self.start_scan(path);
            self.artwork_cache.clear();
            self.workbench.show_album_details = false;
            if let Some(error) = settings_error {
                self.error = Some(error);
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn connect_smb_share(&mut self) {
        let url = self.preferences.smb_url_draft.trim();
        let auth = crate::features::library::smb::SmbAuth {
            username: self.preferences.smb_username.trim().to_owned(),
            password: self.preferences.smb_password.clone(),
            workgroup: self.preferences.smb_workgroup.trim().to_owned(),
        };
        if let Err(error) = crate::features::library::smb::validate_url(url) {
            self.error = Some(error);
            return;
        }
        let samba_settings = crate::platform::persistence::settings::SambaSettings {
            share_url: url.to_owned(),
            username: auth.username.clone(),
            password: auth.password.clone(),
            workgroup: auth.workgroup.clone(),
        };
        if let Err(error) =
            crate::platform::persistence::settings::save_samba_settings(&samba_settings)
        {
            self.error = Some(format!("Could not save SMB settings: {error}"));
            return;
        }
        let path = PathBuf::from(url);
        self.settings.music_path = Some(path.clone());
        if let Err(error) = self.settings.save() {
            self.error = Some(format!("Could not save SMB location: {error}"));
            return;
        }
        self.playback.set_smb_auth(auth.clone());
        self.smb_auth = Some(auth);
        self.start_scan(path);
    }

    pub(crate) fn save_settings(&mut self) {
        if let Err(error) = self.settings.save() {
            self.error = Some(format!("Could not save settings: {error}"));
        }
    }

    fn persist_window_size(&mut self, ctx: &egui::Context) {
        let size = ctx.input(|input| input.screen_rect.size());
        if !size.x.is_finite() || !size.y.is_finite() || size.x < 760.0 || size.y < 520.0 {
            return;
        }
        let size = [size.x.round(), size.y.round()];
        if self.last_window_size == Some(size) && self.settings.window_size == Some(size) {
            return;
        }

        self.last_window_size = Some(size);
        self.settings.window_size = Some(size);
        self.save_settings();
    }

    pub(super) fn record_new_albums(&mut self) {
        let now = unix_time_seconds();
        let mut changed = false;
        for album in &self.library.model.albums {
            let key =
                crate::features::library::sorting::album_sort_key(&album.artist, &album.title);
            if let std::collections::btree_map::Entry::Vacant(entry) =
                self.settings.album_added.entry(key)
            {
                entry.insert(now);
                changed = true;
            }
        }
        if changed {
            self.save_settings();
        }
    }

    fn observe_playback_track(&mut self) {
        if self.last_observed_track == self.playback.current {
            return;
        }
        self.last_observed_track = self.playback.current;
        let Some(track_index) = self.playback.current else {
            return;
        };
        let Some(album_index) = self.library.model.track_album.get(track_index).copied() else {
            return;
        };
        let Some(album) = self.library.model.albums.get(album_index) else {
            return;
        };
        let key = crate::features::library::sorting::album_sort_key(&album.artist, &album.title);
        *self
            .settings
            .album_play_counts
            .entry(key.clone())
            .or_default() += 1;
        self.settings
            .album_last_played
            .insert(key, unix_time_seconds());
        self.save_settings();
    }
}

fn unix_time_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
