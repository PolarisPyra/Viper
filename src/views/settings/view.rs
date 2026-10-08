use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.set_min_width(400.0);

    ui.horizontal(|ui| {
        ui.with_layout(
            eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);
                section_title(ui, "Layout");
                ui.add_space(8.0);

                ui.label("Configure the layout and arrangement of views.");
                ui.add_space(16.0);

                ui.checkbox(&mut false, "Show sidebar");
                ui.add_space(4.0);
                ui.label("Display the left navigation sidebar.");
                ui.add_space(8.0);

                ui.checkbox(&mut false, "Show album details panel");
                ui.add_space(4.0);
                ui.label("Show additional information when viewing albums.");

                ui.add_space(24.0);

                section_title(ui, "Grid View");
                ui.add_space(8.0);

                ui.label("Settings for album grid display.");
                ui.add_space(16.0);

                ui.label("Album art size:");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button("Small").clicked() {
                        // TODO: Change art size
                    }
                    if ui.button("Medium").clicked() {
                        // TODO: Change art size
                    }
                    if ui.button("Large").clicked() {
                        // TODO: Change art size
                    }
                });

                ui.add_space(24.0);

                section_title(ui, "Album Sorting");
                ui.add_space(8.0);

                ui.label("Default sorting for albums.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Sort by:");
                    if ui.button("Name").clicked() {
                        // TODO: Change sort
                    }
                    if ui.button("Artist").clicked() {
                        // TODO: Change sort
                    }
                    if ui.button("Year").clicked() {
                        // TODO: Change sort
                    }
                });
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
