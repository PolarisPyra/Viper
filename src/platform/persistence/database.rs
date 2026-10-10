use rusqlite::Connection;
use std::{env, io, path::PathBuf, time::Duration};

const INITIAL_SCHEMA: &str = include_str!("migrations/0001_initial.sql");
const DISCORD_PRESENCE_SCHEMA: &str = include_str!("migrations/0002_discord_application_id.sql");
const ALBUM_LAYOUT_SCHEMA: &str = include_str!("migrations/0003_album_layout.sql");
const SAMBA_SETTINGS_SCHEMA: &str = include_str!("migrations/0004_samba_settings.sql");

pub(crate) fn database_path() -> io::Result<PathBuf> {
    Ok(config_directory()?.join("viper").join("viper.sqlite3"))
}

pub(crate) fn legacy_database_path() -> io::Result<PathBuf> {
    Ok(config_directory()?
        .join("musicplayer")
        .join("musicplayer.sqlite3"))
}

fn config_directory() -> io::Result<PathBuf> {
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory is unavailable"))?;
    Ok(home.join(".config"))
}

pub(crate) fn open() -> io::Result<Connection> {
    let path = database_path()?;
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "database path has no parent directory",
        )
    })?;
    std::fs::create_dir_all(parent)?;

    if !path.exists() {
        let legacy_path = legacy_database_path()?;
        if legacy_path.exists() {
            let legacy_connection = Connection::open(legacy_path).map_err(database_error)?;
            legacy_connection
                .backup("main", &path, None)
                .map_err(database_error)?;
        }
    }

    let mut connection = Connection::open(path).map_err(database_error)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(database_error)?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(database_error)?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(database_error)?;

    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(database_error)?;
    if version < 1 {
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(INITIAL_SCHEMA)
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", 1)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
    }
    if version < 2 {
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(DISCORD_PRESENCE_SCHEMA)
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", 2)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
    }
    if version < 3 {
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(ALBUM_LAYOUT_SCHEMA)
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", 3)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
    }
    if version < 4 {
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(SAMBA_SETTINGS_SCHEMA)
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", 4)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
    }

    if version < 5 {
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(include_str!("migrations/0005_smb_cache.sql"))
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", 5)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
    }

    if version < 6 {
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(include_str!("migrations/0006_compact_sidebar.sql"))
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", 6)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
    }

    if version < 7 {
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(include_str!("migrations/0007_theme.sql"))
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", 7)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
    }

    Ok(connection)
}

pub(crate) fn database_error(error: rusqlite::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}
