use super::ViperApp;
use crate::library::{scan_library, Library, ScanProgress};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, TryRecvError},
        Arc,
    },
};

impl ViperApp {
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
