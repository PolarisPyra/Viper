use crate::{caching::TrackMetadataCache, storage::settings::Settings};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::ItemKey,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap, HashSet},
    fs,
    hash::Hasher,
    io::{self, Cursor, ErrorKind},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, UNIX_EPOCH},
};

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "aac", "aiff", "wma", "ape", "wv", "dsf",
    "dff", "webm",
];
static ARTWORK_CACHE_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

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
    album_art: HashMap<(String, String), Arc<[u8]>>,
}

#[derive(Default)]
pub struct ScanProgress {
    pub completed: AtomicUsize,
    pub total: AtomicUsize,
    pub phase: AtomicUsize,
}

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
        let art = cover_for_track(&tracks[album_tracks[0]]);
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
                album_art.insert(key, art);
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

fn cover_for_track(track: &Track) -> Option<Arc<[u8]>> {
    let folder = track.path.parent()?;
    find_cover_in(folder)
        .or_else(|| {
            if is_disc_directory(folder) {
                folder.parent().and_then(find_cover_in)
            } else {
                None
            }
        })
        .or_else(|| prepared_cover_for_track(&track.path))
        .map(Arc::from)
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
    let (
        tag_title,
        tag_artist,
        tag_album,
        tag_album_artist,
        disc_number,
        track_number,
        duration_ms,
        release_year,
        audio,
    ) = read_metadata(&path);
    Track {
        path,
        title: tag_title.unwrap_or_default(),
        artist: tag_artist.unwrap_or_default(),
        album_artist: tag_album_artist.unwrap_or_default(),
        album: tag_album.unwrap_or_default(),
        disc_number,
        track_number,
        duration_ms,
        release_year,
        audio,
    }
}

fn read_metadata(
    path: &Path,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<u32>,
    Option<u64>,
    Option<u32>,
    AudioProperties,
) {
    if let Ok(file) = lofty::read_from_path(path) {
        let properties = file.properties();
        let duration_ms = {
            let millis = properties.duration().as_millis() as u64;
            (millis > 0).then_some(millis)
        };
        let audio = AudioProperties {
            bitrate_kbps: properties.audio_bitrate(),
            sample_rate_hz: properties.sample_rate(),
            bit_depth: properties.bit_depth(),
            channels: properties.channels(),
        };
        let values = if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
            let text = |key| {
                tag.get_string(key)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            };
            let number = |key| text(key).and_then(|value| value.split('/').next()?.parse().ok());
            (
                text(ItemKey::TrackTitle),
                text(ItemKey::TrackArtist),
                text(ItemKey::AlbumTitle),
                text(ItemKey::AlbumArtist),
                number(ItemKey::DiscNumber),
                number(ItemKey::TrackNumber),
                duration_ms,
                text(ItemKey::RecordingDate).and_then(|date| date.get(..4)?.parse::<u32>().ok()),
                audio,
            )
        } else {
            (None, None, None, None, None, None, duration_ms, None, audio)
        };
        return values;
    }
    // Formats outside Lofty's supported set still use ffprobe as a fallback.
    let Ok(output) = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-show_entries",
            "format=duration,bit_rate:format_tags=title,artist,album,album_artist,albumartist,track,tracknumber,disc,discnumber,date,year:stream=sample_rate,channels,bits_per_sample,bits_per_raw_sample,bit_rate",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
    else {
        return (
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            AudioProperties::default(),
        );
    };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return (
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            AudioProperties::default(),
        );
    };
    let tags = json.get("format").and_then(|format| format.get("tags"));
    let stream = json
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .and_then(|streams| streams.first());
    let parse_property = |value: Option<&serde_json::Value>| {
        value.and_then(|value| {
            value
                .as_str()
                .and_then(|text| text.parse().ok())
                .or_else(|| value.as_u64().and_then(|number| u32::try_from(number).ok()))
        })
    };
    let bitrate_bps = parse_property(
        stream
            .and_then(|stream| stream.get("bit_rate"))
            .or_else(|| json.get("format").and_then(|format| format.get("bit_rate"))),
    );
    let audio = AudioProperties {
        bitrate_kbps: bitrate_bps.map(|bitrate| bitrate / 1000),
        sample_rate_hz: parse_property(stream.and_then(|stream| stream.get("sample_rate"))),
        bit_depth: parse_property(
            stream
                .and_then(|stream| stream.get("bits_per_raw_sample"))
                .or_else(|| stream.and_then(|stream| stream.get("bits_per_sample"))),
        )
        .and_then(|depth| u8::try_from(depth).ok()),
        channels: parse_property(stream.and_then(|stream| stream.get("channels")))
            .and_then(|channels| u8::try_from(channels).ok()),
    };
    let lookup = |key: &str| {
        tags?
            .as_object()?
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .and_then(|(_, value)| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let parse_number = |value: Option<String>| {
        value.and_then(|tag| tag.split('/').next()?.trim().parse::<u32>().ok())
    };
    (
        lookup("title"),
        lookup("artist"),
        lookup("album"),
        lookup("album_artist").or_else(|| lookup("albumartist")),
        parse_number(lookup("disc").or_else(|| lookup("discnumber"))),
        parse_number(lookup("track").or_else(|| lookup("tracknumber"))),
        json.get("format")
            .and_then(|format| format.get("duration"))
            .and_then(serde_json::Value::as_str)
            .and_then(|duration| duration.parse::<f64>().ok())
            .map(|seconds| (seconds.max(0.0) * 1000.0) as u64),
        lookup("date")
            .or_else(|| lookup("year"))
            .and_then(|date| date.get(..4)?.parse::<u32>().ok()),
        audio,
    )
}

fn extract_embedded_art(path: &Path) -> Option<Vec<u8>> {
    if let Ok(file) = lofty::read_from_path(path) {
        return file
            .primary_tag()
            .or_else(|| file.first_tag())
            .and_then(|tag| tag.pictures().first())
            .map(|picture| picture.data().to_vec());
    }
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args([
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-f",
            "image2pipe",
            "-vcodec",
            "mjpeg",
            "pipe:1",
        ])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then_some(output.stdout)
        .filter(|bytes| !bytes.is_empty())
}

fn is_disc_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let normalized: String = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect();
    ["disc", "disk", "cd"].iter().any(|prefix| {
        normalized
            .strip_prefix(prefix)
            .is_some_and(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()))
    })
}

fn find_cover_in(folder: &Path) -> Option<Vec<u8>> {
    [
        "cover.jpg",
        "cover.jpeg",
        "cover.png",
        "Cover.jpg",
        "front.jpg",
        "front.png",
        "folder.jpg",
        "folder.jpeg",
        "folder.png",
        "album.jpg",
        "album.png",
        "artwork.jpg",
    ]
    .iter()
    .find_map(|name| prepared_cover_for_source(&folder.join(name)))
}

fn prepared_cover_for_track(path: &Path) -> Option<Vec<u8>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!(
                "Could not inspect audio file for artwork cache {}: {error}",
                path.display()
            );
            return extract_embedded_art(path).and_then(|bytes| prepare_cover(&bytes));
        }
    };
    let cache_path = match artwork_cache_path(path, &metadata) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Could not locate album artwork cache: {error}");
            return extract_embedded_art(path).and_then(|bytes| prepare_cover(&bytes));
        }
    };
    read_or_create_prepared_cover(&cache_path, || {
        extract_embedded_art(path).and_then(|bytes| prepare_cover(&bytes))
    })
}

fn prepared_cover_for_source(path: &Path) -> Option<Vec<u8>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => return None,
        Err(error) if error.kind() == ErrorKind::NotFound => return None,
        Err(error) => {
            eprintln!(
                "Could not inspect album artwork source {}: {error}",
                path.display()
            );
            return None;
        }
    };
    let cache_path = match artwork_cache_path(path, &metadata) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Could not locate album artwork cache: {error}");
            return fs::read(path).ok().and_then(|bytes| prepare_cover(&bytes));
        }
    };
    read_or_create_prepared_cover(&cache_path, || match fs::read(path) {
        Ok(bytes) => prepare_cover(&bytes),
        Err(error) => {
            eprintln!(
                "Could not read album artwork source {}: {error}",
                path.display()
            );
            None
        }
    })
}

fn read_or_create_prepared_cover(
    cache_path: &Path,
    create: impl FnOnce() -> Option<Vec<u8>>,
) -> Option<Vec<u8>> {
    match fs::read(cache_path) {
        Ok(bytes) => {
            if image::load_from_memory(&bytes).is_ok() {
                return Some(bytes);
            }
            eprintln!(
                "Invalid prepared artwork cache {}, rebuilding it",
                cache_path.display()
            );
            if let Err(error) = fs::remove_file(cache_path) {
                if error.kind() != ErrorKind::NotFound {
                    eprintln!(
                        "Could not remove invalid album artwork cache {}: {error}",
                        cache_path.display()
                    );
                }
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => eprintln!(
            "Could not read album artwork cache {}: {error}",
            cache_path.display()
        ),
    }

    let bytes = create()?;
    let Some(directory) = cache_path.parent() else {
        eprintln!("Album artwork cache path has no parent directory");
        return Some(bytes);
    };
    if let Err(error) = fs::create_dir_all(directory) {
        eprintln!(
            "Could not create album artwork cache {}: {error}",
            directory.display()
        );
        return Some(bytes);
    }
    let temporary_id = ARTWORK_CACHE_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let temporary_path = cache_path.with_extension(format!("{temporary_id}.tmp"));
    if let Err(error) = fs::write(&temporary_path, &bytes) {
        eprintln!(
            "Could not write album artwork cache {}: {error}",
            temporary_path.display()
        );
        return Some(bytes);
    }
    if let Err(error) = fs::rename(&temporary_path, cache_path) {
        eprintln!(
            "Could not save album artwork cache {}: {error}",
            cache_path.display()
        );
        if let Err(remove_error) = fs::remove_file(&temporary_path) {
            if remove_error.kind() != ErrorKind::NotFound {
                eprintln!(
                    "Could not remove temporary album artwork cache {}: {remove_error}",
                    temporary_path.display()
                );
            }
        }
    }
    Some(bytes)
}

fn artwork_cache_path(source_path: &Path, metadata: &fs::Metadata) -> io::Result<PathBuf> {
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    let settings_path = Settings::file_path()?;
    let mut hasher = DefaultHasher::new();
    hasher.write(source_path.as_os_str().to_string_lossy().as_bytes());
    hasher.write_u64(metadata.len());
    hasher.write_u64(modified.as_secs());
    hasher.write_u32(modified.subsec_nanos());
    Ok(settings_path
        .with_file_name("cached")
        .join("art-source-v1")
        .join(format!("{:016x}.img", hasher.finish())))
}

fn prepare_cover(bytes: &[u8]) -> Option<Vec<u8>> {
    const MAX_COVER_EDGE: u32 = 320;
    const MAX_SOURCE_EDGE: u32 = 4096;
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_EDGE);
    limits.max_image_height = Some(MAX_SOURCE_EDGE);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    let thumbnail = decoded.thumbnail(MAX_COVER_EDGE, MAX_COVER_EDGE);
    let mut output = Cursor::new(Vec::new());
    if decoded.color().has_alpha() {
        let rgba = thumbnail.to_rgba8();
        if rgba.pixels().any(|pixel| pixel[3] != u8::MAX) {
            image::DynamicImage::ImageRgba8(rgba)
                .write_to(&mut output, image::ImageFormat::Png)
                .ok()?;
        } else {
            let rgb = image::DynamicImage::ImageRgba8(rgba).to_rgb8();
            let rgb = image::DynamicImage::ImageRgb8(rgb);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 88)
                .encode_image(&rgb)
                .ok()?;
        }
    } else {
        let rgb = image::DynamicImage::ImageRgb8(thumbnail.to_rgb8());
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 88)
            .encode_image(&rgb)
            .ok()?;
    }
    Some(output.into_inner())
}
