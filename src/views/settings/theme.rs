use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.set_min_width(400.0);

    ui.horizontal(|ui| {
        ui.with_layout(
            eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);
                section_title(ui, "Appearance");
                ui.add_space(8.0);

                ui.label("Customize the look and feel of the application.");
                ui.add_space(16.0);

                ui.label(
                    eframe::egui::RichText::new("Theme")
                        .size(13.0)
                        .strong()
                        .color(ui.visuals().widgets.inactive.text_color()),
                );
                ui.add_space(4.0);
                ui.label("Choose a theme for the application.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Dark (Default)").clicked() {
                        // TODO: Change theme
                    }
                    if ui.button("Light").clicked() {
                        // TODO: Change theme
                    }
                    if ui.button("System").clicked() {
                        // TODO: Change theme
                    }
                });

                ui.add_space(24.0);

                section_title(ui, "Font Size");
                ui.add_space(8.0);

                ui.label("Adjust the font size for better readability.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Font size:");
                    if ui.button("-").clicked() {
                        // TODO: Decrease font size
                    }
                    ui.label("14px");
                    if ui.button("+").clicked() {
                        // TODO: Increase font size
                    }
                });

                ui.add_space(24.0);

                section_title(ui, "Display");
                ui.add_space(8.0);

                ui.label("Display settings.");
                ui.add_space(16.0);

                ui.checkbox(&mut false, "Show album artwork in window title");
                ui.add_space(4.0);
                ui.checkbox(&mut false, "Always show scrollbars");
                ui.add_space(4.0);
                ui.checkbox(&mut false, "Smooth scrolling");
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
