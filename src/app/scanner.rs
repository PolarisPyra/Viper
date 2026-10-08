use super::MusicApp;
use crate::{
    library::watcher::{FileChange, FileWatcher},
    library::{
        merge_discovered_tracks, remove_library_tracks, scan_added_tracks, scan_library, Library,
        ScanProgress,
    },
};
use eframe::egui;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, TryRecvError},
        Arc,
    },
    time::{Duration, Instant},
};

impl MusicApp {
    pub(super) fn start_scan(&mut self, root: PathBuf) {
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
        self.last_observed_track = None;
        self.library = Library::default();
        self.artwork_cache.clear();
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

    pub(super) fn poll_file_watcher(&mut self, ctx: &egui::Context) {
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
                        self.album_sort_dirty = true;
                        self.record_new_albums();
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

    pub(super) fn poll_scan(&mut self) -> bool {
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
                self.album_sort_dirty = true;
                self.record_new_albums();
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
}
