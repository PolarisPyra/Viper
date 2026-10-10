use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, ErrorKind},
    path::PathBuf,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Page shown when the application starts.
pub enum StartupView {
    /// Open the home screen.
    #[default]
    Home,
    /// Open the album browser.
    Albums,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Sort order for the album browser.
pub enum AlbumSort {
    /// Sort by album artist.
    AlbumArtist,
    /// Sort by the stable library identifier.
    Id,
    /// Sort by album artist metadata.
    Artist,
    /// Sort by total album duration.
    Duration,
    /// Sort by play count, highest first.
    MostPlayed,
    /// Sort by album name.
    #[default]
    Name,
    /// Shuffle the album order.
    Random,
    /// Sort by user rating.
    Rating,
    /// Sort by when the album was added.
    RecentlyAdded,
    /// Sort by most recent playback.
    RecentlyPlayed,
    /// Sort by number of tracks.
    SongCount,
    /// Place favorite albums first.
    Favorited,
    /// Sort by release year.
    ReleaseYear,
}

impl AlbumSort {
    /// Every album sorting option shown by the application.
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

    /// Return the display label for this sorting option.
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
/// Presentation style for the album browser.
pub enum AlbumLayout {
    /// Show albums as a cover grid.
    #[default]
    Grid,
    /// Show albums as a text list.
    List,
}

#[derive(Clone, Debug, Default)]
/// Saved SMB share location and authentication details.
pub struct SambaSettings {
    /// SMB URL identifying the server and share.
    pub share_url: String,
    /// Username used to authenticate to the share.
    pub username: String,
    /// Redacted password stored in the app-specific `.env` file.
    pub password: SecretString,
    /// SMB workgroup or domain.
    pub workgroup: String,
}

/// Load saved SMB connection details.
///
/// # Returns
/// The saved connection, or `None` when no connection is configured.
///
/// # Errors
/// Returns an I/O error if the database or password environment file cannot be read.
pub fn load_samba_settings() -> io::Result<Option<SambaSettings>> {
    use rusqlite::OptionalExtension;

    let connection = super::database::open()?;
    let credentials = connection
        .query_row(
            "SELECT share_url, username, workgroup
             FROM samba_settings WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(super::database::database_error)?;
    credentials
        .map(|(share_url, username, workgroup)| {
            Ok(SambaSettings {
                share_url,
                username,
                password: SecretString::from(super::database::load_smb_password()?),
                workgroup,
            })
        })
        .transpose()
}

/// Persist SMB connection details.
///
/// # Arguments
/// * `settings` - Share URL and credentials to save.
///
/// # Errors
/// Returns an I/O error if the database or password environment file cannot be updated.
pub fn save_samba_settings(settings: &SambaSettings) -> io::Result<()> {
    let connection = super::database::open()?;
    super::database::save_smb_password(settings.password.expose_secret())?;
    connection
        .execute(
            "INSERT INTO samba_settings (id, share_url, username, workgroup)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
                share_url=excluded.share_url,
                username=excluded.username,
                workgroup=excluded.workgroup",
            rusqlite::params![settings.share_url, settings.username, settings.workgroup,],
        )
        .map_err(super::database::database_error)?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
/// Persisted application preferences and library state.
pub struct Settings {
    /// Selected local folder or SMB share URL.
    pub music_path: Option<PathBuf>,
    /// Last saved application window size in logical pixels.
    pub window_size: Option<[f32; 2]>,
    /// Page opened when the application starts.
    pub startup_view: StartupView,
    /// Current left sidebar width in logical pixels.
    pub left_panel_width: f32,
    /// Whether the left sidebar is hidden.
    pub left_panel_hidden: bool,
    /// Whether sidebar entries display icons without their labels.
    pub compact_sidebar: bool,
    /// Selected application color theme.
    pub theme: crate::shared::ui::theme::ThemeId,
    /// Current right panel width in logical pixels.
    pub right_panel_width: f32,
    /// Playback volume as an integer percentage from 0 to 100.
    pub volume: u8,
    /// Selected album sort order.
    pub album_sort: AlbumSort,
    /// Whether album sorting is ascending.
    pub sort_ascending: bool,
    /// Stable keys of albums marked as favorites.
    pub favorite_albums: BTreeSet<String>,
    /// User ratings keyed by stable album key.
    pub album_ratings: BTreeMap<String, u8>,
    /// Playback counts keyed by stable album key.
    pub album_play_counts: BTreeMap<String, u64>,
    /// Unix timestamps of each album's most recent playback.
    pub album_last_played: BTreeMap<String, u64>,
    /// Unix timestamps recording when each album was added.
    pub album_added: BTreeMap<String, u64>,
    /// Optional Discord Rich Presence application ID.
    pub discord_application_id: Option<String>,
    /// Selected album presentation layout.
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
            compact_sidebar: false,
            theme: Default::default(),
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
    /// Load settings from SQLite, migrating legacy settings when necessary.
    ///
    /// # Returns
    /// The stored settings, or defaults when no settings exist.
    ///
    /// # Errors
    /// Returns an I/O error when settings cannot be decoded or persisted.
    pub fn load() -> io::Result<Self> {
        use rusqlite::OptionalExtension;

        let connection = super::database::open()?;
        let stored = connection
            .query_row(
                "SELECT music_path, window_width, window_height, startup_view,
                        left_panel_width, left_panel_hidden, right_panel_width, volume,
                        album_sort, sort_ascending, discord_application_id, album_layout, compact_sidebar, theme
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
                        row.get::<_, bool>(12)?,
                        row.get::<_, String>(13)?,
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
            compact_sidebar,
            theme,
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
                compact_sidebar,
                theme: decode_setting(&theme)?,
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

    /// Save settings and album state in one database transaction.
    ///
    /// # Errors
    /// Returns an I/O error if encoding or database writes fail.
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
                album_sort, sort_ascending, discord_application_id, album_layout, compact_sidebar, theme
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO UPDATE SET
                music_path=excluded.music_path, window_width=excluded.window_width,
                window_height=excluded.window_height, startup_view=excluded.startup_view,
                left_panel_width=excluded.left_panel_width, left_panel_hidden=excluded.left_panel_hidden,
                right_panel_width=excluded.right_panel_width, volume=excluded.volume,
                album_sort=excluded.album_sort, sort_ascending=excluded.sort_ascending,
                discord_application_id=excluded.discord_application_id,
                album_layout=excluded.album_layout,
                compact_sidebar=excluded.compact_sidebar,
                theme=excluded.theme",
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
                self.compact_sidebar,
                encode_setting(&self.theme)?,
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

    /// Return the path to the application SQLite database.
    ///
    /// # Returns
    /// The configured database path.
    ///
    /// # Errors
    /// Returns an I/O error if the configuration directory is unavailable.
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
