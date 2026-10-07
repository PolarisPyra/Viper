use eframe::egui;

pub fn show(ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(16, 18, 23))
                .inner_margin(0),
        )
        .show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Home")
                        .size(22.0)
                        .color(egui::Color32::from_gray(145)),
                );
            });
        });
}
