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

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "aac", "aiff", "wma", "ape", "wv", "dsf",
    "dff", "webm",
];

pub struct FileWatcher {
    // Keeping the watcher alive keeps the recursive OS watch registered.
    _watcher: RecommendedWatcher,
    events: Receiver<notify::Result<Event>>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FileChange {
    Upsert(PathBuf),
    Remove(PathBuf),
}

impl FileWatcher {
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

    pub fn poll(&self) -> Result<Vec<FileChange>, String> {
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
                Ok(Err(error)) => return Err(error.to_string()),
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

fn is_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            AUDIO_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}
