use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlbumLayout {
    #[default]
    Grid,
    List,
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
    pub discord_application_id: Option<String>,
    pub album_layout: AlbumLayout,
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
            discord_application_id: None,
            album_layout: AlbumLayout::Grid,
        }
    }
}

impl Settings {
    pub fn load() -> io::Result<Self> {
        use rusqlite::OptionalExtension;

        let connection = super::database::open()?;
        let stored = connection
            .query_row(
                "SELECT music_path, window_width, window_height, startup_view,
                        left_panel_width, left_panel_hidden, right_panel_width, volume,
                        album_sort, sort_ascending, discord_application_id, album_layout
                 FROM app_settings WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<f64>>(1)?,
                        row.get::<_, Option<f64>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, f32>(4)?,
                        row.get::<_, bool>(5)?,
                        row.get::<_, f32>(6)?,
                        row.get::<_, u8>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, bool>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, String>(11)?,
                    ))
                },
            )
            .optional()
            .map_err(super::database::database_error)?;

        let mut settings = if let Some((
            music_path,
            window_width,
            window_height,
            startup_view,
            left_panel_width,
            left_panel_hidden,
            right_panel_width,
            volume,
            album_sort,
            sort_ascending,
            discord_application_id,
            album_layout,
        )) = stored
        {
            Self {
                music_path: music_path.map(PathBuf::from),
                window_size: window_width
                    .zip(window_height)
                    .map(|(width, height)| [width as f32, height as f32]),
                startup_view: decode_setting(&startup_view)?,
                left_panel_width,
                left_panel_hidden,
                right_panel_width,
                volume,
                album_sort: decode_setting(&album_sort)?,
                sort_ascending,
                discord_application_id,
                album_layout: decode_setting(&album_layout)?,
                ..Self::default()
            }
        } else {
            load_legacy_settings()?.unwrap_or_default()
        };

        {
            let mut statement = connection
                .prepare("SELECT album_key, favorite, rating, play_count, last_played, added FROM album_state")
                .map_err(super::database::database_error)?;
            let mut rows = statement
                .query([])
                .map_err(super::database::database_error)?;
            while let Some(row) = rows.next().map_err(super::database::database_error)? {
                let key: String = row.get(0).map_err(super::database::database_error)?;
                if row
                    .get::<_, bool>(1)
                    .map_err(super::database::database_error)?
                {
                    settings.favorite_albums.insert(key.clone());
                }
                if let Some(value) = row
                    .get::<_, Option<u8>>(2)
                    .map_err(super::database::database_error)?
                {
                    settings.album_ratings.insert(key.clone(), value);
                }
                if let Some(value) = row
                    .get::<_, Option<i64>>(3)
                    .map_err(super::database::database_error)?
                    .and_then(|value| u64::try_from(value).ok())
                {
                    settings.album_play_counts.insert(key.clone(), value);
                }
                if let Some(value) = row
                    .get::<_, Option<i64>>(4)
                    .map_err(super::database::database_error)?
                    .and_then(|value| u64::try_from(value).ok())
                {
                    settings.album_last_played.insert(key.clone(), value);
                }
                if let Some(value) = row
                    .get::<_, Option<i64>>(5)
                    .map_err(super::database::database_error)?
                    .and_then(|value| u64::try_from(value).ok())
                {
                    settings.album_added.insert(key, value);
                }
            }
        }

        if !connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM app_settings WHERE id = 1)",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(super::database::database_error)?
        {
            settings.save()?;
            for path in legacy_settings_paths()? {
                let _ = fs::remove_file(path);
            }
        }
        Ok(settings)
    }

    pub fn save(&self) -> io::Result<()> {
        let mut connection = super::database::open()?;
        let transaction = connection
            .transaction()
            .map_err(super::database::database_error)?;
        let (window_width, window_height) =
            self.window_size.map_or((None, None), |[width, height]| {
                (Some(f64::from(width)), Some(f64::from(height)))
            });
        transaction.execute(
            "INSERT INTO app_settings (
                id, music_path, window_width, window_height, startup_view,
                left_panel_width, left_panel_hidden, right_panel_width, volume,
                album_sort, sort_ascending, discord_application_id, album_layout
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                music_path=excluded.music_path, window_width=excluded.window_width,
                window_height=excluded.window_height, startup_view=excluded.startup_view,
                left_panel_width=excluded.left_panel_width, left_panel_hidden=excluded.left_panel_hidden,
                right_panel_width=excluded.right_panel_width, volume=excluded.volume,
                album_sort=excluded.album_sort, sort_ascending=excluded.sort_ascending,
                discord_application_id=excluded.discord_application_id,
                album_layout=excluded.album_layout",
            rusqlite::params![
                self.music_path.as_ref().map(|path| path.to_string_lossy().into_owned()),
                window_width,
                window_height,
                encode_setting(&self.startup_view)?,
                self.left_panel_width,
                self.left_panel_hidden,
                self.right_panel_width,
                self.volume,
                encode_setting(&self.album_sort)?,
                self.sort_ascending,
                self.discord_application_id,
                encode_setting(&self.album_layout)?,
            ],
        ).map_err(super::database::database_error)?;

        transaction
            .execute("DELETE FROM album_state", [])
            .map_err(super::database::database_error)?;
        let keys: BTreeSet<_> = self
            .favorite_albums
            .iter()
            .chain(self.album_ratings.keys())
            .chain(self.album_play_counts.keys())
            .chain(self.album_last_played.keys())
            .chain(self.album_added.keys())
            .collect();
        for key in keys {
            transaction.execute(
                "INSERT INTO album_state (album_key, favorite, rating, play_count, last_played, added)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    key,
                    self.favorite_albums.contains(key),
                    self.album_ratings.get(key).copied(),
                    self.album_play_counts.get(key).copied().map(sqlite_integer),
                    self.album_last_played.get(key).copied().map(sqlite_integer),
                    self.album_added.get(key).copied().map(sqlite_integer),
                ],
            ).map_err(super::database::database_error)?;
        }
        transaction
            .commit()
            .map_err(super::database::database_error)
    }

    pub fn file_path() -> io::Result<PathBuf> {
        super::database::database_path()
    }
}

fn encode_setting<T: Serialize>(value: &T) -> io::Result<String> {
    serde_json::to_string(value).map_err(|error| io::Error::new(ErrorKind::InvalidData, error))
}

fn decode_setting<T: for<'de> Deserialize<'de>>(value: &str) -> io::Result<T> {
    serde_json::from_str(value).map_err(|error| io::Error::new(ErrorKind::InvalidData, error))
}

fn sqlite_integer(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn legacy_settings_paths() -> io::Result<[PathBuf; 4]> {
    let database = super::database::database_path()?;
    let legacy_database = super::database::legacy_database_path()?;
    Ok([
        database.with_file_name("settings.json"),
        database.with_file_name("musicplayer.json"),
        legacy_database.with_file_name("settings.json"),
        legacy_database.with_file_name("musicplayer.json"),
    ])
}

fn load_legacy_settings() -> io::Result<Option<Settings>> {
    for path in legacy_settings_paths()? {
        match fs::read(path) {
            Ok(bytes) => {
                return serde_json::from_slice(&bytes)
                    .map(Some)
                    .map_err(|error| io::Error::new(ErrorKind::InvalidData, error));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}
