pub mod app;
#[path = "lib/caching.rs"]
pub mod caching;
pub mod metadata;
pub mod playback;
pub mod storage;
pub mod views;
#[path = "lib/watcher.rs"]
pub mod watcher;

const APP_ICON_PNG: &[u8] = include_bytes!("../assets/logo.png");

fn load_system_fallback_fonts() -> eframe::egui::FontDefinitions {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};
    use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
    use std::sync::Arc;

    let mut definitions = FontDefinitions::default();
    let source = SystemSource::new();
    let fallback_families: &[&[&str]] = &[
        &[
            "Noto Sans CJK JP",
            "Noto Sans CJK SC",
            "Noto Sans CJK TC",
            "Noto Sans CJK KR",
            "Yu Gothic",
            "Meiryo",
            "PingFang SC",
            "Malgun Gothic",
        ],
        &["Noto Sans Arabic", "Noto Naskh Arabic", "Geeza Pro"],
        &["Noto Sans Devanagari", "Nirmala UI", "Kohinoor Devanagari"],
        &["Noto Sans Bengali", "Nirmala UI"],
        &["Noto Sans Thai", "Leelawadee UI"],
        &["Noto Sans Hebrew", "Arial Hebrew"],
        &[
            "Noto Sans Tamil",
            "Noto Sans Gujarati",
            "Noto Sans Telugu",
            "Noto Sans Malayalam",
            "Noto Sans Khmer",
            "Noto Sans Ethiopic",
        ],
    ];

    for (index, candidates) in fallback_families.iter().enumerate() {
        let families: Vec<_> = candidates
            .iter()
            .map(|family| FamilyName::Title((*family).to_owned()))
            .collect();
        let Ok(handle) = source.select_best_match(&families, &Properties::new()) else {
            continue;
        };
        let Ok(font) = handle.load() else {
            continue;
        };
        let Some(bytes) = font.copy_font_data() else {
            continue;
        };
        drop(font);
        let bytes = Arc::try_unwrap(bytes).unwrap_or_else(|shared| shared.as_ref().clone());
        let name = format!("system-fallback-{index}");
        definitions
            .font_data
            .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            definitions
                .families
                .entry(family)
                .or_default()
                .push(name.clone());
        }
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
            cc.egui_ctx.set_fonts(load_system_fallback_fonts());
            Ok(Box::new(app::MusicApp::new()))
        }),
    )
}
