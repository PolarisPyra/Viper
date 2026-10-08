use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{self, ErrorKind},
    path::PathBuf,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupView {
    #[default]
    Home,
    Albums,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlbumSort {
    AlbumArtist,
    Id,
    Artist,
    Duration,
    MostPlayed,
    #[default]
    Name,
    Random,
    Rating,
    RecentlyAdded,
    RecentlyPlayed,
    SongCount,
    Favorited,
    ReleaseYear,
}

impl AlbumSort {
    pub const ALL: [Self; 13] = [
        Self::AlbumArtist,
        Self::Id,
        Self::Artist,
        Self::Duration,
        Self::MostPlayed,
        Self::Name,
        Self::Random,
        Self::Rating,
        Self::RecentlyAdded,
        Self::RecentlyPlayed,
        Self::SongCount,
        Self::Favorited,
        Self::ReleaseYear,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AlbumArtist => "Album Artist",
            Self::Id => "ID",
            Self::Artist => "Artist",
            Self::Duration => "Duration",
            Self::MostPlayed => "Most played",
            Self::Name => "Name",
            Self::Random => "Random",
            Self::Rating => "Rating",
            Self::RecentlyAdded => "Recently added",
            Self::RecentlyPlayed => "Recently played",
            Self::SongCount => "Song count",
            Self::Favorited => "Favorited",
            Self::ReleaseYear => "Release year",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub music_path: Option<PathBuf>,
    pub window_size: Option<[f32; 2]>,
    pub startup_view: StartupView,
    pub left_panel_width: f32,
    pub left_panel_hidden: bool,
    pub right_panel_width: f32,
    pub volume: u8,
    pub album_sort: AlbumSort,
    pub sort_ascending: bool,
    pub favorite_albums: BTreeSet<String>,
    pub album_ratings: BTreeMap<String, u8>,
    pub album_play_counts: BTreeMap<String, u64>,
    pub album_last_played: BTreeMap<String, u64>,
    pub album_added: BTreeMap<String, u64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            music_path: None,
            window_size: None,
            startup_view: StartupView::Home,
            left_panel_width: 240.0,
            left_panel_hidden: false,
            right_panel_width: 320.0,
            volume: 70,
            album_sort: AlbumSort::Name,
            sort_ascending: true,
            favorite_albums: BTreeSet::new(),
            album_ratings: BTreeMap::new(),
            album_play_counts: BTreeMap::new(),
            album_last_played: BTreeMap::new(),
            album_added: BTreeMap::new(),
        }
    }
}

impl Settings {
    pub fn load() -> io::Result<Self> {
        let path = Self::file_path()?;
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| io::Error::new(ErrorKind::InvalidData, error)),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let previous_path = path.with_file_name("musicplayer.json");
                match fs::read(&previous_path) {
                    Ok(bytes) => {
                        let settings = serde_json::from_slice(&bytes)
                            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
                        if fs::write(&path, bytes).is_ok() {
                            let _ = fs::remove_file(previous_path);
                        }
                        Ok(settings)
                    }
                    Err(previous_error) if previous_error.kind() == ErrorKind::NotFound => {
                        Ok(Self::default())
                    }
                    Err(previous_error) => Err(previous_error),
                }
            }

            Err(error) => Err(error),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::file_path()?;
        let directory = path.parent().ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidInput,
                "settings path has no parent directory",
            )
        })?;
        fs::create_dir_all(directory)?;
        let temporary_path = path.with_extension("json.tmp");
        let contents = serde_json::to_vec_pretty(self)
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;
        fs::write(&temporary_path, contents)?;
        fs::rename(temporary_path, path)
    }

    pub fn file_path() -> io::Result<PathBuf> {
        let home = env::var_os("HOME")
            .or_else(|| env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "home directory is unavailable"))?;
        Ok(home
            .join(".config")
            .join("musicplayer")
            .join("settings.json"))
    }
}
