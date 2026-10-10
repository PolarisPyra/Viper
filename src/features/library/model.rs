use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{atomic::AtomicUsize, Arc},
};

/// Tagged metadata and playback properties for one audio track.
#[derive(Clone, Deserialize, Serialize)]
pub struct Track {
    /// Track file path, including an SMB URL when the track is remote.
    pub path: PathBuf,
    /// Display title, populated from tags or the file name.
    pub title: String,
    /// Track artist, or an empty string when unavailable.
    pub artist: String,
    /// Album artist, or an empty string when unavailable.
    pub album_artist: String,
    /// Album title, or an empty string when unavailable.
    pub album: String,
    /// Disc number from the track tags, if present.
    pub disc_number: Option<u32>,
    /// Track number from the track tags, if present.
    pub track_number: Option<u32>,
    /// Track duration in milliseconds, if known.
    pub duration_ms: Option<u64>,
    /// Release year from the track tags, if present.
    pub release_year: Option<u32>,
    /// Audio stream properties.
    #[serde(default)]
    pub audio: AudioProperties,
    /// Whether no usable descriptive tags were found.
    #[serde(default)]
    pub metadata_missing: bool,
}

/// Technical properties of an audio stream.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct AudioProperties {
    /// Average bitrate in kilobits per second, if available.
    pub bitrate_kbps: Option<u32>,
    /// Sample rate in hertz, if available.
    pub sample_rate_hz: Option<u32>,
    /// Sample bit depth, if available.
    pub bit_depth: Option<u8>,
    /// Number of audio channels, if available.
    pub channels: Option<u8>,
}

/// An album and the indexes of its tracks in a library.
#[derive(Clone)]
pub struct Album {
    /// Album title.
    pub title: String,
    /// Album artist.
    pub artist: String,
    /// Track indexes into [`Library::tracks`], ordered for playback.
    pub tracks: Vec<usize>,
    /// Shared encoded album artwork, if available.
    pub art: Option<Arc<[u8]>>,
}

/// Tracks and album groups discovered by a library scan.
#[derive(Default)]
pub struct Library {
    /// All discovered tracks.
    pub tracks: Vec<Track>,
    /// Albums grouped from tracks with album metadata.
    pub albums: Vec<Album>,
    /// For each track, its album index or `usize::MAX` when it has no album.
    pub track_album: Vec<usize>,
    /// Number of directories that could not be scanned.
    pub unreadable_directories: usize,
    pub(crate) scan_error: Option<String>,
    /// Tracks whose tags could not be read.
    pub missing_metadata_tracks: usize,
    /// Empty tracks skipped during scanning.
    pub skipped_empty_files: Vec<PathBuf>,
    /// Nonfatal per-track errors from the scanner.
    pub scan_errors: Vec<String>,
}

/// Newly discovered tracks and associated artwork returned by an incremental scan.
pub struct DiscoveredTracks {
    /// Tracks not yet merged into the loaded library.
    pub tracks: Vec<Track>,
    pub(super) album_art: HashMap<(String, String), Arc<[u8]>>,
}

/// Atomic progress counters for an active library scan.
#[derive(Default)]
pub struct ScanProgress {
    /// Number of scan items completed.
    pub completed: AtomicUsize,
    /// Total number of scan items known so far.
    pub total: AtomicUsize,
    /// Current phase: track scanning or album assembly.
    pub phase: AtomicUsize,
}
