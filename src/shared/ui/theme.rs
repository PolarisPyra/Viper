//! Theme registry and semantic colors for native widgets and custom-painted views.
//! Add theme choices here; screens should consume Palette roles, not vendor colors.
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeId {
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    #[default]
    CatppuccinMocha,
    RosePine,
    RosePineMoon,
    RosePineDawn,
}

impl ThemeId {
    pub(crate) const ALL: [Self; 7] = [
        Self::CatppuccinLatte,
        Self::CatppuccinFrappe,
        Self::CatppuccinMacchiato,
        Self::CatppuccinMocha,
        Self::RosePine,
        Self::RosePineMoon,
        Self::RosePineDawn,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::CatppuccinFrappe => "Catppuccin Frappé",
            Self::CatppuccinMacchiato => "Catppuccin Macchiato",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::RosePine => "Rosé Pine",
            Self::RosePineMoon => "Rosé Pine Moon",
            Self::RosePineDawn => "Rosé Pine Dawn",
        }
    }

    fn definition(self) -> ThemeDefinition {
        match self {
            Self::CatppuccinLatte => catppuccin::definition(catppuccin_egui::LATTE),
            Self::CatppuccinFrappe => catppuccin::definition(catppuccin_egui::FRAPPE),
            Self::CatppuccinMacchiato => catppuccin::definition(catppuccin_egui::MACCHIATO),
            Self::CatppuccinMocha => catppuccin::definition(catppuccin_egui::MOCHA),
            Self::RosePine => rose_pine::definition(rose_pine::Variant::Main),
            Self::RosePineMoon => rose_pine::definition(rose_pine::Variant::Moon),
            Self::RosePineDawn => rose_pine::definition(rose_pine::Variant::Dawn),
        }
    }
}

mod catppuccin;
mod rose_pine;

/// Each provider supplies native egui visuals and custom-paint semantic colors.
struct ThemeDefinition {
    visuals: egui::Visuals,
    palette: Palette,
}

#[derive(Clone, Copy)]
struct AppliedTheme {
    id: ThemeId,
    palette: Palette,
}

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub canvas: egui::Color32,
    pub panel: egui::Color32,
    pub sidebar: egui::Color32,
    pub input: egui::Color32,
    pub surface: egui::Color32,
    pub hover: egui::Color32,
    pub selected: egui::Color32,
    pub border: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub subtle: egui::Color32,
    pub accent: egui::Color32,
    pub secondary: egui::Color32,
    pub on_accent: egui::Color32,
    pub warning: egui::Color32,
    pub danger: egui::Color32,
    pub backdrop: egui::Color32,
}

pub(crate) fn colors(ctx: &egui::Context) -> Palette {
    ctx.data(|data| data.get_temp::<AppliedTheme>(egui::Id::new("app-theme")))
        .map(|theme| theme.palette)
        .unwrap_or_else(|| ThemeId::default().definition().palette)
}

pub(crate) fn apply(ctx: &egui::Context, theme: ThemeId) {
    let id = egui::Id::new("app-theme");
    if ctx
        .data(|data| data.get_temp::<AppliedTheme>(id))
        .map(|theme| theme.id)
        == Some(theme)
    {
        return;
    }
    let ThemeDefinition {
        mut visuals,
        palette,
    } = theme.definition();
    visuals.selection.bg_fill = palette.selected;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, palette.text);
    visuals.hyperlink_color = palette.secondary;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.danger;
    ctx.set_visuals(visuals);
    ctx.data_mut(|data| data.insert_temp(id, AppliedTheme { id: theme, palette }));
    ctx.request_repaint();
}

pub(crate) fn style_dropdown(ui: &mut egui::Ui) {
    let colors = colors(ui.ctx());
    let border = egui::Stroke::new(1.0_f32, colors.border);
    let visuals = ui.visuals_mut();
    visuals.window_fill = colors.panel;
    visuals.window_stroke = border;
    visuals.menu_corner_radius = egui::CornerRadius::same(6);
    visuals.selection.bg_fill = colors.selected;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, colors.text);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = colors.panel;
        widget.weak_bg_fill = colors.panel;
        widget.bg_stroke = border;
    }
    visuals.widgets.hovered.bg_fill = colors.hover;
    visuals.widgets.hovered.weak_bg_fill = colors.hover;
}
