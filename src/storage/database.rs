use rusqlite::Connection;
use std::{env, io, path::PathBuf, time::Duration};

const INITIAL_SCHEMA: &str = include_str!("migrations/0001_initial.sql");
const SCHEMA_VERSION: i64 = 1;

pub(crate) fn database_path() -> io::Result<PathBuf> {
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory is unavailable"))?;
    Ok(home
        .join(".config")
        .join("musicplayer")
        .join("musicplayer.sqlite3"))
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
    if version < SCHEMA_VERSION {
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(INITIAL_SCHEMA)
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
    }

    Ok(connection)
}

pub(crate) fn database_error(error: rusqlite::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}
