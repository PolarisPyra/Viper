use crate::{metadata::Track, storage::settings::Settings};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Default)]
pub struct TrackMetadataCache {
    entries: BTreeMap<PathBuf, CachedTrack>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CachedTrack {
    size: u64,
    modified_secs: u64,
    modified_nanos: u32,
    track: Track,
}

impl TrackMetadataCache {
    pub fn load() -> Self {
        let Some(path) = cache_path() else {
            return Self::default();
        };
        let Ok(file) = File::open(path) else {
            return Self::default();
        };
        let Ok(entries) = serde_json::from_reader::<_, Vec<CachedTrack>>(BufReader::new(file))
        else {
            return Self::default();
        };
        Self {
            entries: entries
                .into_iter()
                .map(|mut entry| {
                    let path = std::mem::take(&mut entry.track.path);
                    (path, entry)
                })
                .collect(),
        }
    }

    pub fn get(&self, path: &Path, metadata: &fs::Metadata) -> Option<Track> {
        let entry = self.entries.get(path)?;
        let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
        (entry.size == metadata.len()
            && entry.modified_secs == modified.as_secs()
            && entry.modified_nanos == modified.subsec_nanos())
        .then(|| {
            let mut track = entry.track.clone();
            track.path = path.to_owned();
            track
        })
    }

    pub fn insert(&mut self, path: PathBuf, metadata: &fs::Metadata, track: Track) {
        let Ok(modified) = metadata.modified().and_then(|time| {
            time.duration_since(UNIX_EPOCH)
                .map_err(std::io::Error::other)
        }) else {
            return;
        };
        let mut track = track;
        track.path = PathBuf::new();
        self.entries.insert(
            path,
            CachedTrack {
                size: metadata.len(),
                modified_secs: modified.as_secs(),
                modified_nanos: modified.subsec_nanos(),
                track,
            },
        );
    }

    pub fn save(self) {
        let Some(path) = cache_path() else {
            return;
        };
        let Some(directory) = path.parent() else {
            return;
        };
        if fs::create_dir_all(directory).is_err() {
            return;
        }
        let entries: Vec<_> = self
            .entries
            .into_iter()
            .map(|(path, mut entry)| {
                entry.track.path = path;
                entry
            })
            .collect();
        let temporary_path = path.with_extension("json.tmp");
        let Ok(file) = File::create(&temporary_path) else {
            return;
        };
        let mut writer = BufWriter::new(file);
        if serde_json::to_writer(&mut writer, &entries).is_ok() {
            drop(writer);
            let _ = fs::rename(temporary_path, path);
        }
    }
}

fn cache_path() -> Option<PathBuf> {
    Settings::file_path()
        .ok()
        .map(|path| path.with_file_name("cached").join("tracks.json"))
}
