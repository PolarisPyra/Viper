use crate::features::library::{Album, Track};
use rodio::{Decoder, MixerDeviceSink, Player};
use std::{collections::VecDeque, fs::File, io::BufReader, path::Path, time::Duration};

/// Repeat behavior for the current track or playback context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RepeatMode {
    /// Stop when the queue ends.
    #[default]
    Off,
    /// Repeat the current album or library session.
    All,
    /// Repeat the current track.
    One,
}

/// Current audio playback state, queue, history, and volume.
pub struct Playback {
    /// Index of the currently playing track in the supplied library, if any.
    pub current: Option<usize>,
    /// Most recent playback or seek error shown to the user.
    pub error: Option<String>,
    output: Option<MixerDeviceSink>,
    player: Option<Player>,
    manual_queue: VecDeque<usize>,
    auto_queue: VecDeque<usize>,
    history: VecDeque<usize>,
    context_tracks: Vec<usize>,
    position: Duration,
    paused: bool,
    gapless_active: bool,
    gapless_track: Option<usize>,
    gapless_from_manual_queue: bool,
    gapless_preempt: bool,
    gapless_skip_current: bool,
    source_count: usize,
    /// Whether upcoming tracks are shuffled.
    pub(crate) shuffle_enabled: bool,
    /// Repeat mode for the current song or listening context.
    pub(crate) repeat_mode: RepeatMode,
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
        self.prepare_selection();
        self.context_tracks.clear();
        self.context_tracks.push(track_index);
        self.error = None;
        self.start_track(tracks, track_index);
    }

    /// Start a random library track and shuffle the remaining tracks.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks.
    pub fn play_random_track(&mut self, tracks: &[Track]) {
        if tracks.is_empty() {
            return;
        }
        self.prepare_selection();
        self.error = None;
        self.context_tracks = (0..tracks.len()).collect();
        self.auto_queue.extend(0..tracks.len());
        self.shuffle_queue();
        if let Some(track_index) = self.auto_queue.pop_front() {
            self.start_track(tracks, track_index);
        }
    }

    /// Play an album and arrange its remaining tracks according to shuffle settings.
    ///
    /// # Arguments
    /// * `album` - Album whose track indexes define playback order.
    /// * `tracks` - Current library tracks.
    pub fn play_album(&mut self, album: &Album, tracks: &[Track]) {
        let Some((&first, rest)) = album.tracks.split_first() else {
            return;
        };
        self.prepare_selection();
        self.error = None;
        self.context_tracks = album.tracks.clone();
        let first = if self.shuffle_enabled {
            self.auto_queue.extend(album.tracks.iter().copied());
            self.auto_queue.retain(|&index| index != first);
            self.shuffle_queue();
            self.auto_queue.pop_front().unwrap_or(first)
        } else {
            self.auto_queue.extend(rest.iter().copied());
            first
        };
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
        self.prepare_selection();
        self.error = None;
        self.context_tracks = album.tracks.clone();
        if self.shuffle_enabled {
            self.auto_queue.extend(
                album
                    .tracks
                    .iter()
                    .copied()
                    .filter(|&index| index != track_index),
            );
        } else {
            self.auto_queue
                .extend(album.tracks[position + 1..].iter().copied());
        }
        if self.shuffle_enabled {
            self.shuffle_queue();
        }
        self.start_track(tracks, track_index);
    }

    /// Start the next queued track when the active track has ended.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks.
    pub fn advance_if_finished(&mut self, tracks: &[Track]) {
        if self.gapless_active {
            let source_count = self.player.as_ref().map_or(0, Player::len);
            let completed = self.source_count.saturating_sub(source_count);
            if completed > 0 {
                if let Some(finished_track) = self.current {
                    if self.gapless_skip_current {
                        self.gapless_skip_current = false;
                    } else {
                        self.push_history(finished_track);
                    }
                }
                self.current = if let Some(track_index) = self.gapless_track.take() {
                    if self.gapless_from_manual_queue {
                        self.manual_queue.pop_front();
                    } else {
                        self.auto_queue.pop_front();
                    }
                    self.gapless_from_manual_queue = false;
                    Some(track_index)
                } else {
                    self.pop_next_queued_track()
                };
                self.position = Duration::ZERO;
            }
            self.source_count = source_count;
            if source_count == 0 {
                self.player = None;
                self.position = Duration::ZERO;
                self.paused = false;
                self.gapless_active = false;
                if let Some(finished_track) = self.current.take() {
                    self.push_history(finished_track);
                }
                self.refill_repeating_queue();
                if let Some(next) = self.pop_next_queued_track() {
                    self.start_track(tracks, next);
                }
            } else {
                self.gapless_active = source_count > 1;
            }
            return;
        }
        let finished = self
            .player
            .as_ref()
            .is_some_and(|player| player.empty() && !self.paused);
        if !finished {
            return;
        }

        let finished_track = self.current;
        if self.repeat_mode == RepeatMode::One {
            if let Some(track) = finished_track {
                self.start_track(tracks, track);
            }
            return;
        }
        self.player = None;
        self.current = None;
        self.position = Duration::ZERO;
        self.paused = false;
        if let Some(track) = finished_track {
            self.push_history(track);
        }
        self.refill_repeating_queue();
        if let Some(next) = self.pop_next_queued_track() {
            self.start_track(tracks, next);
        }
    }

    /// Append queued tracks to the active player for gapless transitions.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks indexed by the playback queue.
    /// * `enabled` - Whether gapless transitions should be active.
    pub fn update_gapless(&mut self, tracks: &[Track], enabled: bool) {
        if !enabled {
            if self.gapless_active {
                self.rebuild_current(tracks);
            }
            return;
        }
        if self.gapless_active || self.current.is_none() {
            return;
        }
        let Some(player) = self.player.as_ref() else {
            return;
        };
        if self.gapless_preempt {
            let Some(track_index) = self.manual_queue.front().copied() else {
                self.gapless_preempt = false;
                return;
            };
            let Some(track) = tracks.get(track_index) else {
                self.manual_queue.pop_front();
                self.gapless_preempt = false;
                return;
            };
            match self.open_decoder(&track.path) {
                Ok(decoder) => {
                    player.append(decoder);
                    // The prior track has ended; the preloaded automatic track is now active.
                    player.skip_one();
                    self.source_count = player.len();
                    self.gapless_active = self.source_count > 1;
                    self.gapless_track = Some(track_index);
                    self.gapless_from_manual_queue = true;
                    self.gapless_preempt = false;
                    self.gapless_skip_current = true;
                }
                Err(error) => {
                    self.manual_queue.pop_front();
                    self.gapless_preempt = false;
                    self.error = Some(error);
                }
            }
            return;
        }
        let Some(track_index) = self.next_queued_track() else {
            return;
        };
        let Some(track) = tracks.get(track_index) else {
            self.discard_next_queued_track();
            return;
        };
        match self.open_decoder(&track.path) {
            Ok(decoder) => player.append(decoder),
            Err(error) => {
                self.discard_next_queued_track();
                self.error = Some(error);
                return;
            }
        }
        self.source_count = player.len();
        self.gapless_active = self.source_count > 1;
        self.gapless_track = self.gapless_active.then_some(track_index);
        self.gapless_from_manual_queue = self.gapless_active && !self.manual_queue.is_empty();
    }

    /// Return manually queued tracks, excluding automatic album and shuffle tracks.
    pub(crate) fn queue(&self) -> &VecDeque<usize> {
        &self.manual_queue
    }

    /// Return whether Previous can move to a track from playback history.
    pub(crate) fn has_history(&self) -> bool {
        !self.history.is_empty()
    }

    /// Return whether Next can play a queued song or repeat the current context.
    pub(crate) fn has_next(&self) -> bool {
        !self.manual_queue.is_empty()
            || !self.auto_queue.is_empty()
            || (self.repeat_mode == RepeatMode::All && !self.context_tracks.is_empty())
    }

    fn pop_next_queued_track(&mut self) -> Option<usize> {
        self.manual_queue
            .pop_front()
            .or_else(|| self.auto_queue.pop_front())
    }

    fn next_queued_track(&self) -> Option<usize> {
        self.manual_queue
            .front()
            .or_else(|| self.auto_queue.front())
            .copied()
    }

    fn discard_next_queued_track(&mut self) {
        if self.manual_queue.is_empty() {
            self.auto_queue.pop_front();
        } else {
            self.manual_queue.pop_front();
        }
    }

    /// Toggle shuffle mode and reorder the tracks waiting in the queue.
    pub(crate) fn toggle_shuffle(&mut self, tracks: &[Track]) {
        self.shuffle_enabled = !self.shuffle_enabled;
        if self.shuffle_enabled {
            self.auto_queue.clear();
            if self.context_tracks.is_empty() {
                self.context_tracks.extend(0..tracks.len());
            }
            self.auto_queue.extend(
                self.context_tracks
                    .iter()
                    .copied()
                    .filter(|&index| Some(index) != self.current),
            );
            self.shuffle_queue();
            if self.gapless_active {
                self.rebuild_gapless(tracks);
            }
        } else {
            self.auto_queue.clear();
            if !self.context_tracks.is_empty() {
                if let Some(current) = self.current {
                    if let Some(position) = self.context_tracks.iter().position(|&i| i == current) {
                        self.auto_queue
                            .extend(self.context_tracks[position + 1..].iter().copied());
                    }
                }
            }
            if self.gapless_active {
                self.rebuild_gapless(tracks);
            }
        }
    }

    /// Cycle repeat behavior through off, repeat context, and repeat track.
    pub(crate) fn cycle_repeat_mode(&mut self, tracks: &[Track]) {
        self.repeat_mode = match self.repeat_mode {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        if self.repeat_mode != RepeatMode::Off && self.gapless_active {
            self.rebuild_current(tracks);
        }
    }

    fn shuffle_queue(&mut self) {
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos() as u64);
        for index in (1..self.auto_queue.len()).rev() {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.auto_queue.swap(index, (seed as usize) % (index + 1));
        }
    }

    /// Add a track to the end of the playback queue.
    pub(crate) fn enqueue(&mut self, track_index: usize) {
        self.manual_queue.push_back(track_index);
        if self.gapless_active && self.gapless_track.is_some() && !self.gapless_from_manual_queue {
            self.gapless_preempt = true;
        }
    }

    /// Remove a queued track and return it when the index exists.
    pub(crate) fn remove_queued(&mut self, queue_index: usize, tracks: &[Track]) -> Option<usize> {
        let removed = self.manual_queue.remove(queue_index);
        if removed.is_some() && self.gapless_active {
            self.rebuild_gapless(tracks);
        }
        removed
    }

    /// Move a queued track to a new position.
    pub(crate) fn move_queued(&mut self, from: usize, to: usize, tracks: &[Track]) {
        if from >= self.manual_queue.len() || to >= self.manual_queue.len() || from == to {
            return;
        }
        if let Some(track) = self.manual_queue.remove(from) {
            self.manual_queue.insert(to, track);
            if self.gapless_active {
                self.rebuild_gapless(tracks);
            }
        }
    }

    fn rebuild_gapless(&mut self, tracks: &[Track]) {
        self.rebuild_current(tracks);
        self.update_gapless(tracks, true);
    }

    fn rebuild_current(&mut self, tracks: &[Track]) {
        let (Some(current), Some(player)) = (self.current, self.player.take()) else {
            self.gapless_active = false;
            return;
        };
        let position = player.get_pos();
        let paused = self.paused;
        player.clear();
        self.gapless_active = false;
        self.gapless_track = None;
        self.gapless_from_manual_queue = false;
        self.gapless_preempt = false;
        self.gapless_skip_current = false;
        self.start_track_at(tracks, current, position);
        if paused {
            if let Some(player) = &self.player {
                player.pause();
                self.paused = true;
                self.position = position;
            }
        }
    }

    /// Skip the current track and start the next queued track, if one exists.
    ///
    /// # Arguments
    /// * `tracks` - Current library tracks.
    pub fn skip_next(&mut self, tracks: &[Track]) {
        self.refill_repeating_queue();
        if let Some(next) = self.pop_next_queued_track() {
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
        if let Some(current) = self.current {
            if self.position() > Duration::from_secs(3) {
                self.stop_current();
                self.start_track(tracks, current);
                return;
            }
        }
        let Some(previous) = self.history.pop_back() else {
            if let Some(current) = self.current {
                self.stop_current();
                self.start_track(tracks, current);
            }
            return;
        };
        if let Some(current) = self.current {
            self.auto_queue.push_front(current);
        }
        self.stop_current();
        self.start_track(tracks, previous);
    }

    /// Stop the active track and clear the queue and playback history.
    pub fn stop(&mut self) {
        self.stop_current();
        self.manual_queue.clear();
        self.auto_queue.clear();
        self.history.clear();
        self.context_tracks.clear();
    }

    fn prepare_selection(&mut self) {
        if let Some(current) = self.current {
            self.push_history(current);
        }
        self.stop_current();
        self.manual_queue.clear();
        self.auto_queue.clear();
        self.error = None;
    }

    fn refill_repeating_queue(&mut self) {
        if self.repeat_mode != RepeatMode::All || !self.auto_queue.is_empty() {
            return;
        }
        self.auto_queue.extend(self.context_tracks.iter().copied());
        if self.shuffle_enabled {
            self.shuffle_queue();
        }
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
        self.manual_queue = self
            .manual_queue
            .drain(..)
            .filter_map(|index| remap.get(index).copied().flatten())
            .collect();
        self.auto_queue = self
            .auto_queue
            .drain(..)
            .filter_map(|index| remap.get(index).copied().flatten())
            .collect();
        self.history = self
            .history
            .drain(..)
            .filter_map(|index| remap.get(index).copied().flatten())
            .collect();
        self.context_tracks = self
            .context_tracks
            .iter()
            .filter_map(|&index| remap.get(index).copied().flatten())
            .collect();
        self.gapless_track = self
            .gapless_track
            .and_then(|index| remap.get(index).copied().flatten());
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
                self.source_count = 1;
                self.gapless_active = false;
                self.gapless_track = None;
                self.gapless_from_manual_queue = false;
                self.gapless_preempt = false;
                self.gapless_skip_current = false;
                self.current = Some(track_index);
                self.position = position;
                self.paused = false;
            }
            Err(error) => {
                self.player = None;
                self.current = None;
                self.position = Duration::ZERO;
                self.paused = false;
                self.manual_queue.clear();
                self.auto_queue.clear();
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

        let decoder = self.open_decoder(path)?;
        let player = Player::connect_new(
            self.output
                .as_ref()
                .expect("audio output is initialized")
                .mixer(),
        );
        player.append(decoder);
        Ok(player)
    }

    fn open_decoder(&self, path: &Path) -> Result<Decoder<BufReader<File>>, String> {
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
        Ok(decoder)
    }

    fn stop_current(&mut self) {
        self.player = None;
        self.current = None;
        self.position = Duration::ZERO;
        self.paused = false;
        self.source_count = 0;
        self.gapless_active = false;
        self.gapless_track = None;
        self.gapless_from_manual_queue = false;
        self.gapless_preempt = false;
        self.gapless_skip_current = false;
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
            manual_queue: VecDeque::new(),
            auto_queue: VecDeque::new(),
            history: VecDeque::new(),
            position: Duration::ZERO,
            paused: false,
            gapless_active: false,
            gapless_track: None,
            gapless_from_manual_queue: false,
            gapless_preempt: false,
            gapless_skip_current: false,
            source_count: 0,
            shuffle_enabled: false,
            context_tracks: Vec::new(),
            repeat_mode: RepeatMode::Off,
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
