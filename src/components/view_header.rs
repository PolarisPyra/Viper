use crate::app::{Page, ViperApp};
use eframe::egui;

pub fn show(ctx: &egui::Context, app: &mut ViperApp) {
    egui::TopBottomPanel::top("library-topbar")
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(16, 18, 23))
                .inner_margin(egui::Margin::symmetric(20, 12)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(match app.page {
                        Page::Home => "Home",
                        Page::Albums => "Albums",
                    })
                    .size(28.0)
                    .strong()
                    .color(egui::Color32::WHITE),
                );
                if app.scanning {
                    let completed = app
                        .scan_progress
                        .completed
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let total = app
                        .scan_progress
                        .total
                        .load(std::sync::atomic::Ordering::Relaxed);
                    let phase = app
                        .scan_progress
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
                        .fill(egui::Color32::from_rgb(27, 31, 41))
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgb(43, 48, 61),
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
                                    egui::Color32::WHITE,
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut app.search)
                                        .text_color(egui::Color32::WHITE)
                                        .hint_text(
                                            egui::RichText::new("Search albums")
                                                .color(egui::Color32::WHITE),
                                        )
                                        .desired_width(230.0)
                                        .frame(false),
                                );
                            });
                        });
                });
            });
            if let Some(error) = &app.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        });
}
