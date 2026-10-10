mod content_host;
mod context;
mod shell;
mod state;

pub(crate) use context::{WorkbenchAction, WorkbenchContext};
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
