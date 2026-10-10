//! Recursive local filesystem watching for library changes.
use super::is_audio_path;
use eframe::egui;
use notify::{
    event::{ModifyKind, RenameMode},
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
};
use thiserror::Error;

/// Watches a local music folder and requests UI repaints for filesystem events.
pub struct FileWatcher {
    // Keeping the watcher alive keeps the recursive OS watch registered.
    _watcher: RecommendedWatcher,
    events: Receiver<notify::Result<Event>>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// A relevant file or directory change detected by [`FileWatcher`].
pub enum FileChange {
    /// A file or directory was created or changed.
    Upsert(PathBuf),
    /// A file or directory was removed.
    Remove(PathBuf),
}

/// Error returned when a filesystem watcher reports an operating-system failure.
#[derive(Debug, Error)]
pub enum FileWatcherError {
    /// The platform watcher reported an error while polling the event queue.
    #[error("filesystem watcher failed: {0}")]
    Notify(#[from] notify::Error),
}

impl FileWatcher {
    /// Start recursively watching a local music folder.
    ///
    /// # Arguments
    /// * `root` - Folder to watch.
    /// * `repaint` - egui context used to request redraws on events.
    ///
    /// # Returns
    /// An active watcher.
    ///
    /// # Errors
    /// Returns a notify error if the watcher cannot be created or registered.
    pub fn new(root: &Path, repaint: egui::Context) -> notify::Result<Self> {
        let (sender, events) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
            repaint.request_repaint();
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            events,
        })
    }

    /// Collect queued filesystem events and collapse duplicate paths.
    ///
    /// # Returns
    /// Relevant file changes since the previous poll.
    ///
    /// # Errors
    /// Returns [`FileWatcherError`] if the operating system reports a watcher failure.
    pub fn poll(&self) -> Result<Vec<FileChange>, FileWatcherError> {
        let mut changed = HashSet::new();
        loop {
            match self.events.try_recv() {
                Ok(Ok(event)) => {
                    let kind = match event.kind {
                        EventKind::Create(_) => 0,
                        EventKind::Remove(_) => 1,
                        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => 1,
                        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => 4,
                        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => 3,
                        EventKind::Modify(ModifyKind::Name(_)) => 5,
                        EventKind::Modify(ModifyKind::Any | ModifyKind::Other) => 5,
                        EventKind::Modify(_) => 2,
                        _ => continue,
                    };
                    for (index, path) in event.paths.into_iter().enumerate() {
                        let path_kind = match kind {
                            4 if index == 0 => 1,
                            4 => 3,
                            5 if path.exists() => 3,
                            5 => 1,
                            other => other,
                        };
                        let relevant = match path_kind {
                            0 | 3 => is_audio_path(&path) || path.is_dir(),
                            1 => true,
                            _ => is_audio_path(&path),
                        };
                        if relevant {
                            changed.insert((path_kind, path));
                        }
                    }
                }
                Ok(Err(error)) => return Err(FileWatcherError::from(error)),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        Ok(changed
            .into_iter()
            .map(|(kind, path)| {
                if kind == 1 {
                    FileChange::Remove(path)
                } else {
                    FileChange::Upsert(path)
                }
            })
            .collect())
    }
}
