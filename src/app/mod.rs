//! Native application state and startup entry point.
mod application;
mod bootstrap;

pub use application::ViperApp;
pub use bootstrap::run;
