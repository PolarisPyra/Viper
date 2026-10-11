//! Automatic lyrics lookup and in-memory results for the lyrics panel.
mod client;
pub(crate) mod panel;
mod state;

pub(crate) use client::Lyrics;
pub(crate) use state::{LyricsFeature, LyricsResult};
