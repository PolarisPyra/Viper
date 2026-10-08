pub mod app;
pub mod artwork;
pub mod components;
pub mod library;
pub mod playback;
pub mod storage;
pub mod views;

const APP_ICON_PNG: &[u8] = include_bytes!("../assets/logo.png");

fn load_app_fonts() -> eframe::egui::FontDefinitions {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};
    use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
    use std::sync::Arc;

    let mut definitions = FontDefinitions::default();
    definitions.font_data.insert(
        "inter".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/InterVariable.ttf"
        ))),
    );
    definitions
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".to_owned());

    let families = [
        "Noto Sans CJK JP",
        "Noto Sans JP",
        "Yu Gothic",
        "Yu Gothic UI",
        "Meiryo",
        "Hiragino Sans",
        "Hiragino Kaku Gothic ProN",
    ]
    .iter()
    .map(|family| FamilyName::Title((*family).to_owned()))
    .collect::<Vec<_>>();
    let Ok(handle) = SystemSource::new().select_best_match(&families, &Properties::new()) else {
        return definitions;
    };
    let Ok(font) = handle.load() else {
        return definitions;
    };
    let Some(bytes) = font.copy_font_data() else {
        return definitions;
    };
    drop(font);
    let bytes = Arc::try_unwrap(bytes).unwrap_or_else(|shared| shared.as_ref().clone());
    definitions.font_data.insert(
        "japanese-fallback".to_owned(),
        Arc::new(FontData::from_owned(bytes)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        definitions
            .families
            .entry(family)
            .or_default()
            .push("japanese-fallback".to_owned());
    }
    definitions
}

pub fn run() -> eframe::Result<()> {
    let icon = image::load_from_memory(APP_ICON_PNG)
        .expect("failed to load assets/logo.png")
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    let (icon_width, icon_height) = icon.dimensions();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_app_id("musicplayer")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([760.0, 520.0])
            .with_icon(eframe::egui::IconData {
                rgba: icon.into_raw(),
                width: icon_width,
                height: icon_height,
            }),
        ..Default::default()
    };
    eframe::run_native(
        "musicplayer",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_fonts(load_app_fonts());
            Ok(Box::new(app::MusicApp::new()))
        }),
    )
}
