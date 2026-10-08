use eframe::egui;

const BUTTON: egui::Color32 = egui::Color32::from_rgb(37, 40, 53);
const BORDER: egui::Color32 = egui::Color32::from_rgb(67, 72, 91);
const ROW_HOVERED: egui::Color32 = egui::Color32::from_rgb(32, 36, 47);
const ROW_SELECTED: egui::Color32 = egui::Color32::from_rgb(34, 38, 49);

pub fn show(ui: &mut egui::Ui, app: &mut crate::app::MusicApp) {
    ui.set_min_width(420.0);
    ui.add_space(8.0);
    section_title(ui, "Music library");
    ui.label("Choose the folder that contains your music.");
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.set_width(ui.available_width());
        if let Some(path) = &app.settings.music_path {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(path.display().to_string())
                        .size(12.0)
                        .color(ui.visuals().widgets.inactive.text_color()),
                )
                .sense(egui::Sense::hover())
                .truncate(),
            );
        } else {
            ui.label(
                egui::RichText::new("No music folder selected")
                    .size(12.0)
                    .color(ui.visuals().widgets.inactive.text_color()),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = egui::Button::new(
                egui::RichText::new("Choose folder")
                    .size(12.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            )
            .fill(BUTTON)
            .stroke(egui::Stroke::new(1.0_f32, BORDER))
            .corner_radius(7.0)
            .min_size(egui::vec2(112.0, 34.0));
            if ui.add(button).clicked() {
                app.choose_folder();
            }
        });
    });

    ui.add_space(28.0);
    section_title(ui, "Startup");
    ui.label("Choose which page opens when the player starts.");
    ui.add_space(12.0);

    let startup_view = app.settings.startup_view;
    let mut new_startup_view = startup_view;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        for (page, label) in [
            (crate::storage::settings::StartupView::Home, "Home"),
            (crate::storage::settings::StartupView::Albums, "Albums"),
        ] {
            let is_startup = new_startup_view == page;
            let (rect, response) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::click());
            let fill = if response.hovered() {
                ROW_HOVERED
            } else if is_startup {
                ROW_SELECTED
            } else {
                BUTTON
            };
            ui.painter().rect(
                rect,
                7.0,
                fill,
                egui::Stroke::new(1.0_f32, BORDER),
                egui::StrokeKind::Inside,
            );

            let center = egui::pos2(rect.left() + 18.0, rect.center().y);
            ui.painter().circle_stroke(
                center,
                7.0,
                egui::Stroke::new(
                    1.4_f32,
                    if is_startup {
                        egui::Color32::from_gray(210)
                    } else {
                        egui::Color32::from_gray(115)
                    },
                ),
            );
            if is_startup {
                ui.painter()
                    .circle_filled(center, 3.5, egui::Color32::from_gray(220));
            }
            ui.painter().text(
                egui::pos2(rect.left() + 36.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(13.0),
                egui::Color32::from_gray(if is_startup { 235 } else { 195 }),
            );
            if is_startup {
                ui.painter().text(
                    egui::pos2(rect.right() - 12.0, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    "Current startup page",
                    egui::FontId::proportional(11.0),
                    egui::Color32::from_gray(145),
                );
            }
            if response.clicked() {
                new_startup_view = page;
            }
        }
    });
    if new_startup_view != startup_view {
        app.settings.startup_view = new_startup_view;
        if let Err(error) = app.settings.save() {
            app.error = Some(format!("Could not save settings: {error}"));
        }
    }
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
