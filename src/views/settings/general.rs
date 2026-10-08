use eframe::egui;

pub fn show(ui: &mut egui::Ui, app: &mut crate::app::MusicApp) {
    ui.set_min_width(400.0);

    ui.horizontal(|ui| {
        ui.with_layout(
            eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);
                section_title(ui, "General");
                ui.add_space(8.0);

                ui.label("General settings for the application.");
                ui.add_space(16.0);

                ui.label(
                    eframe::egui::RichText::new("Music Path")
                        .size(13.0)
                        .strong()
                        .color(ui.visuals().widgets.inactive.text_color()),
                );
                ui.add_space(4.0);
                ui.label("The location where your music files are stored.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Music folder:");
                    if let Some(path) = &app.settings.music_path {
                        ui.label(
                            eframe::egui::RichText::new(path.display().to_string())
                                .size(11.0)
                                .color(ui.visuals().widgets.inactive.text_color()),
                        );
                    } else {
                        ui.label(
                            eframe::egui::RichText::new("No folder selected")
                                .size(11.0)
                                .color(ui.visuals().widgets.inactive.text_color()),
                        );
                    }
                    if ui.button("Choose Folder").clicked() {
                        app.choose_folder();
                    }
                });

                ui.add_space(24.0);

                section_title(ui, "Startup");
                ui.add_space(8.0);

                ui.label("Controls what happens when the application starts.");
                ui.add_space(16.0);

                let startup_view = app.settings.startup_view;
                ui.horizontal(|ui| {
                    ui.label("Startup view:");
                    let mut new_startup_view = startup_view;
                    ui.selectable_value(
                        &mut new_startup_view,
                        crate::storage::settings::StartupView::Home,
                        "Home",
                    )
                    .on_hover_text("Show Home view on startup");
                    ui.selectable_value(
                        &mut new_startup_view,
                        crate::storage::settings::StartupView::Albums,
                        "Albums",
                    )
                    .on_hover_text("Show Albums view on startup");
                    if new_startup_view != startup_view {
                        app.settings.startup_view = new_startup_view;
                        if let Err(error) = app.settings.save() {
                            app.error = Some(format!("Could not save settings: {error}"));
                        }
                    }
                });

                ui.add_space(24.0);

                section_title(ui, "Behavior");
                ui.add_space(8.0);

                ui.label("Application behavior settings.");
                ui.add_space(16.0);

                ui.checkbox(
                    &mut app.settings.left_panel_hidden,
                    "Hide sidebar by default",
                )
                .on_hover_text("Hide the library sidebar on startup");
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
