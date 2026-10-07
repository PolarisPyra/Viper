use crate::{
    metadata::{
        merge_discovered_tracks, remove_library_tracks, scan_added_tracks, scan_library,
        DiscoveredTracks, Library, ScanProgress,
    },
    playback::Playback,
    storage::settings::{Settings, StartupView},
    views,
    watcher::{FileChange, FileWatcher},
};
use eframe::egui;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
    time::{Duration, Instant},
};

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
    pub(crate) selected_album: Option<usize>,
    pub(crate) selected_track: Option<usize>,
    pub(crate) scanning: bool,
    pub(crate) error: Option<String>,
    pub(crate) show_folder_dialog: bool,
    pub(crate) show_settings: bool,
    pub(crate) show_album_details: bool,
    textures: HashMap<usize, egui::TextureHandle>,
    texture_lru: VecDeque<usize>,
    visible_texture_indices: HashSet<usize>,
    texture_requests: HashSet<(usize, usize)>,
    texture_epoch: usize,
    texture_workers: Arc<AtomicUsize>,
    texture_receiver: Receiver<(usize, usize, Option<egui::ColorImage>)>,
    texture_sender: mpsc::Sender<(usize, usize, Option<egui::ColorImage>)>,
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
        let (texture_sender, texture_receiver) = mpsc::channel();
        let mut app = Self {
            library: Library::default(),
            playback,
            settings,
            search: String::new(),
            album_filter_query: String::new(),
            filtered_albums: Arc::new(Vec::new()),
            album_filter_dirty: true,
            page: startup_page,
            selected_album: None,
            selected_track: None,
            scanning: false,
            error: settings_error,
            show_folder_dialog: saved_path.is_none(),
            show_settings: false,
            show_album_details: false,
            textures: HashMap::new(),
            texture_lru: VecDeque::new(),
            visible_texture_indices: HashSet::new(),
            texture_requests: HashSet::new(),
            texture_epoch: 0,
            texture_workers: Arc::new(AtomicUsize::new(0)),
            texture_receiver,
            texture_sender,
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
            self.textures.clear();
            self.texture_lru.clear();
            self.show_album_details = false;
            if let Some(error) = settings_error {
                self.error = Some(error);
            }
        }
    }

    fn start_scan(&mut self, root: PathBuf) {
        if let Some(cancel) = self.scan_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(cancel) = self.watch_scan_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.watch_scan_receiver = None;
        self.watcher_root = None;
        self.pending_watch_paths.clear();
        self.pending_removed_paths.clear();
        self.watch_debounce_until = None;
        self.file_watcher = None;
        self.playback.stop();
        self.library = Library::default();
        self.texture_epoch = self.texture_epoch.wrapping_add(1);
        self.texture_requests.clear();
        self.album_filter_dirty = true;
        self.selected_album = None;
        self.error = None;
        self.scanning = true;

        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let progress = Arc::new(ScanProgress::default());
        let worker_progress = Arc::clone(&progress);
        self.scan_progress = progress;
        self.scan_receiver = Some(receiver);
        self.scan_cancel = Some(cancel);

        if let Err(error) = std::thread::Builder::new()
            .name("music-library-scan".into())
            .spawn(move || {
                let library = scan_library(&root, &worker_cancel, &worker_progress);
                let _ = sender.send(library);
            })
        {
            self.scan_receiver = None;
            self.scan_cancel = None;
            self.scanning = false;
            self.error = Some(format!("Could not start library scan: {error}"));
        }
    }

    fn poll_file_watcher(&mut self, ctx: &egui::Context) {
        if let Some(root) = self.settings.music_path.as_ref() {
            if self.watcher_root.as_ref() != Some(root) {
                self.file_watcher = None;
                match FileWatcher::new(root, ctx.clone()) {
                    Ok(watcher) => {
                        self.file_watcher = Some(watcher);
                        self.watcher_root = Some(root.clone());
                    }
                    Err(error) => {
                        self.watcher_root = Some(root.clone());
                        self.error = Some(format!("Could not watch music folder: {error}"));
                    }
                }
            }
        }
        if let Some(watcher) = &self.file_watcher {
            match watcher.poll() {
                Ok(paths) if !paths.is_empty() => {
                    for change in paths {
                        match change {
                            FileChange::Upsert(path) => {
                                self.pending_watch_paths.insert(path);
                            }
                            FileChange::Remove(path) => {
                                self.pending_removed_paths.insert(path);
                            }
                        }
                    }
                    self.watch_debounce_until = Some(Instant::now() + Duration::from_millis(800));
                }
                Ok(_) => {}
                Err(error) => {
                    self.error = Some(format!("Music folder watcher error: {error}"));
                    self.file_watcher = None;
                }
            }
        }

        let mut finished = false;
        if let Some(receiver) = &self.watch_scan_receiver {
            match receiver.try_recv() {
                Ok((discovered, removed_paths)) => {
                    finished = true;
                    let remap = remove_library_tracks(&mut self.library, &removed_paths);
                    self.playback.remap_tracks(&remap);
                    if let Some(selected) = self.selected_track {
                        self.selected_track = remap.get(selected).copied().flatten();
                    }
                    if self.selected_album.is_some_and(|index| {
                        self.library
                            .albums
                            .get(index)
                            .map_or(true, |album| album.tracks.is_empty())
                    }) {
                        self.selected_album = None;
                        self.show_album_details = false;
                    }
                    let removed_any = remap.iter().any(Option::is_none);
                    let added = merge_discovered_tracks(&mut self.library, discovered);
                    if added > 0 || removed_any {
                        self.album_filter_dirty = true;
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    finished = true;
                    self.error = Some("The music folder update scan stopped unexpectedly".into());
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if finished {
            self.watch_scan_receiver = None;
            self.watch_scan_cancel = None;
        }

        if !self.scanning
            && self.watch_scan_receiver.is_none()
            && (!self.pending_watch_paths.is_empty() || !self.pending_removed_paths.is_empty())
            && self
                .watch_debounce_until
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            let paths: Vec<_> = self.pending_watch_paths.drain().collect();
            let removed_paths: Vec<_> = self.pending_removed_paths.drain().collect();
            self.watch_debounce_until = None;
            let cancel = Arc::new(AtomicBool::new(false));
            let worker_cancel = Arc::clone(&cancel);
            let (sender, receiver) = mpsc::channel();
            match std::thread::Builder::new()
                .name("music-library-watch-scan".into())
                .spawn(move || {
                    let discovered = scan_added_tracks(&paths, &worker_cancel);
                    let _ = sender.send((discovered, removed_paths));
                }) {
                Ok(_) => {
                    self.watch_scan_cancel = Some(cancel);
                    self.watch_scan_receiver = Some(receiver);
                }
                Err(error) => self.error = Some(format!("Could not update music library: {error}")),
            }
        }

        if !self.pending_watch_paths.is_empty()
            || !self.pending_removed_paths.is_empty()
            || self.watch_scan_receiver.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
    }

    fn poll_scan(&mut self) -> bool {
        let Some(receiver) = self.scan_receiver.as_ref() else {
            return false;
        };
        match receiver.try_recv() {
            Ok(library) => {
                if library.unreadable_directories > 0 {
                    self.error = Some(format!(
                        "Skipped {} unreadable director{} while scanning",
                        library.unreadable_directories,
                        if library.unreadable_directories == 1 {
                            "y"
                        } else {
                            "ies"
                        }
                    ));
                } else if library.tracks.is_empty() {
                    self.error = Some("No supported audio files were found in this folder".into());
                }
                self.library = library;
                self.album_filter_dirty = true;
                self.scan_receiver = None;
                self.scan_cancel = None;
                self.scanning = false;
                true
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.scan_receiver = None;
                self.scan_cancel = None;
                self.scanning = false;
                self.error = Some("The library scan stopped unexpectedly".into());
                true
            }
        }
    }

    pub(crate) fn filtered_album_indices(&mut self) -> Arc<Vec<usize>> {
        if self.album_filter_dirty || self.album_filter_query != self.search {
            let query = self.search.to_lowercase();
            let matches = self
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
            self.filtered_albums = Arc::new(matches);
            self.album_filter_query.clone_from(&self.search);
            self.album_filter_dirty = false;
        }
        Arc::clone(&self.filtered_albums)
    }

    pub(crate) fn album_texture(
        &mut self,
        ctx: &egui::Context,
        album_index: usize,
    ) -> Option<egui::TextureHandle> {
        let bytes = self.library.albums.get(album_index)?.art.as_deref()?;
        self.visible_texture_indices.insert(album_index);
        if let Some(texture) = self.textures.get(&album_index).cloned() {
            self.texture_lru.retain(|&index| index != album_index);
            self.texture_lru.push_back(album_index);
            return Some(texture);
        }
        let request = (self.texture_epoch, album_index);
        if !self.texture_requests.contains(&request) {
            let mut active = self.texture_workers.load(Ordering::Relaxed);
            while active < 4 {
                match self.texture_workers.compare_exchange_weak(
                    active,
                    active + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(observed) => active = observed,
                }
            }
            if active < 4 {
                let bytes = bytes.to_vec();
                let sender = self.texture_sender.clone();
                let repaint = ctx.clone();
                let workers = Arc::clone(&self.texture_workers);
                let epoch = self.texture_epoch;
                if std::thread::Builder::new()
                    .name("album-art-decode".into())
                    .spawn(move || {
                        let decoded = image::load_from_memory(&bytes)
                            .ok()
                            .map(|image| image.into_rgba8())
                            .map(|image| {
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [image.width() as usize, image.height() as usize],
                                    image.as_raw(),
                                )
                            });
                        let _ = sender.send((epoch, album_index, decoded));
                        workers.fetch_sub(1, Ordering::Relaxed);
                        repaint.request_repaint();
                    })
                    .is_ok()
                {
                    self.texture_requests.insert(request);
                } else {
                    self.texture_workers.fetch_sub(1, Ordering::Relaxed);
                }
            }
        }
        None
    }

    fn poll_album_textures(&mut self, ctx: &egui::Context) {
        while let Ok((epoch, album_index, image)) = self.texture_receiver.try_recv() {
            self.texture_requests.remove(&(epoch, album_index));
            if epoch != self.texture_epoch {
                continue;
            }
            let Some(image) = image else {
                continue;
            };
            let texture = ctx.load_texture(
                format!("album-art-{epoch}-{album_index}"),
                image,
                egui::TextureOptions::LINEAR,
            );
            const MAX_CACHED_ALBUM_TEXTURES: usize = 48;
            while self.textures.len() >= MAX_CACHED_ALBUM_TEXTURES {
                let Some(oldest_position) = self
                    .texture_lru
                    .iter()
                    .position(|index| !self.visible_texture_indices.contains(index))
                else {
                    // Maximized windows can show more covers than the normal
                    // cache budget. Keep visible covers resident and trim them
                    // after they scroll out of view.
                    break;
                };
                let oldest = self.texture_lru.remove(oldest_position).unwrap();
                self.textures.remove(&oldest);
            }
            self.textures.insert(album_index, texture);
            self.texture_lru.push_back(album_index);
        }
    }
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
        let popup_active = self.show_album_details || self.show_folder_dialog || self.show_settings;
        let escape_pressed = popup_active
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if escape_pressed {
            if self.show_album_details {
                self.show_album_details = false;
            } else {
                self.show_folder_dialog = false;
                self.show_settings = false;
            }
        }
        let space_pressed = !ctx.wants_keyboard_input()
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
            self.texture_epoch = self.texture_epoch.wrapping_add(1);
            self.texture_requests.clear();
            self.textures.clear();
            self.texture_lru.clear();
        }
        self.poll_file_watcher(ctx);
        self.poll_album_textures(ctx);
        self.visible_texture_indices.clear();
        if self.scanning {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.playback.is_playing() {
            self.playback.advance_if_finished(&self.library.tracks);
            ctx.request_repaint_after(Duration::from_millis(400));
        }

        views::topbar::show(ctx, self);
        views::sidepanel::show(ctx, self);
        views::scrubber_controls::show(ctx, self);
        match self.page {
            Page::Home => views::home::show(ctx),
            Page::Albums => {
                views::album_view::show_details(ctx, self);
                views::album_view::show(ctx, self);
            }
        }
        views::dialogs::show_startup(ctx, self);
        views::dialogs::show_settings(ctx, self);
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
