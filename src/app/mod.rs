mod discord_presence;
mod initialization;
mod shell;

pub use initialization::run;
pub(crate) use shell::Page;
pub use shell::ViperApp;
