//! Direct SMB access through Pavão. Library paths are stored as `smb://` URLs;
//! credentials stay in memory and are never written to the settings database.

use pavao::{SmbClient, SmbCredentials, SmbDirentType, SmbOpenOptions, SmbOptions};
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Default)]
pub(crate) struct SmbAuth {
    pub username: String,
    pub password: String,
    pub workgroup: String,
}

struct SharePath {
    server: String,
    share: String,
    path: String,
}

impl SharePath {
    fn parse(url: &str) -> Result<Self, String> {
        let remainder = url
            .strip_prefix("smb://")
            .ok_or_else(|| "SMB library paths must start with smb://".to_owned())?;
        let (server, path) = remainder.split_once('/').ok_or_else(|| {
            "Enter an SMB server and share, for example smb://server/Music".to_owned()
        })?;
        let (share, path) = path.split_once('/').unwrap_or((path, ""));
        if server.is_empty() || share.is_empty() || server.contains('@') {
            return Err("Enter the server and share without embedding credentials".into());
        }
        Ok(Self {
            server: server.to_owned(),
            share: format!("/{share}"),
            path: format!("/{path}"),
        })
    }

    fn client(&self, auth: &SmbAuth) -> Result<SmbClient, String> {
        SmbClient::new(
            SmbCredentials::default()
                .server(format!("smb://{}", self.server))
                .share(&self.share)
                .username(&auth.username)
                .password(&auth.password)
                .workgroup(&auth.workgroup),
            SmbOptions::default(),
        )
        .map_err(|error| error.to_string())
    }
}

pub(crate) struct SmbSession {
    client: SmbClient,
}

impl SmbSession {
    pub(crate) fn new(root: &str, auth: &SmbAuth) -> Result<Self, String> {
        let share = SharePath::parse(root)?;
        Ok(Self {
            client: share.client(auth)?,
        })
    }

    pub(super) fn read_metadata(
        &self,
        path: &Path,
    ) -> Result<super::tag_reader::TrackMetadata, String> {
        let parsed = SharePath::parse(
            path.to_str()
                .ok_or_else(|| "SMB path is not valid UTF-8".to_owned())?,
        )?;
        let remote = self
            .client
            .open_with(&parsed.path, SmbOpenOptions::default().read(true))
            .map_err(|error| error.to_string())?;
        super::tag_reader::read_metadata_from_reader(remote, path)
            .ok_or_else(|| "could not read audio metadata from the share".to_owned())
    }

    pub(crate) fn stage_file(&self, path: &Path) -> Result<PathBuf, String> {
        let parsed = SharePath::parse(
            path.to_str()
                .ok_or_else(|| "SMB path is not valid UTF-8".to_owned())?,
        )?;
        let mut remote = self
            .client
            .open_with(&parsed.path, SmbOpenOptions::default().read(true))
            .map_err(|error| error.to_string())?;
        let name = Path::new(&parsed.path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("track.audio");
        let temp_dir = std::env::temp_dir().join(format!(
            "viper-smb-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|error| error.to_string())?;
        let temp = temp_dir.join(name);
        let mut local = File::create(&temp).map_err(|error| error.to_string())?;
        if let Err(error) = io::copy(&mut remote, &mut local) {
            let _ = std::fs::remove_file(&temp);
            let _ = std::fs::remove_dir(&temp_dir);
            return Err(error.to_string());
        }
        Ok(temp)
    }
}

pub(crate) fn validate_url(url: &str) -> Result<(), String> {
    SharePath::parse(url).map(|_| ())
}

pub(crate) fn is_smb_path(path: &std::path::Path) -> bool {
    path.to_str().is_some_and(|path| path.starts_with("smb://"))
}

pub(crate) fn scan(root: &str, auth: &SmbAuth) -> Result<Vec<PathBuf>, String> {
    let share = SharePath::parse(root)?;
    let client = share.client(auth)?;
    let mut files = Vec::new();
    let mut directories = vec![share.path.clone()];
    while let Some(directory) = directories.pop() {
        let entries = client
            .list_dir(&directory)
            .map_err(|error| error.to_string())?;
        for entry in entries {
            let name = entry.name();
            if name.is_empty() || name == "." || name == ".." {
                continue;
            }
            let path = format!("{}/{}", directory.trim_end_matches('/'), name);
            match entry.get_type() {
                SmbDirentType::Dir => directories.push(path),
                SmbDirentType::File
                    if crate::library::is_audio_path(std::path::Path::new(name)) =>
                {
                    files.push(PathBuf::from(format!(
                        "smb://{}/{}/{}",
                        share.server,
                        share.share.trim_start_matches('/'),
                        path.trim_start_matches('/')
                    )));
                }
                _ => {}
            }
        }
    }
    files.sort();
    Ok(files)
}

pub(crate) fn stage_file(path: &Path, auth: &SmbAuth) -> Result<PathBuf, String> {
    let session = SmbSession::new(
        path.to_str()
            .ok_or_else(|| "SMB path is not valid UTF-8".to_owned())?,
        auth,
    )?;
    session.stage_file(path)
}
