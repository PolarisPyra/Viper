use crate::{library::Track, storage::settings::Settings};
use lofty::file::TaggedFileExt;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::Hasher,
    io::{self, Cursor, ErrorKind},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    time::UNIX_EPOCH,
};

static ARTWORK_CACHE_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn cover_for_track(track: &Track) -> Option<Vec<u8>> {
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
