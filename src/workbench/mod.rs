mod content_host;
mod context;
mod shell;
mod state;

pub(crate) use context::{WorkbenchAction, WorkbenchContext};
pub(crate) use shell::menu_bar::style_menu_popup;
pub(crate) use state::{Page, WorkbenchState};

use eframe::egui;

pub(crate) fn show(
    ctx: &egui::Context,
    context: &mut WorkbenchContext<'_>,
) -> Vec<WorkbenchAction> {
    let mut actions = Vec::with_capacity(2);
    if shell::menu_bar::show(ctx, context.state, context.library, context.playback)
        == Some(shell::menu_bar::MenuBarAction::ChooseMusicFolder)
    {
        actions.push(WorkbenchAction::ChooseMusicFolder);
    }
    shell::player_bar::show(
        ctx,
        context.state,
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
        context.library,
        context.lastfm_service.status(),
        context.lastfm_service.is_connected(context.lastfm_settings),
        context.lastfm_settings.username.as_deref(),
        context.lastfm_service.can_finish_auth(),
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
            crate::features::preferences::PreferencesAction::SaveLastFmSettings => {
                actions.push(WorkbenchAction::SaveLastFmSettings)
            }
            crate::features::preferences::PreferencesAction::StartLastFmAuth => {
                actions.push(WorkbenchAction::StartLastFmAuth)
            }
            crate::features::preferences::PreferencesAction::FinishLastFmAuth => {
                actions.push(WorkbenchAction::FinishLastFmAuth)
            }
            crate::features::preferences::PreferencesAction::DisconnectLastFm => {
                actions.push(WorkbenchAction::DisconnectLastFm)
            }
        }
    }
    actions
}
