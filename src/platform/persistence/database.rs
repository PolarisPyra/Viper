use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::Connection;
use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
    time::Duration,
};

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

/// Open the application database, apply pending migrations, and enable required pragmas.
///
/// # Returns
/// A ready SQLite connection.
///
/// # Errors
/// Returns an I/O error if the database cannot be opened or migrated.
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

    if (4..8).contains(&version) {
        use rusqlite::OptionalExtension;

        let legacy_password = connection
            .query_row(
                "SELECT password FROM samba_settings WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?;
        if let Some(password) = legacy_password.filter(|password| !password.is_empty()) {
            save_smb_password(&password)?;
        }
    }

    if version < 8 {
        connection
            .pragma_update(None, "secure_delete", "ON")
            .map_err(database_error)?;
        let transaction = connection.transaction().map_err(database_error)?;
        transaction
            .execute_batch(include_str!("migrations/0008_smb_password_env.sql"))
            .map_err(database_error)?;
        transaction
            .pragma_update(None, "user_version", 8)
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(database_error)?;
    }

    Ok(connection)
}

/// Return the app-specific `.env` path used for SMB credentials.
pub(crate) fn env_file_path() -> io::Result<PathBuf> {
    let database_path = database_path()?;
    let directory = database_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "database path has no parent directory",
        )
    })?;
    Ok(directory.join(".env"))
}

/// Load the SMB password from the app `.env` file or process environment.
pub(crate) fn load_smb_password() -> io::Result<String> {
    if let Some(password) = env::var_os("VIPER_SMB_PASSWORD") {
        return password.into_string().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "SMB password environment variable is not valid UTF-8",
            )
        });
    }
    let env_path = env_file_path()?;
    if env_path.exists() {
        dotenvy::from_path(&env_path)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    }
    let Some(encoded) = env::var_os("VIPER_SMB_PASSWORD_B64") else {
        return Ok(String::new());
    };
    let encoded = encoded.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "SMB password is not valid UTF-8",
        )
    })?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Save the SMB password in a private app `.env` file and the process environment.
pub(crate) fn save_smb_password(password: &str) -> io::Result<()> {
    let env_path = env_file_path()?;
    let encoded = STANDARD.encode(password);
    let contents = format!("VIPER_SMB_PASSWORD_B64={encoded}\n");
    let temporary_path = env_path.with_extension("env.tmp");
    let parent = env_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "environment file path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary_path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary_path, fs::Permissions::from_mode(0o600))?;
    }
    fs::rename(&temporary_path, &env_path)?;
    env::remove_var("VIPER_SMB_PASSWORD");
    env::set_var("VIPER_SMB_PASSWORD_B64", encoded);
    Ok(())
}

pub(crate) fn database_error(error: rusqlite::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}
