use eframe::egui;

#[derive(Clone, Copy)]
pub enum Icon {
    Previous,
    Next,
    Stop,
    Music,
    Album,
    Home,
    Albums,
    Play,
    Pause,
    Search,
    Close,
    Volume,
}

pub fn button(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 32.0), egui::Sense::click());
    let color = if response.hovered() {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_gray(190)
    };
    draw(ui.painter(), rect, icon, color);
    response.on_hover_text(label)
}

pub fn draw(painter: &egui::Painter, rect: egui::Rect, icon: Icon, color: egui::Color32) {
    let center = rect.center();
    let stroke = egui::Stroke::new(1.8_f32, color);
    match icon {
        Icon::Previous => {
            painter.line_segment(
                [
                    center + egui::vec2(-8.0, -7.0),
                    center + egui::vec2(-8.0, 7.0),
                ],
                stroke,
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + egui::vec2(6.0, -7.0),
                    center + egui::vec2(-5.0, 0.0),
                    center + egui::vec2(6.0, 7.0),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        Icon::Next => {
            painter.line_segment(
                [
                    center + egui::vec2(8.0, -7.0),
                    center + egui::vec2(8.0, 7.0),
                ],
                stroke,
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + egui::vec2(-6.0, -7.0),
                    center + egui::vec2(5.0, 0.0),
                    center + egui::vec2(-6.0, 7.0),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        Icon::Stop => {
            painter.rect_stroke(
                rect.shrink2(egui::vec2(11.0, 10.0)),
                1.5,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Icon::Music => {
            let top = center + egui::vec2(3.5, -8.0);
            let bottom = center + egui::vec2(3.5, 4.5);
            painter.line_segment([top, bottom], stroke);
            painter.line_segment([top, top + egui::vec2(7.0, -2.0)], stroke);
            painter.line_segment(
                [top + egui::vec2(7.0, -2.0), bottom + egui::vec2(7.0, -2.0)],
                stroke,
            );
            painter.add(egui::Shape::ellipse_filled(
                bottom + egui::vec2(-2.0, 1.0),
                egui::vec2(4.0, 2.7),
                color,
            ));
            painter.add(egui::Shape::ellipse_filled(
                bottom + egui::vec2(5.0, -1.0),
                egui::vec2(4.0, 2.7),
                color,
            ));
        }
        Icon::Album => {
            painter.circle_stroke(center, 7.0, stroke);
            painter.circle_stroke(center, 2.5, stroke);
            painter.line_segment(
                [
                    center + egui::vec2(-7.0, 0.0),
                    center + egui::vec2(-2.5, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [center + egui::vec2(2.5, 0.0), center + egui::vec2(7.0, 0.0)],
                stroke,
            );
        }
        Icon::Home => {
            painter.line_segment(
                [
                    center + egui::vec2(-8.0, -1.0),
                    center + egui::vec2(0.0, -8.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(0.0, -8.0),
                    center + egui::vec2(8.0, -1.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-6.0, -2.0),
                    center + egui::vec2(-6.0, 7.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(6.0, -2.0),
                    center + egui::vec2(6.0, 7.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-6.0, 7.0),
                    center + egui::vec2(6.0, 7.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-1.5, 7.0),
                    center + egui::vec2(-1.5, 2.0),
                ],
                stroke,
            );
            painter.line_segment(
                [center + egui::vec2(1.5, 7.0), center + egui::vec2(1.5, 2.0)],
                stroke,
            );
        }
        Icon::Albums => {
            for (x, y) in [(-5.0, -5.0), (3.0, -5.0), (-5.0, 3.0), (3.0, 3.0)] {
                painter.rect_stroke(
                    egui::Rect::from_center_size(center + egui::vec2(x, y), egui::vec2(5.0, 5.0)),
                    1.0,
                    stroke,
                    egui::StrokeKind::Middle,
                );
            }
        }
        Icon::Play => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + egui::vec2(-4.0, -7.0),
                    center + egui::vec2(7.0, 0.0),
                    center + egui::vec2(-4.0, 7.0),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        Icon::Pause => {
            painter.rect_filled(
                egui::Rect::from_center_size(center + egui::vec2(-3.0, 0.0), egui::vec2(3.0, 13.0)),
                1.0,
                color,
            );
            painter.rect_filled(
                egui::Rect::from_center_size(center + egui::vec2(3.0, 0.0), egui::vec2(3.0, 13.0)),
                1.0,
                color,
            );
        }
        Icon::Search => {
            painter.circle_stroke(center + egui::vec2(-2.0, -2.0), 6.0, stroke);
            painter.line_segment(
                [center + egui::vec2(2.5, 2.5), center + egui::vec2(7.0, 7.0)],
                stroke,
            );
        }
        Icon::Close => {
            painter.line_segment(
                [
                    center + egui::vec2(-5.0, -5.0),
                    center + egui::vec2(5.0, 5.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(5.0, -5.0),
                    center + egui::vec2(-5.0, 5.0),
                ],
                stroke,
            );
        }
        Icon::Volume => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + egui::vec2(-8.0, -3.0),
                    center + egui::vec2(-4.0, -3.0),
                    center + egui::vec2(2.0, -8.0),
                    center + egui::vec2(2.0, 8.0),
                    center + egui::vec2(-4.0, 3.0),
                    center + egui::vec2(-8.0, 3.0),
                ],
                color,
                egui::Stroke::NONE,
            ));
            for radius in [6.0_f32, 10.0] {
                let points = [-0.85_f32, -0.4, 0.0, 0.4, 0.85].map(|angle| {
                    center + egui::vec2(2.0 + radius * angle.cos(), radius * angle.sin())
                });
                for segment in points.windows(2) {
                    painter.line_segment([segment[0], segment[1]], stroke);
                }
            }
        }
    }
}
