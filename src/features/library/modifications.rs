use super::{
    album_key,
    model::{Album, DiscoveredTracks, Library},
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

/// /// Merge newly discovered tracks into an existing library.
/// ///
/// /// # Arguments
/// /// * `library` - Library to update.
/// /// * `discovered` - Tracks and artwork from an incremental scan.
/// ///
/// /// # Returns
/// /// Number of tracks added.
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

/// /// Remove tracks under the supplied paths and return old-to-new track indexes.
/// ///
/// /// # Arguments
/// /// * `library` - Library to update.
/// /// * `removed_paths` - Removed file or directory paths.
/// ///
/// /// # Returns
/// /// An index remap with `None` for removed tracks.
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
