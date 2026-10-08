use crate::{
    library::{Album, Track},
    storage::settings::{AlbumSort, Settings},
};

pub(crate) fn album_sort_key(artist: &str, title: &str) -> String {
    format!("{artist}\0{title}")
}

pub(super) fn sort_indices(
    indices: &mut [usize],
    albums: &[Album],
    tracks: &[Track],
    settings: &Settings,
    seed: u64,
) {
    let sort = settings.album_sort;
    let ascending = settings.sort_ascending;
    indices.sort_by(|left_index, right_index| {
        let left = &albums[*left_index];
        let right = &albums[*right_index];
        let left_key = album_sort_key(&left.artist, &left.title);
        let right_key = album_sort_key(&right.artist, &right.title);
        let order = match sort {
            AlbumSort::AlbumArtist => left.artist.to_lowercase().cmp(&right.artist.to_lowercase()),
            AlbumSort::Id => left_index.cmp(right_index),
            AlbumSort::Artist => {
                let left_artist = left
                    .tracks
                    .first()
                    .and_then(|index| tracks.get(*index))
                    .map_or("", |track| track.artist.as_str());
                let right_artist = right
                    .tracks
                    .first()
                    .and_then(|index| tracks.get(*index))
                    .map_or("", |track| track.artist.as_str());
                left_artist.to_lowercase().cmp(&right_artist.to_lowercase())
            }
            AlbumSort::Duration => album_duration(left, tracks).cmp(&album_duration(right, tracks)),
            AlbumSort::MostPlayed => settings
                .album_play_counts
                .get(&left_key)
                .copied()
                .unwrap_or_default()
                .cmp(
                    &settings
                        .album_play_counts
                        .get(&right_key)
                        .copied()
                        .unwrap_or_default(),
                ),
            AlbumSort::Name => left.title.to_lowercase().cmp(&right.title.to_lowercase()),
            AlbumSort::Random => {
                random_sort_hash(&left_key, seed).cmp(&random_sort_hash(&right_key, seed))
            }
            AlbumSort::Rating => settings
                .album_ratings
                .get(&left_key)
                .copied()
                .unwrap_or_default()
                .cmp(
                    &settings
                        .album_ratings
                        .get(&right_key)
                        .copied()
                        .unwrap_or_default(),
                ),
            AlbumSort::RecentlyAdded => settings
                .album_added
                .get(&left_key)
                .copied()
                .unwrap_or_default()
                .cmp(
                    &settings
                        .album_added
                        .get(&right_key)
                        .copied()
                        .unwrap_or_default(),
                ),
            AlbumSort::RecentlyPlayed => settings
                .album_last_played
                .get(&left_key)
                .copied()
                .unwrap_or_default()
                .cmp(
                    &settings
                        .album_last_played
                        .get(&right_key)
                        .copied()
                        .unwrap_or_default(),
                ),
            AlbumSort::SongCount => left.tracks.len().cmp(&right.tracks.len()),
            AlbumSort::Favorited => settings
                .favorite_albums
                .contains(&left_key)
                .cmp(&settings.favorite_albums.contains(&right_key)),
            AlbumSort::ReleaseYear => {
                album_release_year(left, tracks).cmp(&album_release_year(right, tracks))
            }
        };
        let order = if ascending { order } else { order.reverse() };
        if order == std::cmp::Ordering::Equal && sort != AlbumSort::Random {
            left.title
                .to_lowercase()
                .cmp(&right.title.to_lowercase())
                .then_with(|| left_index.cmp(right_index))
        } else {
            order
        }
    });
}

fn album_duration(album: &Album, tracks: &[Track]) -> u64 {
    album
        .tracks
        .iter()
        .filter_map(|index| tracks.get(*index).and_then(|track| track.duration_ms))
        .fold(0, u64::saturating_add)
}

fn album_release_year(album: &Album, tracks: &[Track]) -> u32 {
    album
        .tracks
        .iter()
        .filter_map(|index| tracks.get(*index).and_then(|track| track.release_year))
        .min()
        .unwrap_or_default()
}

fn random_sort_hash(key: &str, seed: u64) -> u64 {
    key.bytes().fold(seed ^ 0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}
