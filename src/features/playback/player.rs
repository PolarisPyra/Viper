use crate::features::library::{Album, Track};
use rodio::{Decoder, MixerDeviceSink, Player};
use std::{collections::VecDeque, fs::File, path::Path, time::Duration};

/// Current audio playback state, queue, history, and volume.
pub struct Playback {
    /// Index of the currently playing track in the supplied library, if any.
    pub current: Option<usize>,
    /// Most recent playback or seek error shown to the user.
    pub error: Option<String>,
    output: Option<MixerDeviceSink>,
    player: Option<Player>,
    queue: VecDeque<usize>,
    history: VecDeque<usize>,
    position: Duration,
    paused: bool,
    /// Playback volume as a percentage from 0 to 100.
    pub volume: u8,
    #[cfg(target_os = "linux")]
    smb_auth: Option<crate::features::library::smb::SmbAuth>,
}

impl Playback {
    #[cfg(target_os = "linux")]
    pub(crate) fn set_smb_auth(&mut self, auth: crate::features::library::smb::SmbAuth) {
        self.smb_auth = Some(auth);
    }

    /// Set the playback volume and apply it to the active player.
    ///
    /// # Arguments
    /// * `volume` - Volume percentage; values above 100 are clamped during playback.
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
        self.apply_volume();
    }

    /// Temporarily apply a volume value while a volume control is being adjusted.
    ///
    /// # Arguments
    /// * `volume` - Preview volume percentage; values above 100 are clamped during playback.
    pub fn preview_volume(&mut self, volume: u8) {
        self.volume = volume;
        self.apply_volume();
    }

    fn apply_volume(&self) {
        if let Some(player) = &self.player {
            player.set_volume(f32::from(self.volume.min(100)) / 100.0);
        }
    }

    /// Return whether a track is currently playing and not paused.
    pub fn is_playing(&self) -> bool {
        self.current.is_some()
            && !self.paused
            && self.player.as_ref().is_some_and(|player| !player.empty())
    }

    /// Return whether playback is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Return the current playback position.
    pub fn position(&self) -> Duration {
        if self.paused {
            self.position
        } else {
            self.player.as_ref().map_or(self.position, Player::get_pos)
        }
    }

    /// Seek within the current track, clamping the position to its known duration.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks indexed by playback state.
    /// * `position` - Requested position within the current track.
    pub fn seek(&mut self, tracks: &[Track], position: Duration) {
        let Some(index) = self.current else {
            return;
        };
        let duration = tracks
            .get(index)
            .and_then(|track| track.duration_ms)
            .map(Duration::from_millis);
        let position = duration.map_or(position, |duration| position.min(duration));
        let Some(player) = &self.player else {
            return;
        };
        match player.try_seek(position) {
            Ok(()) => self.position = position,
            Err(error) => self.error = Some(format!("Could not seek in audio track: {error}")),
        }
    }

    /// Pause playback or resume it when already paused.
    pub fn toggle_pause(&mut self) {
        let Some(player) = &self.player else {
            return;
        };
        if self.paused {
            player.play();
            self.paused = false;
        } else {
            self.position = player.get_pos();
            player.pause();
            self.paused = true;
        }
    }

    /// Replace the current queue with one track and begin playing it.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks.
    /// * `track_index` - Index of the track to play.
    pub fn play(&mut self, tracks: &[Track], track_index: usize) {
        self.stop();
        self.error = None;
        self.start_track(tracks, track_index);
    }

    /// Play an album from its first track and queue the remaining tracks.
    ///
    /// # Arguments
    /// * `album` - Album whose track indexes define playback order.
    /// * `tracks` - Current library tracks.
    pub fn play_album(&mut self, album: &Album, tracks: &[Track]) {
        let Some((&first, rest)) = album.tracks.split_first() else {
            return;
        };
        self.stop();
        self.error = None;
        self.queue.extend(rest.iter().copied());
        self.start_track(tracks, first);
    }

    /// Play a track and queue the following tracks from its album.
    ///
    /// # Arguments
    /// * `album` - Album containing the selected track.
    /// * `tracks` - Current library tracks.
    /// * `track_index` - Index of the selected track in `tracks`.
    pub fn play_track_in_album(&mut self, album: &Album, tracks: &[Track], track_index: usize) {
        let Some(position) = album.tracks.iter().position(|&index| index == track_index) else {
            return;
        };
        self.stop();
        self.error = None;
        self.queue
            .extend(album.tracks[position + 1..].iter().copied());
        self.start_track(tracks, track_index);
    }

    /// Start the next queued track when the active track has ended.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks.
    pub fn advance_if_finished(&mut self, tracks: &[Track]) {
        let finished = self
            .player
            .as_ref()
            .is_some_and(|player| player.empty() && !self.paused);
        if !finished {
            return;
        }

        let finished_track = self.current;
        self.player = None;
        self.current = None;
        self.position = Duration::ZERO;
        self.paused = false;
        if let Some(next) = self.queue.pop_front() {
            if let Some(track) = finished_track {
                self.push_history(track);
            }
            self.start_track(tracks, next);
        }
    }

    /// Skip the current track and start the next queued track, if one exists.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks.
    pub fn skip_next(&mut self, tracks: &[Track]) {
        if let Some(next) = self.queue.pop_front() {
            if let Some(track) = self.current {
                self.push_history(track);
            }
            self.stop_current();
            self.start_track(tracks, next);
        }
    }

    /// Return to the most recently played track, or restart the current track.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks.
    pub fn previous(&mut self, tracks: &[Track]) {
        let Some(previous) = self.history.pop_back() else {
            if let Some(current) = self.current {
                self.stop_current();
                self.start_track(tracks, current);
            }
            return;
        };
        if let Some(current) = self.current {
            self.queue.push_front(current);
        }
        self.stop_current();
        self.start_track(tracks, previous);
    }

    /// Stop the active track and clear the queue and playback history.
    pub fn stop(&mut self) {
        self.stop_current();
        self.queue.clear();
        self.history.clear();
    }

    /// Update stored track indexes after the library has been changed.
    ///
    /// # Arguments
    /// * `remap` - Old-to-new track index mapping; `None` marks a removed track.
    pub fn remap_tracks(&mut self, remap: &[Option<usize>]) {
        if let Some(current) = self.current {
            match remap.get(current).copied().flatten() {
                Some(index) => self.current = Some(index),
                None => {
                    self.stop();
                    return;
                }
            }
        }
        self.queue = self
            .queue
            .drain(..)
            .filter_map(|index| remap.get(index).copied().flatten())
            .collect();
        self.history = self
            .history
            .drain(..)
            .filter_map(|index| remap.get(index).copied().flatten())
            .collect();
    }

    fn start_track(&mut self, tracks: &[Track], track_index: usize) {
        self.start_track_at(tracks, track_index, Duration::ZERO);
    }

    fn start_track_at(&mut self, tracks: &[Track], track_index: usize, position: Duration) {
        let Some(track) = tracks.get(track_index) else {
            self.current = None;
            return;
        };
        let result = self.open_track(&track.path);
        match result {
            Ok(player) => {
                player.set_volume(f32::from(self.volume.min(100)) / 100.0);
                if !position.is_zero() {
                    if let Err(error) = player.try_seek(position) {
                        self.error = Some(format!("Could not seek in audio track: {error}"));
                    }
                }
                self.player = Some(player);
                self.current = Some(track_index);
                self.position = position;
                self.paused = false;
            }
            Err(error) => {
                self.player = None;
                self.current = None;
                self.position = Duration::ZERO;
                self.paused = false;
                self.queue.clear();
                self.error = Some(error);
            }
        }
    }

    fn open_track(&mut self, path: &Path) -> Result<Player, String> {
        if self.output.is_none() {
            let mut output = rodio::DeviceSinkBuilder::open_default_sink()
                .map_err(|error| format!("Could not open audio output: {error}"))?;
            output.log_on_drop(false);
            self.output = Some(output);
        }

        #[cfg(target_os = "linux")]
        let staged_path = if crate::features::library::smb::is_smb_path(path) {
            let auth = self.smb_auth.as_ref().ok_or_else(|| {
                "Reconnect to the SMB share in Preferences to play this track".to_owned()
            })?;
            Some(
                crate::features::library::smb::stage_file(path, auth)
                    .map_err(|error| error.to_string())
                    .map_err(|error| format!("Could not read SMB audio file: {error}"))?,
            )
        } else {
            None
        };
        #[cfg(target_os = "linux")]
        let local_path = staged_path.as_deref().unwrap_or(path);
        #[cfg(not(target_os = "linux"))]
        let local_path = path;
        let cleanup_staged = || {
            #[cfg(target_os = "linux")]
            if let Some(staged_path) = staged_path.as_ref() {
                let parent = staged_path.parent().map(std::path::Path::to_owned);
                let _ = std::fs::remove_file(staged_path);
                if let Some(parent) = parent {
                    let _ = std::fs::remove_dir(parent);
                }
            }
        };
        let file = match File::open(local_path) {
            Ok(file) => file,
            Err(error) => {
                cleanup_staged();
                return Err(format!(
                    "Could not open audio file {}: {error}",
                    path.display()
                ));
            }
        };
        let decoder = match Decoder::try_from(file) {
            Ok(decoder) => decoder,
            Err(error) => {
                cleanup_staged();
                return Err(format!(
                    "Could not decode audio file {}: {error}",
                    path.display()
                ));
            }
        };
        cleanup_staged();
        let player = Player::connect_new(
            self.output
                .as_ref()
                .expect("audio output is initialized")
                .mixer(),
        );
        player.append(decoder);
        Ok(player)
    }

    fn stop_current(&mut self) {
        self.player = None;
        self.current = None;
        self.position = Duration::ZERO;
        self.paused = false;
    }

    fn push_history(&mut self, track: usize) {
        self.history.push_back(track);
        if self.history.len() > 100 {
            self.history.pop_front();
        }
    }
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            current: None,
            error: None,
            output: None,
            player: None,
            queue: VecDeque::new(),
            history: VecDeque::new(),
            position: Duration::ZERO,
            paused: false,
            volume: 70,
            #[cfg(target_os = "linux")]
            smb_auth: None,
        }
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.stop();
    }
}
