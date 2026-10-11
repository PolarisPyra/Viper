use super::client::{self, Lyrics};
use crate::features::library::Track;
use eframe::egui;
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

#[derive(Clone, Debug)]
/// Current outcome of a lyrics lookup for the selected track.
pub(crate) enum LyricsResult {
    /// A lyrics request is running.
    Loading,
    /// Lyrics were found.
    Found(Lyrics),
    /// The service has no lyrics for the track.
    Unavailable,
    /// The lyrics service request failed.
    Failed(String),
}

#[derive(Default)]
/// Asynchronous lyrics requests and their session cache.
pub(crate) struct LyricsFeature {
    cache: HashMap<String, Result<Option<Lyrics>, String>>,
    pending: Option<(String, Receiver<Result<Option<Lyrics>, String>>)>,
}

impl LyricsFeature {
    /// Return cached lyrics or start a background lookup for `track`.
    ///
    /// # Arguments
    /// * `ctx` - egui context used to schedule redraws after the request completes.
    /// * `track` - Track metadata used to query the lyrics provider.
    ///
    /// # Returns
    /// The cached result, or the current loading state.
    pub(crate) fn request(&mut self, ctx: &egui::Context, track: &Track) -> LyricsResult {
        let key = track_key(track);
        if let Some((finished_key, receiver)) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.cache.insert(finished_key.clone(), result);
                    self.pending = None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.cache.insert(
                        finished_key.clone(),
                        Err("Lyrics request stopped".to_owned()),
                    );
                    self.pending = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(result) = self.cache.get(&key) {
            return match result {
                Ok(Some(lyrics)) => LyricsResult::Found(lyrics.clone()),
                Ok(None) => LyricsResult::Unavailable,
                Err(error) => LyricsResult::Failed(error.clone()),
            };
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|(pending_key, _)| pending_key == &key)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
            return LyricsResult::Loading;
        }
        if self.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return LyricsResult::Loading;
        }
        let (sender, receiver) = mpsc::channel();
        let track = track.clone();
        let request_ctx = ctx.clone();
        let request_key = key.clone();
        if let Err(error) = thread::Builder::new()
            .name("viper-lyrics-fetch".to_owned())
            .spawn(move || {
                let _ = sender.send(client::fetch(&track));
                request_ctx.request_repaint();
            })
        {
            return LyricsResult::Failed(format!("Could not start lyrics request: {error}"));
        }
        self.pending = Some((request_key, receiver));
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
        LyricsResult::Loading
    }
}

fn track_key(track: &Track) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}",
        track.path.to_string_lossy(),
        track.title,
        track.artist,
        track.album,
        track.duration_ms.unwrap_or_default()
    )
}
