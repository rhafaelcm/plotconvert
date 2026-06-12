use std::fmt::Write;

use crate::model::{Drawing, Entity, Point};

pub fn write_svg(drawing: &Drawing) -> String {
    let bounds = drawing.bounds().unwrap_or_default();
    let width = (bounds.max.x - bounds.min.x).abs().max(0.001);
    let height = (bounds.max.y - bounds.min.y).abs().max(0.001);
    let mut output = String::with_capacity(drawing.entities.len() * 180);
    let _ = writeln!(output, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        output,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}mm" height="{}mm" viewBox="{} {} {} {}">"#,
        number(width),
        number(height),
        number(bounds.min.x),
        number(-bounds.max.y),
        number(width),
        number(height)
    );
    output.push_str("  <g fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n");
    for entity in &drawing.entities {
        write_entity(&mut output, drawing, entity);
    }
    output.push_str("  </g>\n</svg>\n");
    output
}

fn write_entity(output: &mut String, drawing: &Drawing, entity: &Entity) {
    let style = drawing
        .pens
        .iter()
        .find(|style| style.number == entity.pen());
    let mut color = style
        .and_then(|style| style.color_rgb)
        .unwrap_or_else(|| default_color(entity.pen()));
    // ACI 7 is commonly white on dark CAD backgrounds and black on paper.
    if color == (255, 255, 255) {
        color = (0, 0, 0);
    }
    let width = style.and_then(|style| style.width_mm).unwrap_or(0.25);
    let attributes = format!(
        r##"stroke="#{:02x}{:02x}{:02x}" stroke-width="{}" vector-effect="non-scaling-stroke" data-pen="{}""##,
        color.0,
        color.1,
        color.2,
        number(width),
        entity.pen()
    );

    match entity {
        Entity::Polyline { points, closed, .. } => {
            if points.is_empty() {
                return;
            }
            let tag = if *closed { "polygon" } else { "polyline" };
            let mut coordinates = String::new();
            for (index, point) in points.iter().enumerate() {
                if index > 0 {
                    coordinates.push(' ');
                }
                let _ = write!(coordinates, "{},{}", number(point.x), number(-point.y));
            }
            let _ = writeln!(output, "    <{tag} points=\"{coordinates}\" {attributes}/>");
        }
        Entity::Circle { center, radius, .. } => {
            let _ = writeln!(
                output,
                "    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" {attributes}/>",
                number(center.x),
                number(-center.y),
                number(*radius)
            );
        }
        Entity::Arc {
            center,
            radius,
            start_deg,
            end_deg,
            ..
        } => {
            let start_angle = start_deg.to_radians();
            let end_angle = end_deg.to_radians();
            let start = Point::new(
                center.x + radius * start_angle.cos(),
                center.y + radius * start_angle.sin(),
            );
            let end = Point::new(
                center.x + radius * end_angle.cos(),
                center.y + radius * end_angle.sin(),
            );
            let sweep = (end_deg - start_deg).rem_euclid(360.0);
            let large_arc = i32::from(sweep > 180.0);
            let _ = writeln!(
                output,
                "    <path d=\"M {} {} A {} {} 0 {} 0 {} {}\" {attributes}/>",
                number(start.x),
                number(-start.y),
                number(*radius),
                number(*radius),
                large_arc,
                number(end.x),
                number(-end.y)
            );
        }
        Entity::Text {
            position,
            value,
            height,
            rotation_deg,
            ..
        } => {
            let transform = if rotation_deg.abs() > 1e-9 {
                format!(
                    " transform=\"rotate({} {} {})\"",
                    number(-rotation_deg),
                    number(position.x),
                    number(-position.y)
                )
            } else {
                String::new()
            };
            let _ = writeln!(
                output,
                "    <text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"#{:02x}{:02x}{:02x}\" stroke=\"none\" data-pen=\"{}\"{}>{}</text>",
                number(position.x),
                number(-position.y),
                number(*height),
                color.0,
                color.1,
                color.2,
                entity.pen(),
                transform,
                escape_xml(value)
            );
        }
    }
}

fn number(value: f64) -> String {
    let mut formatted = format!("{value:.6}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.push('0');
    }
    if formatted == "-0.0" {
        "0".into()
    } else {
        formatted
    }
}

fn default_color(pen: u16) -> (u8, u8, u8) {
    const COLORS: [(u8, u8, u8); 7] = [
        (0, 0, 0),
        (255, 0, 0),
        (0, 160, 0),
        (0, 0, 255),
        (255, 128, 0),
        (160, 0, 160),
        (0, 160, 160),
    ];
    COLORS[pen.saturating_sub(1) as usize % COLORS.len()]
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
