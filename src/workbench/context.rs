use crate::{
    features::{
        library::{artwork::ArtworkCache, state::LibraryFeature},
        playback::Playback,
        preferences::PreferencesState,
    },
    platform::persistence::settings::Settings,
    workbench::WorkbenchState,
};

/// Mutable application state exposed to the workbench's view orchestration.
pub(crate) struct WorkbenchContext<'a> {
    pub(crate) state: &'a mut WorkbenchState,
    pub(crate) library: &'a mut LibraryFeature,
    pub(crate) playback: &'a mut Playback,
    pub(crate) settings: &'a mut Settings,
    pub(crate) preferences: &'a mut PreferencesState,
    pub(crate) artwork_cache: &'a mut ArtworkCache,
    pub(crate) error: &'a mut Option<String>,
    pub(crate) dismissed_notice_signature: &'a mut Option<String>,
}

/// User actions that the application layer handles after rendering the workbench.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkbenchAction {
    ChooseMusicFolder,
    SaveSettings,
    ConnectSmbShare,
}
