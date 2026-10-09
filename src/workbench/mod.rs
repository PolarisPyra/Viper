mod content_host;
mod shell;
mod state;

pub(crate) use state::{Page, WorkbenchState};

use crate::{
    features::{
        library::{artwork::ArtworkCache, state::LibraryFeature},
        playback::Playback,
        preferences::PreferencesState,
    },
    platform::persistence::settings::Settings,
};
use eframe::egui;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkbenchAction {
    ChooseMusicFolder,
    SaveSettings,
    ConnectSmbShare,
}

pub(crate) fn show(
    ctx: &egui::Context,
    context: &mut WorkbenchContext<'_>,
) -> Vec<WorkbenchAction> {
    let mut actions = Vec::new();
    if shell::menu_bar::show(ctx, context.state, context.library, context.playback)
        == Some(shell::menu_bar::MenuBarAction::ChooseMusicFolder)
    {
        actions.push(WorkbenchAction::ChooseMusicFolder);
    }
    shell::view_header::show(
        ctx,
        context.state,
        context.library,
        context.error,
        context.dismissed_notice_signature,
    );
    shell::player_bar::show(
        ctx,
        context.playback,
        context.library,
        context.settings,
        context.artwork_cache,
        context.error,
    );
    if let Some(error) = shell::sidebar::show(ctx, context.settings, context.state) {
        *context.error = Some(error);
    }
    if let Some(WorkbenchAction::ChooseMusicFolder) = content_host::show(ctx, context) {
        actions.push(WorkbenchAction::ChooseMusicFolder);
    }
    if let Some(action) = crate::features::preferences::show(
        ctx,
        context.state,
        context.preferences,
        context.settings,
        context.error,
    ) {
        match action {
            crate::features::preferences::PreferencesAction::ChooseMusicFolder => {
                actions.push(WorkbenchAction::ChooseMusicFolder)
            }
            crate::features::preferences::PreferencesAction::SaveSettings => {
                actions.push(WorkbenchAction::SaveSettings)
            }
            crate::features::preferences::PreferencesAction::ConnectSmbShare => {
                actions.push(WorkbenchAction::ConnectSmbShare)
            }
        }
    }
    actions
}
