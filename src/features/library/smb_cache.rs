//! Persistent SMB tags and deduplicated artwork, scoped to the scanned root.
use super::Track;
use crate::platform::persistence::database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, sync::Arc};

const VERSION: u8 = 1;

#[derive(Serialize, Deserialize)]
struct Entry {
    version: u8,
    fingerprint: String,
    track: Track,
    artwork: Option<i64>,
}

#[derive(Default)]
pub(super) struct SmbMetadataCache {
    entries: HashMap<String, Entry>,
    artwork: HashMap<i64, Arc<[u8]>>,
    artwork_ids: HashMap<Arc<[u8]>, i64>,
}

impl SmbMetadataCache {
    pub(super) fn load(root: &Path) -> Self {
        Self::load_inner(root).unwrap_or_else(|error| {
            eprintln!("Could not load SMB metadata cache: {error}");
            Self::default()
        })
    }

    fn load_inner(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut connection = database::open()?;
        // Read tags and artwork from the same database snapshot.
        let connection = connection.transaction()?;
        let mut cache = Self::default();
        let root = root.to_string_lossy();
        let mut statement =
            connection.prepare("SELECT id, data FROM smb_artwork WHERE root = ?1")?;
        let rows = statement.query_map([root.as_ref()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;
        for row in rows {
            let (id, data) = row?;
            cache.artwork.insert(id, Arc::from(data));
        }
        let mut statement =
            connection.prepare("SELECT path, data FROM smb_metadata WHERE root = ?1")?;
        let rows = statement.query_map([root.as_ref()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;
        for row in rows {
            let (path, data) = row?;
            // Corrupt or outdated entries are reread from the server.
            if let Ok(entry) = serde_json::from_slice::<Entry>(&data) {
                if entry.version == VERSION {
                    cache.entries.insert(path, entry);
                }
            }
        }
        Ok(cache)
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn get(&self, path: &Path, fingerprint: &str) -> Option<(Track, Option<Arc<[u8]>>)> {
        let entry = self.entries.get(path.to_str()?)?;
        if entry.fingerprint != fingerprint {
            return None;
        }
        let artwork = match entry.artwork {
            Some(id) => Some(self.artwork.get(&id)?.clone()),
            None => None,
        };
        let mut track = entry.track.clone();
        track.path = path.to_owned();
        Some((track, artwork))
    }

    pub(super) fn insert(
        &mut self,
        fingerprint: String,
        mut track: Track,
        artwork: Option<Arc<[u8]>>,
    ) {
        let artwork = artwork.map(|data| {
            let next_id = self.artwork_ids.len() as i64;
            let id = *self.artwork_ids.entry(data.clone()).or_insert(next_id);
            self.artwork.entry(id).or_insert(data);
            id
        });
        let path = track.path.to_string_lossy().into_owned();
        track.path.clear();
        self.entries.insert(
            path,
            Entry {
                version: VERSION,
                fingerprint,
                track,
                artwork,
            },
        );
    }

    pub(super) fn save(&self, root: &Path) {
        if let Err(error) = self.save_inner(root) {
            eprintln!("Could not save SMB metadata cache: {error}");
        }
    }

    fn save_inner(&self, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut connection = database::open()?;
        let transaction = connection.transaction()?;
        let root = root.to_string_lossy();
        transaction.execute("DELETE FROM smb_metadata WHERE root = ?1", [root.as_ref()])?;
        transaction.execute("DELETE FROM smb_artwork WHERE root = ?1", [root.as_ref()])?;
        {
            let mut statement = transaction
                .prepare("INSERT INTO smb_metadata (root, path, data) VALUES (?1, ?2, ?3)")?;
            for (path, entry) in &self.entries {
                statement.execute(params![root.as_ref(), path, serde_json::to_vec(entry)?])?;
            }
            let mut statement = transaction
                .prepare("INSERT INTO smb_artwork (root, id, data) VALUES (?1, ?2, ?3)")?;
            for (id, data) in &self.artwork {
                statement.execute(params![root.as_ref(), id, data.as_ref()])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }
}
