use crate::library::{Album, Track};
use std::{
    collections::VecDeque,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub struct Playback {
    pub current: Option<usize>,
    pub error: Option<String>,
    player: Option<Child>,
    queue: VecDeque<usize>,
    history: VecDeque<usize>,
    position: Duration,
    started_at: Option<Instant>,
    paused: bool,
    pub volume: u8,
    sink_input: Option<u32>,
    mixer_volume_applied: bool,
    last_sink_lookup: Option<Instant>,
}

impl Playback {
    pub fn set_volume(&mut self, tracks: &[Track], volume: u8) {
        if self.volume == volume {
            if self.current.is_some() && !self.paused && !self.mixer_volume_applied {
                let current = self.current;
                let position = self.position();
                self.kill_player();
                if let Some(index) = current {
                    self.start_track_at(tracks, index, position);
                }
            }
            return;
        }
        let current = self.current;
        let position = self.position();
        let restart = current.is_some() && !self.paused;
        self.volume = volume;
        if restart && !self.apply_stream_volume(volume) {
            self.kill_player();
            if let Some(index) = current {
                self.start_track_at(tracks, index, position);
            }
        } else if restart {
            self.mixer_volume_applied = true;
        }
    }

    pub fn preview_volume(&mut self, _tracks: &[Track], volume: u8) {
        if self.volume == volume && self.mixer_volume_applied {
            return;
        }
        self.volume = volume;
        if self.current.is_some() && !self.paused {
            self.mixer_volume_applied = self.apply_stream_volume(volume);
        }
    }

    fn apply_stream_volume(&mut self, volume: u8) -> bool {
        let sink_input = self.sink_input.or_else(|| self.find_sink_input());
        let Some(sink_input) = sink_input else {
            return false;
        };
        let result = Command::new("pactl")
            .args([
                "set-sink-input-volume",
                &sink_input.to_string(),
                &format!("{volume}%"),
            ])
            .status();
        if matches!(result, Ok(status) if status.success()) {
            self.sink_input = Some(sink_input);
            true
        } else {
            self.sink_input = None;
            false
        }
    }

    fn find_sink_input(&mut self) -> Option<u32> {
        const LOOKUP_INTERVAL: Duration = Duration::from_millis(250);
        if self
            .last_sink_lookup
            .is_some_and(|last| last.elapsed() < LOOKUP_INTERVAL)
        {
            return None;
        }
        self.last_sink_lookup = Some(Instant::now());
        let process_id = self.player.as_ref()?.id().to_string();
        let output = Command::new("pactl")
            .args(["list", "sink-inputs"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let output = String::from_utf8_lossy(&output.stdout);
        output.split("Sink Input #").find_map(|block| {
            let (header, properties) = block.split_once('\n')?;
            let id = header.trim().parse::<u32>().ok()?;
            let has_process_id = properties.lines().any(|line| {
                let Some((key, value)) = line.trim().split_once('=') else {
                    return false;
                };
                key.trim() == "application.process.id"
                    && value.trim().trim_matches('"') == process_id
            });
            has_process_id.then_some(id)
        })
    }

    pub fn is_playing(&self) -> bool {
        self.player.is_some()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn position(&self) -> Duration {
        if self.paused {
            return self.position;
        }
        self.started_at
            .map(|started| self.position.saturating_add(started.elapsed()))
            .unwrap_or(self.position)
    }

    pub fn seek(&mut self, tracks: &[Track], position: Duration) {
        let Some(index) = self.current else {
            return;
        };
        let duration = tracks
            .get(index)
            .and_then(|track| track.duration_ms)
            .map(Duration::from_millis);
        let position = duration.map_or(position, |duration| position.min(duration));
        self.position = position;
        if !self.paused {
            self.kill_player();
            self.start_track_at(tracks, index, position);
        }
    }

    pub fn toggle_pause(&mut self, tracks: &[Track]) {
        let Some(index) = self.current else {
            return;
        };
        if self.paused {
            self.start_track_at(tracks, index, self.position);
        } else {
            self.position = self.position();
            self.kill_player();
            self.started_at = None;
            self.paused = true;
        }
    }

    pub fn play(&mut self, tracks: &[Track], track_index: usize) {
        self.stop();
        self.error = None;
        self.start_track(tracks, track_index);
    }

    pub fn play_album(&mut self, album: &Album, tracks: &[Track]) {
        let Some((&first, rest)) = album.tracks.split_first() else {
            return;
        };
        let rest = rest.to_vec();
        self.stop();
        self.error = None;
        self.queue.extend(rest);
        self.start_track(tracks, first);
    }

    pub fn play_track_in_album(&mut self, album: &Album, tracks: &[Track], track_index: usize) {
        let Some(position) = album.tracks.iter().position(|&index| index == track_index) else {
            return;
        };
        let remaining = album.tracks[position + 1..].to_vec();
        self.stop();
        self.error = None;
        self.queue.extend(remaining);
        self.start_track(tracks, track_index);
    }

    pub fn advance_if_finished(&mut self, tracks: &[Track]) {
        match self.player.as_mut().map(Child::try_wait) {
            Some(Ok(Some(status))) => {
                let finished_track = self.current;
                self.player = None;
                self.current = None;
                self.position = Duration::ZERO;
                self.started_at = None;
                self.paused = false;
                if let Some(next) = self.queue.pop_front() {
                    if let Some(track) = finished_track {
                        self.push_history(track);
                    }
                    self.start_track(tracks, next);
                } else if !status.success() {
                    self.error = Some("ffplay stopped with an error".into());
                }
            }
            Some(Err(error)) => {
                self.stop_current();
                self.queue.clear();
                self.error = Some(format!("Could not check ffplay status: {error}"));
            }
            Some(Ok(None)) | None => {}
        }
    }

    pub fn skip_next(&mut self, tracks: &[Track]) {
        if let Some(next) = self.queue.pop_front() {
            if let Some(track) = self.current {
                self.push_history(track);
            }
            self.stop_current();
            self.start_track(tracks, next);
        }
    }

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

    pub fn stop(&mut self) {
        self.stop_current();
        self.queue.clear();
        self.history.clear();
    }

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
            return;
        };
        let mut command = Command::new("ffplay");
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.arg0("musicplayer");
        }
        let volume = self.volume.to_string();
        command.args([
            "-nodisp",
            "-autoexit",
            "-loglevel",
            "error",
            "-volume",
            &volume,
        ]);
        if !position.is_zero() {
            command.args(["-ss", &format!("{:.3}", position.as_secs_f64())]);
        }
        match command
            .arg(&track.path)
            .env("SDL_APP_NAME", "musicplayer")
            .env("SDL_AUDIO_DEVICE_APP_NAME", "musicplayer")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                #[cfg(target_os = "linux")]
                {
                    // btop and similar process monitors display /proc/<pid>/comm,
                    // which exec resets to "ffplay" after the child starts.
                    let _ = std::fs::write(format!("/proc/{}/comm", child.id()), "musicplayer");
                }
                self.player = Some(child);
                self.sink_input = None;
                self.mixer_volume_applied = true;
                self.last_sink_lookup = None;
                self.current = Some(track_index);
                self.position = position;
                self.started_at = Some(Instant::now());
                self.paused = false;
            }
            Err(error) => {
                self.queue.clear();
                self.error = Some(format!("Could not start ffplay: {error}"));
            }
        }
    }

    fn stop_current(&mut self) {
        self.kill_player();
        self.current = None;
        self.position = Duration::ZERO;
        self.started_at = None;
        self.paused = false;
    }

    fn kill_player(&mut self) {
        self.sink_input = None;
        self.mixer_volume_applied = false;
        self.last_sink_lookup = None;
        if let Some(mut child) = self.player.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
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
            player: None,
            queue: VecDeque::new(),
            history: VecDeque::new(),
            position: Duration::ZERO,
            started_at: None,
            paused: false,
            volume: 70,
            sink_input: None,
            mixer_volume_applied: false,
            last_sink_lookup: None,
        }
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.stop();
    }
}
