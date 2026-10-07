use serde::{Deserialize, Serialize};
use std::{
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub music_path: Option<PathBuf>,
    pub startup_view: StartupView,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub volume: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            music_path: None,
            startup_view: StartupView::Home,
            left_panel_width: 240.0,
            right_panel_width: 320.0,
            volume: 70,
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
