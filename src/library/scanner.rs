use super::{is_audio_path, normalize_album_key};
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
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use super::tag_reader::read_metadata;

pub fn scan_library(root: &Path, cancel: &AtomicBool, progress: &ScanProgress) -> Library {
    #[cfg(target_os = "linux")]
    return scan_library_with_auth(root, cancel, progress, None);
    #[cfg(not(target_os = "linux"))]
    scan_library_with_auth(root, cancel, progress)
}

pub(crate) fn scan_library_with_auth(
    root: &Path,
    cancel: &AtomicBool,
    progress: &ScanProgress,
    #[cfg(target_os = "linux")] smb_auth: Option<super::smb::SmbAuth>,
) -> Library {
    let cache = TrackMetadataCache::load();
    let mut refreshed_cache = TrackMetadataCache::default();
    let missing_metadata_tracks = AtomicUsize::new(0);
    let mut audio_paths = Vec::new();
    let mut unreadable_directories = 0;
    #[cfg(target_os = "linux")]
    let is_smb = super::smb::is_smb_path(root);
    #[cfg(not(target_os = "linux"))]
    let is_smb = false;
    if is_smb {
        #[cfg(target_os = "linux")]
        {
            if let Some(auth) = smb_auth.as_ref() {
                match super::smb::scan(&root.to_string_lossy(), auth) {
                    Ok(paths) => audio_paths = paths,
                    Err(error) => {
                        eprintln!("Could not scan SMB music share: {error}");
                        unreadable_directories = 1;
                    }
                }
            } else {
                unreadable_directories = 1;
            }
        }
    } else {
        scan_dir(
            root,
            root,
            &mut audio_paths,
            &mut unreadable_directories,
            &mut HashSet::new(),
            cancel,
        );
    }
    #[cfg(target_os = "linux")]
    let smb_session = if is_smb {
        let Some(auth) = smb_auth.as_ref() else {
            return Library {
                scan_error: Some("Reconnect to the SMB share before scanning".into()),
                ..Library::default()
            };
        };
        match super::smb::SmbSession::new(&root.to_string_lossy(), auth) {
            Ok(session) => Some(session),
            Err(error) => {
                return Library {
                    scan_error: Some(format!("Could not open SMB share for scanning: {error}")),
                    ..Library::default()
                };
            }
        }
    } else {
        None
    };
    #[cfg(target_os = "linux")]
    let mut smb_album_art: HashMap<(String, String), Arc<[u8]>> = HashMap::new();
    progress.phase.store(1, Ordering::Relaxed);
    progress.total.store(audio_paths.len(), Ordering::Relaxed);
    let scan_path = |path: &PathBuf| {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let metadata = fs::metadata(path).ok();
        let mut track = metadata
            .as_ref()
            .and_then(|metadata| cache.get(path, metadata))
            .unwrap_or_else(|| scan_track(path.clone()));
        if !track.metadata_missing && track_has_no_metadata(&track) {
            track.metadata_missing = true;
        }
        fill_missing_display_metadata(&mut track);
        if track.metadata_missing {
            missing_metadata_tracks.fetch_add(1, Ordering::Relaxed);
        }
        progress.completed.fetch_add(1, Ordering::Relaxed);
        Some((track, metadata))
    };
    let worker_count = std::thread::available_parallelism().map_or(4, |count| count.get().min(8));
    #[cfg(target_os = "linux")]
    let scanned: Vec<_> = if is_smb {
        let session = smb_session
            .as_ref()
            .expect("SMB scan session was initialized");
        let mut scanned = Vec::with_capacity(audio_paths.len());
        for path in &audio_paths {
            if cancel.load(Ordering::Relaxed) {
                return Library::default();
            }
            let mut metadata = match session.read_metadata(path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    return Library {
                        scan_error: Some(format!(
                            "Could not read tags from SMB track {}: {error}. The scan was stopped.",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        )),
                        ..Library::default()
                    };
                }
            };
            let embedded_art = metadata.artwork.take();
            let mut track = track_from_metadata(path.clone(), metadata);
            fill_missing_display_metadata(&mut track);
            if track.metadata_missing {
                missing_metadata_tracks.fetch_add(1, Ordering::Relaxed);
            }
            if !track.album.trim().is_empty() {
                if let Some(art) = embedded_art {
                    smb_album_art
                        .entry((
                            normalize_album_key(&track.album),
                            normalize_album_key(&track.album_artist),
                        ))
                        .or_insert_with(|| Arc::from(art));
                }
            }
            progress.completed.fetch_add(1, Ordering::Relaxed);
            scanned.push(Some((track, None)));
        }
        scanned
    } else {
        match rayon::ThreadPoolBuilder::new()
            .num_threads(worker_count)
            .build()
        {
            Ok(pool) => pool.install(|| audio_paths.par_iter().map(scan_path).collect()),
            Err(_) => audio_paths.iter().map(scan_path).collect(),
        }
    };
    #[cfg(not(target_os = "linux"))]
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
        #[cfg(target_os = "linux")]
        let art = if is_smb {
            smb_album_art
                .get(&(
                    normalize_album_key(&tracks[album_tracks[0]].album),
                    normalize_album_key(&tracks[album_tracks[0]].album_artist),
                ))
                .cloned()
        } else {
            cover_for_track(&tracks[album_tracks[0]]).map(Arc::from)
        };
        #[cfg(not(target_os = "linux"))]
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
        scan_error: None,
        missing_metadata_tracks: missing_metadata_tracks.load(Ordering::Relaxed),
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
        if !is_audio_path(&path) {
            continue;
        }

        audio_paths.push(path);
    }
}

fn scan_track(path: PathBuf) -> Track {
    let metadata = read_metadata(&path);
    track_from_metadata(path, metadata)
}

fn track_from_metadata(path: PathBuf, metadata: super::tag_reader::TrackMetadata) -> Track {
    let metadata_missing = !metadata.has_tags();
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
        metadata_missing,
    }
}

fn track_has_no_metadata(track: &Track) -> bool {
    track.title.trim().is_empty()
        && track.artist.trim().is_empty()
        && track.album_artist.trim().is_empty()
        && track.album.trim().is_empty()
        && track.disc_number.is_none()
        && track.track_number.is_none()
        && track.release_year.is_none()
}

fn fill_missing_display_metadata(track: &mut Track) {
    if track.title.trim().is_empty() {
        track.title = track
            .path
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
    }
    if track.album.trim().is_empty() {
        track.album = track
            .path
            .parent()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
    }
}
