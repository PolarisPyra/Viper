use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.set_min_width(400.0);

    ui.horizontal(|ui| {
        ui.with_layout(
            eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
            |ui| {
                ui.add_space(10.0);
                section_title(ui, "Network");
                ui.add_space(8.0);

                ui.label("Network connection settings.");
                ui.add_space(16.0);

                ui.checkbox(&mut false, "Enable network connectivity");
                ui.add_space(4.0);
                ui.label("Allow the application to connect to the internet.");

                ui.add_space(24.0);

                section_title(ui, "Proxy");
                ui.add_space(8.0);

                ui.label("Configure proxy settings for network connections.");
                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    ui.label("Proxy type:");
                    ui.selectable_value(&mut ProxyType::None, ProxyType::None, "None");
                    ui.selectable_value(&mut ProxyType::Http, ProxyType::Http, "HTTP");
                    ui.selectable_value(&mut ProxyType::Socks5, ProxyType::Socks5, "SOCKS5");
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.add(
                        eframe::egui::TextEdit::singleline(&mut String::new())
                            .hint_text("127.0.0.1"),
                    );
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.add(
                        eframe::egui::TextEdit::singleline(&mut String::new()).hint_text("8080"),
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProxyType {
    None,
    Http,
    Socks5,
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::None
    }
}
