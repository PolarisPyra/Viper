use std::path::Path;

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "aac", "aiff", "wma", "ape", "wv", "dsf",
    "dff", "webm",
];

pub(crate) fn is_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            AUDIO_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

pub(crate) fn album_key(title: &str, artist: &str) -> (String, String) {
    (normalize_album_key(title), normalize_album_key(artist))
}

pub(crate) fn normalize_album_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}
