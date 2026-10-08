use super::model::AudioProperties;
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::ItemKey,
};
use std::{path::Path, process::Command};

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
}

pub(super) fn read_metadata(path: &Path) -> TrackMetadata {
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
        if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
            let text = |key| {
                tag.get_string(key)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            };
            let number = |key| text(key).and_then(|value| value.split('/').next()?.parse().ok());
            return TrackMetadata {
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
            };
        }
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
        };
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
        return empty_metadata();
    };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return empty_metadata();
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

    TrackMetadata {
        title: lookup("title"),
        artist: lookup("artist"),
        album: lookup("album"),
        album_artist: lookup("album_artist").or_else(|| lookup("albumartist")),
        disc_number: parse_number(lookup("disc").or_else(|| lookup("discnumber"))),
        track_number: parse_number(lookup("track").or_else(|| lookup("tracknumber"))),
        duration_ms: json
            .get("format")
            .and_then(|format| format.get("duration"))
            .and_then(serde_json::Value::as_str)
            .and_then(|duration| duration.parse::<f64>().ok())
            .map(|seconds| (seconds.max(0.0) * 1000.0) as u64),
        release_year: lookup("date")
            .or_else(|| lookup("year"))
            .and_then(|date| date.get(..4)?.parse::<u32>().ok()),
        audio,
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
    }
}
