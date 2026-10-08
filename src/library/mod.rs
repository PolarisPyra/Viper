pub mod cache;
mod model;
mod scan;
mod tag_reader;
pub mod watcher;

pub use model::{Album, AudioProperties, DiscoveredTracks, Library, ScanProgress, Track};
pub use scan::{merge_discovered_tracks, remove_library_tracks, scan_added_tracks, scan_library};
