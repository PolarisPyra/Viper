use crate::{
    artwork::ArtworkCache,
    library::watcher::FileWatcher,
    library::{DiscoveredTracks, Library, ScanProgress},
    playback::Playback,
    storage::settings::{AlbumSort, Settings, StartupView},
    views,
};
use eframe::egui;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[path = "scanner.rs"]
mod scanner;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Home,
    Albums,
}

pub struct MusicApp {
    pub(crate) library: Library,
    pub(crate) playback: Playback,
    pub(crate) settings: Settings,
    pub(crate) search: String,
    album_filter_query: String,
    filtered_albums: Arc<Vec<usize>>,
    album_filter_dirty: bool,
    pub(crate) page: Page,
    pub(crate) album_sort_dirty: bool,
    random_sort_seed: u64,
    last_observed_track: Option<usize>,
    pub(crate) selected_album: Option<usize>,
    pub(crate) selected_track: Option<usize>,
    pub(crate) scanning: bool,
    pub(crate) error: Option<String>,
    pub(crate) show_preferences: bool,
    pub(crate) show_album_details: bool,
    pub(crate) preferences_category: Option<crate::components::preferences::Category>,
    artwork_cache: ArtworkCache,
    scan_receiver: Option<Receiver<Library>>,
    scan_cancel: Option<Arc<AtomicBool>>,
    file_watcher: Option<FileWatcher>,
    pending_watch_paths: HashSet<PathBuf>,
    watch_debounce_until: Option<Instant>,
    watch_scan_receiver: Option<Receiver<(DiscoveredTracks, Vec<PathBuf>)>>,
    watch_scan_cancel: Option<Arc<AtomicBool>>,
    watcher_root: Option<PathBuf>,
    pending_removed_paths: HashSet<PathBuf>,
    pub(crate) scan_progress: Arc<ScanProgress>,
}

impl MusicApp {
    pub fn new() -> Self {
        let (settings, settings_error) = match Settings::load() {
            Ok(settings) => (settings, None),
            Err(error) => (
                Settings::default(),
                Some(format!("Could not read settings: {error}")),
            ),
        };
        let saved_path = settings.music_path.clone();
        let startup_page = match settings.startup_view {
            StartupView::Home => Page::Home,
            StartupView::Albums => Page::Albums,
        };
        let mut playback = Playback::default();
        playback.volume = settings.volume;
        let mut app = Self {
            library: Library::default(),
            playback,
            settings,
            search: String::new(),
            album_filter_query: String::new(),
            filtered_albums: Arc::new(Vec::new()),
            album_filter_dirty: true,
            page: startup_page,
            album_sort_dirty: true,
            random_sort_seed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos() as u64),
            last_observed_track: None,
            selected_album: None,
            selected_track: None,
            scanning: false,
            error: settings_error,
            show_preferences: false,
            show_album_details: false,
            preferences_category: None,
            artwork_cache: ArtworkCache::new(),
            scan_receiver: None,
            scan_cancel: None,
            file_watcher: None,
            pending_watch_paths: HashSet::new(),
            watch_debounce_until: None,
            watch_scan_receiver: None,
            watch_scan_cancel: None,
            watcher_root: None,
            pending_removed_paths: HashSet::new(),
            scan_progress: Arc::new(ScanProgress::default()),
        };
        if let Some(path) = saved_path {
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
            self.show_album_details = false;
            if let Some(error) = settings_error {
                self.error = Some(error);
            }
        }
    }

    pub(crate) fn filtered_album_indices(&mut self) -> Arc<Vec<usize>> {
        if self.album_filter_dirty
            || self.album_sort_dirty
            || self.album_filter_query != self.search
        {
            let query = self.search.to_lowercase();
            let mut matches: Vec<_> = self
                .library
                .albums
                .iter()
                .enumerate()
                .filter(|(_, album)| {
                    !album.tracks.is_empty()
                        && (album.title.to_lowercase().contains(&query)
                            || album.artist.to_lowercase().contains(&query))
                })
                .map(|(index, _)| index)
                .collect();
            crate::library::sorting::sort_indices(
                &mut matches,
                &self.library.albums,
                &self.library.tracks,
                &self.settings,
                self.random_sort_seed,
            );
            self.filtered_albums = Arc::new(matches);
            self.album_filter_query.clone_from(&self.search);
            self.album_filter_dirty = false;
            self.album_sort_dirty = false;
        }
        Arc::clone(&self.filtered_albums)
    }

    pub(crate) fn album_sort_changed(&mut self, sort: AlbumSort) {
        if self.settings.album_sort == sort {
            return;
        }
        self.settings.album_sort = sort;
        if sort == AlbumSort::Random {
            self.random_sort_seed = self.random_sort_seed.wrapping_add(1);
        }
        self.album_sort_dirty = true;
        self.save_settings();
    }

    pub(crate) fn toggle_sort_direction(&mut self) {
        self.settings.sort_ascending = !self.settings.sort_ascending;
        self.album_sort_dirty = true;
        self.save_settings();
    }

    fn save_settings(&mut self) {
        if let Err(error) = self.settings.save() {
            self.error = Some(format!("Could not save settings: {error}"));
        }
    }

    pub(super) fn record_new_albums(&mut self) {
        let now = unix_time_seconds();
        let mut changed = false;
        for album in &self.library.albums {
            let key = crate::library::sorting::album_sort_key(&album.artist, &album.title);
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
        let Some(album_index) = self.library.track_album.get(track_index).copied() else {
            return;
        };
        let Some(album) = self.library.albums.get(album_index) else {
            return;
        };
        let key = crate::library::sorting::album_sort_key(&album.artist, &album.title);
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

    pub(crate) fn album_texture(
        &mut self,
        ctx: &egui::Context,
        album_index: usize,
    ) -> Option<egui::TextureHandle> {
        let bytes = self.library.albums.get(album_index)?.art.as_deref()?;
        self.artwork_cache.texture(ctx, album_index, bytes)
    }
}

fn unix_time_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

impl Default for MusicApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for MusicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        ctx.style_mut(|style| {
            style.spacing.scroll.dormant_background_opacity = 0.0;
            style.spacing.scroll.active_background_opacity = 0.0;
            style.spacing.scroll.interact_background_opacity = 0.0;
        });
        ctx.style_mut(|style| style.interaction.selectable_labels = false);
        let escape_pressed = !self.show_preferences
            && self.show_album_details
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if escape_pressed {
            self.show_album_details = false;
        }
        // Always show sidebar when album details or preferences are closed
        if !self.show_album_details && !self.show_preferences && self.settings.left_panel_hidden {
            self.settings.left_panel_hidden = false;
            if let Err(error) = self.settings.save() {
                self.error = Some(format!("Could not save settings: {error}"));
            }
        }
        let space_pressed = !self.show_preferences
            && !ctx.wants_keyboard_input()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Space));
        if space_pressed {
            if self.playback.current.is_some() {
                self.playback.toggle_pause(&self.library.tracks);
            } else if let Some(album_index) = self.selected_album {
                if let Some(album) = self.library.albums.get(album_index) {
                    self.playback.play_album(album, &self.library.tracks);
                }
            }
        }
        if self.poll_scan() {
            self.artwork_cache.clear();
        }
        self.poll_file_watcher(ctx);
        self.artwork_cache.poll(ctx);
        self.artwork_cache.begin_frame();
        if self.scanning {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.playback.is_playing() {
            self.playback.advance_if_finished(&self.library.tracks);
            ctx.request_repaint_after(Duration::from_millis(400));
        }
        self.observe_playback_track();

        crate::components::top_bar::show(ctx, self);
        crate::components::sidepanel::show(ctx, self);
        crate::components::scrubber_controls::show(ctx, self);
        match self.page {
            Page::Home => views::home_view::show(ctx, self),
            Page::Albums => views::album_view::show(ctx, self),
        }
        crate::components::dialogs::show_preferences(ctx, self);
    }
}

impl Drop for MusicApp {
    fn drop(&mut self) {
        if let Some(cancel) = self.scan_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(cancel) = self.watch_scan_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}
