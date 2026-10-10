//! Preferences UI and state.
mod screen;
#[path = "state.rs"]
mod state;

pub use screen::Category;
pub(crate) use screen::{show, PreferencesAction};
pub(crate) use state::PreferencesState;
