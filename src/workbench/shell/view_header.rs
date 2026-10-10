use crate::{
    features::library::state::LibraryFeature,
    workbench::{Page, WorkbenchState},
};
use eframe::egui;

pub fn show(
    ctx: &egui::Context,
    workbench: &WorkbenchState,
    library: &mut LibraryFeature,
    error: &Option<String>,
    dismissed_notice_signature: &mut Option<String>,
) {
    let colors = crate::shared::ui::theme::colors(ctx);
    egui::TopBottomPanel::top("library-topbar")
        .frame(
            egui::Frame::new()
                .fill(colors.canvas)
                .inner_margin(egui::Margin::symmetric(20, 12)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(match workbench.page {
                        Page::Home => "Home",
                        Page::Albums => "Albums",
                    })
                    .size(28.0)
                    .strong()
                    .color(colors.text),
                );
                if library.scanning {
                    let completed = library
                        .progress
                        .completed
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let total = library
                        .progress
                        .total
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let phase = library
                        .progress
                        .phase
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let label = match phase {
                        0 => "Finding audio files…".to_owned(),
                        1 => format!("Reading track tags · {completed}/{total}"),
                        _ => format!("Loading album artwork · {completed}/{total}"),
                    };
                    let fraction = if total == 0 {
                        0.0
                    } else {
                        completed as f32 / total as f32
                    };
                    let progress_width = (ui.available_width() - 288.0).clamp(120.0, 270.0);
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(progress_width)
                            .text(label)
                            .animate(true),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::new()
                        .fill(colors.panel)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            colors.border,
                        ))
                        .corner_radius(3.0)
                        .inner_margin(egui::Margin::symmetric(10, 5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(18.0, 20.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    egui_phosphor::regular::MAGNIFYING_GLASS,
                                    egui::FontId::new(
                                        18.0,
                                        egui::FontFamily::Name("phosphor".into()),
                                    ),
                                    colors.text,
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut library.view.search)
                                        .text_color(colors.text)
                                        .hint_text(
                                            egui::RichText::new("Search albums")
                                                .color(colors.text),
                                        )
                                        .desired_width(230.0)
                                        .frame(false),
                                );
                            });
                        });
                });
            });
            let has_notices = error.is_some()
                || library.model.missing_metadata_tracks > 0
                || !library.model.skipped_empty_files.is_empty()
                || !library.model.scan_errors.is_empty();
            if has_notices {
                let notice_signature = format!(
                    "{:?}|{}|{:?}|{:?}",
                    error,
                    library.model.missing_metadata_tracks,
                    library.model.skipped_empty_files,
                    library.model.scan_errors
                );
                if dismissed_notice_signature.as_deref()
                    != Some(notice_signature.as_str())
                {
                    egui::Frame::new()
                        .fill(colors.panel)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            colors.border,
                        ))
                        .corner_radius(5.0)
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.horizontal_top(|ui| {
                                ui.vertical(|ui| {
                                    if let Some(message) = error {
                                        ui.colored_label(colors.danger, message);
                                    }
                                    if library.model.missing_metadata_tracks > 0 {
                                        ui.colored_label(
                                            colors.warning,
                                            format!(
                                                "{} tracks have no metadata; using filenames and folders.",
                                                library.model.missing_metadata_tracks
                                            ),
                                        );
                                    }
                                    if !library.model.skipped_empty_files.is_empty() {
                                        egui::CollapsingHeader::new(
                                            egui::RichText::new(format!(
                                                "Skipped {} empty audio files (0 bytes)",
                                                library.model.skipped_empty_files.len()
                                            ))
                                            .color(colors.warning),
                                        )
                                        .id_salt("skipped-empty-audio-files")
                                        .show(ui, |ui| {
                                            for path in &library.model.skipped_empty_files {
                                                ui.label(path.display().to_string());
                                            }
                                        });
                                    }
                                    if !library.model.scan_errors.is_empty() {
                                        egui::CollapsingHeader::new(
                                            egui::RichText::new(format!(
                                                "Could not read tags for {} tracks",
                                                library.model.scan_errors.len()
                                            ))
                                            .color(colors.danger),
                                        )
                                        .id_salt("scan-track-errors")
                                        .show(ui, |ui| {
                                            for error in &library.model.scan_errors {
                                                ui.colored_label(
                                                    colors.danger,
                                                    error,
                                                );
                                            }
                                        });
                                    }
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Min),
                                    |ui| {
                                        if ui
                                            .small_button("×")
                                            .on_hover_text("Dismiss notices")
                                            .clicked()
                                        {
                                            *dismissed_notice_signature =
                                                Some(notice_signature.clone());
                                        }
                                    },
                                );
                            });
                        });
                }
            }
        });
}
