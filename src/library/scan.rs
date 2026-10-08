use crate::{
    artwork::cover_for_track,
    library::{
        cache::TrackMetadataCache,
        model::{Album, DiscoveredTracks, Library, ScanProgress, Track},
    },
};
use rayon::prelude::*;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use super::tag_reader::read_metadata;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "aac", "aiff", "wma", "ape", "wv", "dsf",
    "dff", "webm",
];
pub fn scan_library(root: &Path, cancel: &AtomicBool, progress: &ScanProgress) -> Library {
    let cache = TrackMetadataCache::load();
    let mut refreshed_cache = TrackMetadataCache::default();
    let mut audio_paths = Vec::new();
    let mut unreadable_directories = 0;
    scan_dir(
        root,
        root,
        &mut audio_paths,
        &mut unreadable_directories,
        &mut HashSet::new(),
        cancel,
    );
    progress.phase.store(1, Ordering::Relaxed);
    progress.total.store(audio_paths.len(), Ordering::Relaxed);
    let scan_path = |path: &PathBuf| {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let metadata = fs::metadata(path).ok();
        let track = metadata
            .as_ref()
            .and_then(|metadata| cache.get(path, metadata))
            .unwrap_or_else(|| scan_track(path.clone()));
        progress.completed.fetch_add(1, Ordering::Relaxed);
        Some((track, metadata))
    };
    let worker_count = std::thread::available_parallelism().map_or(4, |count| count.get().min(8));
    let scanned: Vec<_> = match rayon::ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
    {
        Ok(pool) => pool.install(|| audio_paths.par_iter().map(scan_path).collect()),
        Err(_) => audio_paths.iter().map(scan_path).collect(),
    };
    // The paths are now owned by the scanned Track values; release the original
    // directory-walk list before assembling the final library and cover art.
    drop(audio_paths);
    drop(cache);
    if cancel.load(Ordering::Relaxed) {
        return Library::default();
    }
    let mut tracks = Vec::with_capacity(scanned.len());
    for (track, cache_metadata) in scanned.into_iter().flatten() {
        if let Some(metadata) = cache_metadata {
            refreshed_cache.insert(track.path.clone(), &metadata, track.clone());
        }
        tracks.push(track);
    }
    if cancel.load(Ordering::Relaxed) {
        return Library::default();
    }
    refreshed_cache.save();
    tracks.sort_by(|a, b| a.path.cmp(&b.path));

    let mut groups: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (index, track) in tracks.iter().enumerate() {
        // Keep tracks with missing album tags playable, but don't manufacture an album for them.
        if track.album.trim().is_empty() {
            continue;
        }
        groups
            .entry((
                normalize_album_key(&track.album),
                normalize_album_key(&track.album_artist),
            ))
            .or_default()
            .push(index);
    }

    let mut albums = Vec::with_capacity(groups.len());
    let mut track_album = vec![usize::MAX; tracks.len()];
    progress.total.fetch_add(groups.len(), Ordering::Relaxed);
    progress.phase.store(2, Ordering::Relaxed);
    for (_, mut album_tracks) in groups {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        album_tracks.sort_by(|left, right| {
            let left = &tracks[*left];
            let right = &tracks[*right];
            left.disc_number
                .unwrap_or(1)
                .cmp(&right.disc_number.unwrap_or(1))
                .then_with(|| {
                    left.track_number
                        .unwrap_or(u32::MAX)
                        .cmp(&right.track_number.unwrap_or(u32::MAX))
                })
                .then_with(|| left.path.cmp(&right.path))
        });
        let title = tracks[album_tracks[0]].album.trim().to_owned();
        let artist = tracks[album_tracks[0]].album_artist.trim().to_owned();
        let art = cover_for_track(&tracks[album_tracks[0]]).map(Arc::from);
        let album_index = albums.len();
        for track_index in &album_tracks {
            track_album[*track_index] = album_index;
        }
        albums.push(Album {
            title,
            artist,
            tracks: album_tracks,
            art,
        });
        progress.completed.fetch_add(1, Ordering::Relaxed);
    }

    Library {
        tracks,
        albums,
        track_album,
        unreadable_directories,
    }
}

pub fn scan_added_tracks(paths: &[PathBuf], cancel: &AtomicBool) -> DiscoveredTracks {
    let mut candidates = Vec::new();
    let mut visited = HashSet::new();
    let mut unreadable_directories = 0;
    for path in paths {
        if cancel.load(Ordering::Relaxed) {
            return DiscoveredTracks {
                tracks: Vec::new(),
                album_art: HashMap::new(),
            };
        }
        if path.is_dir() {
            scan_dir(
                path,
                path,
                &mut candidates,
                &mut unreadable_directories,
                &mut visited,
                cancel,
            );
        } else if path.is_file() && is_audio_path(path) {
            candidates.push(path.clone());
        }
    }
    candidates.sort();
    candidates.dedup();
    wait_for_stable_files(&candidates, cancel);
    let worker_count = std::thread::available_parallelism().map_or(4, |count| count.get().min(8));
    let tracks: Vec<_> = match rayon::ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()
    {
        Ok(pool) => pool.install(|| {
            candidates
                .par_iter()
                .filter(|path| !cancel.load(Ordering::Relaxed) && path.is_file())
                .map(|path| scan_track(path.clone()))
                .collect()
        }),
        Err(_) => candidates
            .iter()
            .filter(|path| !cancel.load(Ordering::Relaxed) && path.is_file())
            .map(|path| scan_track(path.clone()))
            .collect(),
    };
    let mut album_art = HashMap::new();
    let mut seen_album_art = HashSet::new();
    for track in &tracks {
        if track.album.trim().is_empty() {
            continue;
        }
        let key = (
            normalize_album_key(&track.album),
            normalize_album_key(&track.album_artist),
        );
        if seen_album_art.insert(key.clone()) {
            if let Some(art) = cover_for_track(track) {
                album_art.insert(key, Arc::from(art));
            }
        }
    }
    DiscoveredTracks { tracks, album_art }
}

pub fn merge_discovered_tracks(library: &mut Library, discovered: DiscoveredTracks) -> usize {
    let mut album_indices = HashMap::with_capacity(library.albums.len());
    let mut existing_paths = HashSet::with_capacity(library.tracks.len());
    existing_paths.extend(library.tracks.iter().map(|track| track.path.clone()));
    for (index, album) in library.albums.iter().enumerate() {
        album_indices
            .entry(album_key(&album.title, &album.artist))
            .or_insert(index);
    }
    let mut added_paths = HashSet::new();
    let mut updated_albums = HashSet::new();
    let mut added_count = 0;
    for track in discovered.tracks {
        if existing_paths.contains(&track.path) || !added_paths.insert(track.path.clone()) {
            continue;
        }
        let track_index = library.tracks.len();
        let key =
            (!track.album.trim().is_empty()).then(|| album_key(&track.album, &track.album_artist));
        library.tracks.push(track);
        added_count += 1;
        let Some(key) = key else {
            library.track_album.push(usize::MAX);
            continue;
        };
        let art = discovered.album_art.get(&key).cloned();
        let album_index = if let Some(&index) = album_indices.get(&key) {
            if library.albums[index].art.is_none() {
                library.albums[index].art = art;
            }
            library.albums[index].tracks.push(track_index);
            index
        } else {
            let album = &library.tracks[track_index];
            let index = library.albums.len();
            library.albums.push(Album {
                title: album.album.trim().to_owned(),
                artist: album.album_artist.trim().to_owned(),
                tracks: vec![track_index],
                art,
            });
            album_indices.insert(key, index);
            index
        };
        library.track_album.push(album_index);
        updated_albums.insert(album_index);
    }
    for album_index in updated_albums {
        library.albums[album_index].tracks.sort_by(|left, right| {
            let left = &library.tracks[*left];
            let right = &library.tracks[*right];
            left.disc_number
                .unwrap_or(1)
                .cmp(&right.disc_number.unwrap_or(1))
                .then_with(|| {
                    left.track_number
                        .unwrap_or(u32::MAX)
                        .cmp(&right.track_number.unwrap_or(u32::MAX))
                })
                .then_with(|| left.path.cmp(&right.path))
        });
    }
    added_count
}

pub fn remove_library_tracks(
    library: &mut Library,
    removed_paths: &[PathBuf],
) -> Vec<Option<usize>> {
    let mut remap = vec![None; library.tracks.len()];
    let mut tracks = Vec::with_capacity(library.tracks.len());
    for (old_index, track) in library.tracks.drain(..).enumerate() {
        if removed_paths
            .iter()
            .any(|removed| track.path == *removed || track.path.starts_with(removed))
        {
            continue;
        }
        remap[old_index] = Some(tracks.len());
        tracks.push(track);
    }
    for album in &mut library.albums {
        album.tracks = album
            .tracks
            .iter()
            .filter_map(|&old_index| remap.get(old_index).copied().flatten())
            .collect();
    }
    library.track_album = library
        .track_album
        .iter()
        .enumerate()
        .filter_map(|(old_index, &album_index)| remap[old_index].map(|_| album_index))
        .collect();
    library.tracks = tracks;
    remap
}

fn wait_for_stable_files(paths: &[PathBuf], cancel: &AtomicBool) {
    let signature = |path: &Path| {
        let metadata = fs::metadata(path).ok()?;
        Some((metadata.len(), metadata.modified().ok()?))
    };
    let mut previous: Vec<_> = paths.iter().map(|path| signature(path)).collect();
    for _ in 0..20 {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(Duration::from_millis(150));
        let current: Vec<_> = paths.iter().map(|path| signature(path)).collect();
        if current == previous {
            return;
        }
        previous = current;
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

fn album_key(title: &str, artist: &str) -> (String, String) {
    (normalize_album_key(title), normalize_album_key(artist))
}

fn normalize_album_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn scan_dir(
    root: &Path,
    path: &Path,
    audio_paths: &mut Vec<PathBuf>,
    unreadable_directories: &mut usize,
    visited: &mut HashSet<PathBuf>,
    cancel: &AtomicBool,
) {
    if cancel.load(Ordering::Relaxed) {
        return;
    }
    let Ok(canonical_path) = fs::canonicalize(path) else {
        *unreadable_directories += 1;
        return;
    };
    if !visited.insert(canonical_path) {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        *unreadable_directories += 1;
        return;
    };
    for entry in entries.flatten() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            scan_dir(
                root,
                &path,
                audio_paths,
                unreadable_directories,
                visited,
                cancel,
            );
            continue;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !AUDIO_EXTENSIONS.contains(&extension.as_str()) {
            continue;
        }

        audio_paths.push(path);
    }
}

fn scan_track(path: PathBuf) -> Track {
    let metadata = read_metadata(&path);
    Track {
        path,
        title: metadata.title.unwrap_or_default(),
        artist: metadata.artist.unwrap_or_default(),
        album_artist: metadata.album_artist.unwrap_or_default(),
        album: metadata.album.unwrap_or_default(),
        disc_number: metadata.disc_number,
        track_number: metadata.track_number,
        duration_ms: metadata.duration_ms,
        release_year: metadata.release_year,
        audio: metadata.audio,
    }
}
