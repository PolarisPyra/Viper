use super::ViperApp;
use std::path::PathBuf;

impl ViperApp {
    pub(super) fn start_scan(&mut self, root: PathBuf) {
        self.playback
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .stop();
        self.last_observed_track = None;
        self.artwork_cache.clear();
        self.error = None;

        #[cfg(target_os = "linux")]
        let smb_auth = self.smb_auth.clone();
        if let Err(error) = self.library.start_scan(
            root,
            #[cfg(target_os = "linux")]
            smb_auth,
        ) {
            self.error = Some(error);
        }
        #[cfg(target_os = "linux")]
        self.mpris.set_library(
            std::sync::Arc::clone(&self.library.model.tracks),
            std::sync::Arc::clone(&self.library.model.track_album),
            &self.library.model.albums,
        );
    }

    pub(super) fn poll_scan(&mut self) -> bool {
        let Some(completion) = self.library.poll_scan() else {
            return false;
        };
        #[cfg(target_os = "linux")]
        self.mpris.set_library(
            std::sync::Arc::clone(&self.library.model.tracks),
            std::sync::Arc::clone(&self.library.model.track_album),
            &self.library.model.albums,
        );
        match completion {
            crate::features::library::state::ScanCompletion::Loaded { warning } => {
                if let Some(warning) = warning {
                    self.error = Some(warning);
                }
                self.record_new_albums();
            }
            crate::features::library::state::ScanCompletion::Failed(error) => {
                self.error = Some(error);
            }
        }
        true
    }
}
