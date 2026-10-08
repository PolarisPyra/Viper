use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.set_min_width(400.0);

    ui.horizontal(|ui| {
        ui.with_layout(
            eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);

                // App icon/title
                ui.horizontal(|ui| {
                    ui.add_space(40.0); // Placeholder for icon
                    ui.with_layout(
                        eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
                        |ui| {
                            ui.label(
                                eframe::egui::RichText::new("Music Player")
                                    .size(24.0)
                                    .strong()
                                    .color(ui.visuals().widgets.active.text_color()),
                            );
                            ui.label(
                                eframe::egui::RichText::new("Version 1.0.0")
                                    .size(14.0)
                                    .color(
                                        ui.visuals()
                                            .widgets
                                            .active
                                            .text_color()
                                            .gamma_multiply(0.6),
                                    ),
                            );
                        },
                    );
                });

                ui.add_space(32.0);

                section_title(ui, "About");
                ui.add_space(8.0);

                ui.label("A simple, beautiful music player for your desktop.");
                ui.add_space(16.0);
                ui.label("Created with Rust and egui.");

                ui.add_space(32.0);

                section_title(ui, "Credits");
                ui.add_space(8.0);

                ui.label("Development");
                ui.add_space(4.0);
                ui.label("• Lead Developer");
                ui.label("•Contributors");

                ui.add_space(16.0);

                ui.label("Libraries");
                ui.add_space(4.0);
                ui.label("• eframe - Rust GUI framework");
                ui.label("• egui - Immediate mode GUI");
                ui.label("• rodio - Audio playback");

                ui.add_space(32.0);

                section_title(ui, "License");
                ui.add_space(8.0);

                ui.label("This project is licensed under the MIT License.");

                ui.add_space(16.0);

                if ui.button("Open License").clicked() {
                    // TODO: Open license file
                }
            },
        );
    });
}

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(
        eframe::egui::RichText::new(title)
            .size(15.0)
            .strong()
            .color(ui.visuals().widgets.active.text_color()),
    );
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(8.0);
}
