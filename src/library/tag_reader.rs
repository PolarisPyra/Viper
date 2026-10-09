use super::model::AudioProperties;
use lofty::{
    file::{AudioFile, FileType, TaggedFile, TaggedFileExt},
    probe::Probe,
    tag::ItemKey,
};
use std::{
    io::{BufReader, Read, Seek},
    path::Path,
};
use symphonia::core::meta::StandardTagKey;

pub(super) struct TrackMetadata {
    pub(super) title: Option<String>,
    pub(super) artist: Option<String>,
    pub(super) album: Option<String>,
    pub(super) album_artist: Option<String>,
    pub(super) disc_number: Option<u32>,
    pub(super) track_number: Option<u32>,
    pub(super) duration_ms: Option<u64>,
    pub(super) release_year: Option<u32>,
    pub(super) audio: AudioProperties,
    pub(super) artwork: Option<Vec<u8>>,
}

impl TrackMetadata {
    pub(super) fn has_tags(&self) -> bool {
        self.title.is_some()
            || self.artist.is_some()
            || self.album.is_some()
            || self.album_artist.is_some()
            || self.disc_number.is_some()
            || self.track_number.is_some()
            || self.release_year.is_some()
    }
}

pub(super) fn read_metadata(path: &Path) -> TrackMetadata {
    if let Ok(file) = lofty::read_from_path(path) {
        return metadata_from_tagged_file(file, false);
    }

    let Some(media) = super::symphonia::probe(path) else {
        return empty_metadata();
    };
    let lookup = |standard_key: StandardTagKey, aliases: &[&str]| {
        media
            .tags
            .iter()
            .find(|tag| {
                tag.standard_key == Some(standard_key)
                    || aliases
                        .iter()
                        .any(|alias| tag.key.eq_ignore_ascii_case(alias))
            })
            .map(|tag| tag.value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let number = |standard_key, aliases| {
        lookup(standard_key, aliases).and_then(|value| value.split('/').next()?.trim().parse().ok())
    };

    TrackMetadata {
        title: lookup(StandardTagKey::TrackTitle, &["title", "tracktitle"]),
        artist: lookup(StandardTagKey::Artist, &["artist"]),
        album: lookup(StandardTagKey::Album, &["album"]),
        album_artist: lookup(
            StandardTagKey::AlbumArtist,
            &["album_artist", "albumartist"],
        ),
        disc_number: number(StandardTagKey::DiscNumber, &["disc", "discnumber"]),
        track_number: number(StandardTagKey::TrackNumber, &["track", "tracknumber"]),
        duration_ms: media.duration_ms,
        release_year: lookup(
            StandardTagKey::Date,
            &[
                "date",
                "year",
                "recordingdate",
                "originaldate",
                "releasedate",
            ],
        )
        .and_then(|date| date.get(..4)?.parse().ok()),
        audio: media.audio,
        artwork: None,
    }
}

pub(super) fn read_metadata_from_reader<R: Read + Seek>(
    reader: R,
    path: &Path,
) -> Option<TrackMetadata> {
    let reader = BufReader::with_capacity(128 * 1024, reader);
    let probe = match FileType::from_path(path) {
        Some(file_type) => Probe::with_file_type(reader, file_type),
        None => Probe::new(reader).guess_file_type().ok()?,
    };
    Some(metadata_from_tagged_file(probe.read().ok()?, true))
}

fn metadata_from_tagged_file(file: TaggedFile, include_artwork: bool) -> TrackMetadata {
    let properties = file.properties();
    let millis = properties.duration().as_millis() as u64;
    let duration_ms = (millis > 0).then_some(millis);
    let audio = AudioProperties {
        bitrate_kbps: properties.audio_bitrate(),
        sample_rate_hz: properties.sample_rate(),
        bit_depth: properties.bit_depth(),
        channels: properties.channels(),
    };
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let artwork = include_artwork
        .then(|| {
            tag.and_then(|tag| tag.pictures().first())
                .map(|picture| picture.data().to_vec())
        })
        .flatten();
    let Some(tag) = tag else {
        return TrackMetadata {
            title: None,
            artist: None,
            album: None,
            album_artist: None,
            disc_number: None,
            track_number: None,
            duration_ms,
            release_year: None,
            audio,
            artwork,
        };
    };
    let text = |key| {
        tag.get_string(key)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let number = |key| text(key).and_then(|value| value.split('/').next()?.parse().ok());
    TrackMetadata {
        title: text(ItemKey::TrackTitle),
        artist: text(ItemKey::TrackArtist),
        album: text(ItemKey::AlbumTitle),
        album_artist: text(ItemKey::AlbumArtist),
        disc_number: number(ItemKey::DiscNumber),
        track_number: number(ItemKey::TrackNumber),
        duration_ms,
        release_year: text(ItemKey::RecordingDate)
            .and_then(|date| date.get(..4)?.parse::<u32>().ok()),
        audio,
        artwork,
    }
}

fn empty_metadata() -> TrackMetadata {
    TrackMetadata {
        title: None,
        artist: None,
        album: None,
        album_artist: None,
        disc_number: None,
        track_number: None,
        duration_ms: None,
        release_year: None,
        audio: AudioProperties::default(),
        artwork: None,
    }
}
