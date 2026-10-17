use crate::platform::presence::discord::DiscordPresence;
use crate::platform::presence::lastfm::LastFmService;
use crate::{
    features::playback::Playback,
    features::{
        library::state::LibraryFeature, lyrics::LyricsFeature, preferences::PreferencesState,
    },
    platform::persistence::settings::{LastFmSettings, Settings, StartupView},
    shared::artwork::ArtworkCache,
    workbench::{Page, WorkbenchState},
};
use anyhow::Context;
use eframe::egui;
use secrecy::ExposeSecret;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "library_loader.rs"]
mod library_loader;
#[path = "library_watcher.rs"]
mod library_watcher;
#[path = "runtime.rs"]
mod runtime;

/// Main application state and egui runtime integration.
pub struct ViperApp {
    pub(crate) workbench: WorkbenchState,
    pub(crate) library: LibraryFeature,
    pub(crate) playback: Arc<Mutex<Playback>>,
    pub(crate) lyrics: LyricsFeature,
    pub(crate) settings: Settings,
    pub(crate) preferences: PreferencesState,
    discord_presence: DiscordPresence,
    pub(crate) lastfm_service: LastFmService,
    pub(crate) lastfm_settings: LastFmSettings,
    last_observed_track: Option<usize>,
    pub(crate) error: Option<String>,
    pub(crate) artwork_cache: ArtworkCache,
    last_window_size: Option<[f32; 2]>,
    #[cfg(target_os = "linux")]
    smb_auth: Option<crate::features::library::smb::SmbAuth>,
    #[cfg(target_os = "linux")]
    pub(crate) mpris: crate::platform::mpris::MprisBridge,
}

impl ViperApp {
    /// Create the application from persisted settings, falling back to defaults on load errors.
    ///
    /// # Returns
    /// A ready-to-run application instance.
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
        let (lastfm_settings, lastfm_error) =
            match crate::platform::persistence::settings::load_lastfm_settings() {
                Ok(settings) => (settings, None),
                Err(error) => (
                    LastFmSettings::default(),
                    Some(format!("Could not read Last.fm settings: {error}")),
                ),
            };
        let saved_path = settings.music_path.clone();
        #[cfg(target_os = "linux")]
        let saved_samba = crate::platform::persistence::settings::load_samba_settings()
            .ok()
            .flatten();
        let mut preferences = PreferencesState::new(
            settings.discord_application_id.clone().unwrap_or_default(),
            lastfm_settings.api_key.expose_secret().to_owned(),
            lastfm_settings.api_secret.expose_secret().to_owned(),
        );
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
                .map(|samba| samba.password.expose_secret().to_owned())
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
        let playback = Arc::new(Mutex::new(playback));
        #[cfg(target_os = "linux")]
        let mpris = crate::platform::mpris::MprisBridge::new(Arc::clone(&playback));
        let mut app = Self {
            workbench: WorkbenchState::new(startup_page),
            library: LibraryFeature::new(random_sort_seed),
            playback,
            lyrics: LyricsFeature::default(),
            settings,
            preferences,
            discord_presence: DiscordPresence::default(),
            lastfm_service: LastFmService::default(),
            lastfm_settings,
            last_observed_track: None,
            error: settings_error.or(lastfm_error),
            artwork_cache: ArtworkCache::new(),
            last_window_size: None,
            #[cfg(target_os = "linux")]
            smb_auth: saved_samba
                .as_ref()
                .map(|samba| crate::features::library::smb::SmbAuth {
                    username: samba.username.clone(),
                    password: samba.password.clone(),
                    workgroup: samba.workgroup.clone(),
                }),
            #[cfg(target_os = "linux")]
            mpris,
        };
        if let Some(path) = saved_path {
            #[cfg(target_os = "linux")]
            if crate::features::library::smb::is_smb_path(&path) {
                if let Some(auth) = app.smb_auth.clone() {
                    app.playback
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .set_smb_auth(auth);
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
        let url = self.preferences.smb_url_draft.trim().to_owned();
        let result = (|| -> anyhow::Result<()> {
            let auth = crate::features::library::smb::SmbAuth {
                username: self.preferences.smb_username.trim().to_owned(),
                password: secrecy::SecretString::from(self.preferences.smb_password.clone()),
                workgroup: self.preferences.smb_workgroup.trim().to_owned(),
            };
            crate::features::library::smb::validate_url(&url)
                .context("Could not connect to SMB share")?;
            let samba_settings = crate::platform::persistence::settings::SambaSettings {
                share_url: url.clone(),
                username: auth.username.clone(),
                password: auth.password.clone(),
                workgroup: auth.workgroup.clone(),
            };
            crate::platform::persistence::settings::save_samba_settings(&samba_settings)
                .context("Could not save SMB settings")?;
            let path = PathBuf::from(url);
            self.settings.music_path = Some(path.clone());
            self.settings
                .save()
                .context("Could not save SMB location")?;
            self.playback
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_smb_auth(auth.clone());
            self.smb_auth = Some(auth);
            self.start_scan(path);
            Ok(())
        })();
        if let Err(error) = result {
            self.error = Some(format!("{error:#}"));
        }
    }

    pub(crate) fn save_settings(&mut self) {
        if let Err(error) = self.settings.save() {
            self.error = Some(format!("Could not save settings: {error}"));
        }
    }

    pub(crate) fn save_lastfm_settings_from_preferences(&mut self) {
        let api_key = self.preferences.lastfm_api_key_draft.trim().to_owned();
        let api_secret = self.preferences.lastfm_api_secret_draft.trim().to_owned();
        if api_key != self.lastfm_settings.api_key.expose_secret()
            || api_secret != self.lastfm_settings.api_secret.expose_secret()
        {
            self.lastfm_settings.session_key = None;
            self.lastfm_settings.username = None;
            self.lastfm_service.disconnect();
        }
        self.lastfm_settings.api_key = secrecy::SecretString::from(api_key);
        self.lastfm_settings.api_secret = secrecy::SecretString::from(api_secret);
        if let Err(error) =
            crate::platform::persistence::settings::save_lastfm_settings(&self.lastfm_settings)
        {
            self.error = Some(format!("Could not save Last.fm settings: {error}"));
        }
    }

    pub(crate) fn disconnect_lastfm(&mut self) {
        self.lastfm_settings.session_key = None;
        self.lastfm_settings.username = None;
        self.lastfm_service.disconnect();
        if let Err(error) =
            crate::platform::persistence::settings::save_lastfm_settings(&self.lastfm_settings)
        {
            self.error = Some(format!("Could not save Last.fm settings: {error}"));
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

    fn observe_playback_track(&mut self, playback: &Playback) {
        if self.last_observed_track == playback.current {
            return;
        }
        self.last_observed_track = playback.current;
        let Some(track_index) = playback.current else {
            return;
        };
        let Some(album_index) = self
            .library
            .model
            .track_album
            .get(track_index)
            .copied()
            .flatten()
        else {
            return;
        };
        let Some(album) = self.library.model.albums.get(album_index.get()) else {
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
