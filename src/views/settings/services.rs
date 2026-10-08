use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.set_min_width(400.0);

    ui.horizontal(|ui| {
        ui.with_layout(
            eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);
                section_title(ui, "External Services");
                ui.add_space(8.0);

                ui.label("Connect third-party services for enhanced functionality.");
                ui.add_space(16.0);

                service_row(ui, "Last.fm", "Scrobble your listening activity");
                ui.add_space(8.0);
                service_row(ui, "Discogs", "Sync your collection");
                ui.add_space(8.0);
                service_row(ui, "Spotify", "Sync playlists and favorites");

                ui.add_space(24.0);

                section_title(ui, "WebDAV");
                ui.add_space(8.0);

                ui.label("Sync settings with a WebDAV server.");
                ui.add_space(16.0);

                ui.checkbox(&mut false, "Enable WebDAV sync");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Server URL:");
                    ui.add(
                        eframe::egui::TextEdit::singleline(&mut String::new())
                            .hint_text("https://example.com/webdav"),
                    );
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

fn service_row(ui: &mut egui::Ui, name: &str, description: &str) {
    ui.horizontal(|ui| {
        let mut enabled = false;
        if ui.checkbox(&mut enabled, "").clicked() {
            // TODO: Toggle service
        }
        ui.vertical(|ui| {
            ui.label(
                eframe::egui::RichText::new(name)
                    .size(13.0)
                    .strong()
                    .color(ui.visuals().widgets.active.text_color()),
            );
            ui.label(
                eframe::egui::RichText::new(description)
                    .size(11.0)
                    .color(ui.visuals().widgets.active.text_color().gamma_multiply(0.6)),
            );
        });
    });
}
