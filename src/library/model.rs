use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::AtomicUsize, Arc},
};

#[derive(Clone, Deserialize, Serialize)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
    pub duration_ms: Option<u64>,
    pub release_year: Option<u32>,
    #[serde(default)]
    pub audio: AudioProperties,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct AudioProperties {
    pub bitrate_kbps: Option<u32>,
    pub sample_rate_hz: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
}

#[derive(Clone)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub tracks: Vec<usize>,
    pub art: Option<Arc<[u8]>>,
}

#[derive(Default)]
pub struct Library {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub track_album: Vec<usize>,
    pub unreadable_directories: usize,
}

pub struct DiscoveredTracks {
    pub tracks: Vec<Track>,
    pub(super) album_art: HashMap<(String, String), Arc<[u8]>>,
}

#[derive(Default)]
pub struct ScanProgress {
    pub completed: AtomicUsize,
    pub total: AtomicUsize,
    pub phase: AtomicUsize,
}
