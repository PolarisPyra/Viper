mod analyzer;
mod audio;
pub mod cache;
mod model;
mod modifications;
pub(crate) mod sorting;
pub(crate) mod symphonia;
mod tag_reader;
pub mod watcher;

pub use analyzer::{scan_added_tracks, scan_library};
pub(crate) use audio::{album_key, is_audio_path, normalize_album_key};
pub use model::{Album, AudioProperties, DiscoveredTracks, Library, ScanProgress, Track};
pub use modifications::{merge_discovered_tracks, remove_library_tracks};
