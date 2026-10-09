use super::{Page, WorkbenchAction, WorkbenchContext};
use crate::features;
use eframe::egui;

pub(crate) fn show(
    ctx: &egui::Context,
    context: &mut WorkbenchContext<'_>,
) -> Option<WorkbenchAction> {
    match context.state.page {
        Page::Home => features::home::show(
            ctx,
            context.state,
            context.library,
            context.playback,
            context.settings,
            context.artwork_cache,
        )
        .map(|action| match action {
            features::home::HomeAction::ChooseMusicFolder => WorkbenchAction::ChooseMusicFolder,
        }),
        Page::Albums => {
            if let Some(error) = features::albums::track_list::show(
                ctx,
                context.state,
                context.library,
                context.playback,
                context.settings,
                context.artwork_cache,
            ) {
                *context.error = Some(error);
            }
            features::albums::show(
                ctx,
                features::albums::AlbumScreen {
                    workbench: context.state,
                    library: context.library,
                    playback: context.playback,
                    settings: context.settings,
                    artwork_cache: context.artwork_cache,
                    error: context.error,
                },
            );
            None
        }
    }
}
