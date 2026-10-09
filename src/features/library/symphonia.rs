use super::AudioProperties;
use std::{fs::File, path::Path};
use symphonia::core::{
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, MetadataRevision, StandardTagKey, StandardVisualKey},
    probe::Hint,
};

pub(crate) struct ProbedMedia {
    pub(crate) tags: Vec<ProbedTag>,
    pub(crate) visuals: Vec<ProbedVisual>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) audio: AudioProperties,
}

pub(crate) struct ProbedTag {
    pub(crate) standard_key: Option<StandardTagKey>,
    pub(crate) key: String,
    pub(crate) value: String,
}

pub(crate) struct ProbedVisual {
    pub(crate) usage: Option<StandardVisualKey>,
    pub(crate) data: Vec<u8>,
}

pub(crate) fn probe(path: &Path) -> Option<ProbedMedia> {
    let file = File::open(path).ok()?;
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }

    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let mut format = probed.format;

    let audio_track = format
        .tracks()
        .iter()
        .find(|track| {
            track.codec_params.channels.is_some() || track.codec_params.sample_rate.is_some()
        })
        .or_else(|| format.default_track());
    let (duration_ms, audio) = audio_track.map_or((None, AudioProperties::default()), |track| {
        let codec = &track.codec_params;
        let duration_ms = codec
            .time_base
            .zip(codec.n_frames)
            .map(|(time_base, frames)| {
                let duration = time_base.calc_time(frames);
                let milliseconds = u128::from(duration.seconds) * 1_000
                    + (duration.frac.max(0.0) * 1_000.0) as u128;
                milliseconds.min(u128::from(u64::MAX)) as u64
            });
        let channels = codec
            .channels
            .and_then(|channels| u8::try_from(channels.count()).ok());
        (
            duration_ms,
            AudioProperties {
                bitrate_kbps: None,
                sample_rate_hz: codec.sample_rate,
                bit_depth: codec
                    .bits_per_sample
                    .or(codec.bits_per_coded_sample)
                    .and_then(|depth| u8::try_from(depth).ok()),
                channels,
            },
        )
    });

    let revision = format.metadata().current().map(copy_revision).or_else(|| {
        probed
            .metadata
            .get()
            .and_then(|metadata| metadata.current().map(copy_revision))
    })?;
    let (tags, visuals) = revision;

    Some(ProbedMedia {
        tags,
        visuals,
        duration_ms,
        audio,
    })
}

fn copy_revision(revision: &MetadataRevision) -> (Vec<ProbedTag>, Vec<ProbedVisual>) {
    let tags = revision
        .tags()
        .iter()
        .map(|tag| ProbedTag {
            standard_key: tag.std_key,
            key: tag.key.clone(),
            value: tag.value.to_string(),
        })
        .collect();
    let visuals = revision
        .visuals()
        .iter()
        .map(|visual| ProbedVisual {
            usage: visual.usage,
            data: visual.data.to_vec(),
        })
        .collect();

    (tags, visuals)
}
