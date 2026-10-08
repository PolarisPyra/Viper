use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.set_min_width(400.0);

    ui.horizontal(|ui| {
        ui.with_layout(
            eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);
                section_title(ui, "Audio Output");
                ui.add_space(8.0);

                ui.label("Configure audio output devices and settings.");
                ui.add_space(16.0);

                ui.label(
                    eframe::egui::RichText::new("Output Device")
                        .size(13.0)
                        .strong()
                        .color(ui.visuals().widgets.inactive.text_color()),
                );
                ui.add_space(4.0);
                ui.label("Select the audio device for playback.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    let devices = vec![
                        "Default Device",
                        "Speakers",
                        "Headphones",
                        "Bluetooth Speaker",
                    ];
                    let mut selected_device = devices[0].to_string();
                    if ui
                        .selectable_value(
                            &mut selected_device,
                            devices[0].to_string(),
                            "Default Device",
                        )
                        .changed()
                    {
                        // TODO: Handle device change
                    }
                });

                ui.add_space(24.0);

                section_title(ui, "Volume");
                ui.add_space(8.0);

                ui.label("Default volume level for playback.");
                ui.add_space(8.0);

                // Create a dummy volume value for demonstration
                let mut dummy_volume = 70.0;
                ui.horizontal(|ui| {
                    ui.label("Volume:");
                    ui.add(eframe::egui::Slider::new(&mut dummy_volume, 0.0..=100.0));
                    ui.label(format!("{}%", dummy_volume as i32));
                });

                ui.add_space(24.0);

                section_title(ui, "Playback");
                ui.add_space(8.0);

                ui.label("Playback behavior settings.");
                ui.add_space(16.0);

                let mut dummy_bool = false;
                ui.checkbox(&mut dummy_bool, "Normalize volume across tracks");
                ui.add_space(4.0);
                ui.checkbox(&mut dummy_bool, "Crossfade between tracks");
                ui.add_space(4.0);
                ui.checkbox(&mut dummy_bool, "Play next track automatically");
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
