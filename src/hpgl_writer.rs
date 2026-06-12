use std::fmt::Write;

use crate::model::{Drawing, Entity, Point};
use crate::{ConversionOptions, PltDialect};

pub fn write_hpgl2(drawing: &Drawing, options: &ConversionOptions) -> String {
    let bounds = drawing.bounds().unwrap_or_default();
    let width = (bounds.min.x.abs().max(bounds.max.x.abs()) * options.units_per_mm).ceil() as i64;
    let height = (bounds.min.y.abs().max(bounds.max.y.abs()) * options.units_per_mm).ceil() as i64;
    let pen_count = drawing
        .entities
        .iter()
        .map(Entity::pen)
        .max()
        .unwrap_or(1)
        .clamp(1, 255);

    let mut output = String::with_capacity(drawing.entities.len() * 96);
    match options.plt_dialect {
        PltDialect::Hpgl => output.push_str("IN;"),
        PltDialect::Hpgl2 => {
            output.push_str("\x1b%-1B");
            let _ = write!(
                output,
                "BP;IN;PS{},{};NP{};",
                width.max(1),
                height.max(1),
                pen_count
            );
            for style in &drawing.pens {
                if let Some((red, green, blue)) = style.color_rgb {
                    let _ = write!(output, "PC{},{},{},{};", style.number, red, green, blue);
                }
                if let Some(width) = style.width_mm {
                    let _ = write!(output, "PW{},{};", number(width), style.number);
                }
            }
        }
    }

    let mut current_pen = u16::MAX;
    for entity in &drawing.entities {
        if entity.pen() != current_pen {
            let _ = write!(output, "SP{};", entity.pen().clamp(0, 255));
            current_pen = entity.pen();
        }
        write_entity(&mut output, entity, options.units_per_mm);
    }
    if options.plt_dialect == PltDialect::Hpgl2 {
        output.push_str("PU;SP0;PG;\r\n");
    } else {
        output.push_str("PU;SP0;\r\n");
    }
    output
}

fn write_entity(output: &mut String, entity: &Entity, units_per_mm: f64) {
    match entity {
        Entity::Polyline { points, closed, .. } => {
            let Some(first) = points.first() else {
                return;
            };
            write_move(output, "PU", *first, units_per_mm);
            output.push_str("PD");
            for (index, point) in points.iter().skip(1).enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_coordinate(output, *point, units_per_mm);
            }
            if *closed
                && points
                    .last()
                    .is_some_and(|last| last.distance(*first) > 1e-8)
            {
                if points.len() > 1 {
                    output.push(',');
                }
                write_coordinate(output, *first, units_per_mm);
            }
            output.push(';');
        }
        Entity::Circle { center, radius, .. } => {
            write_move(output, "PU", *center, units_per_mm);
            let _ = write!(output, "CI{};", coordinate(*radius, units_per_mm));
        }
        Entity::Arc {
            center,
            radius,
            start_deg,
            end_deg,
            ..
        } => {
            let start_radians = start_deg.to_radians();
            let start = Point::new(
                center.x + radius * start_radians.cos(),
                center.y + radius * start_radians.sin(),
            );
            write_move(output, "PU", start, units_per_mm);
            let sweep = (end_deg - start_deg).rem_euclid(360.0);
            let _ = write!(
                output,
                "AA{},{},{};",
                coordinate(center.x, units_per_mm),
                coordinate(center.y, units_per_mm),
                number(sweep)
            );
        }
        Entity::Text {
            position,
            value,
            height,
            rotation_deg,
            ..
        } => {
            write_move(output, "PU", *position, units_per_mm);
            let angle = rotation_deg.to_radians();
            let _ = write!(
                output,
                "SI{},{};DI{},{};LB{}\x03",
                number(height * 0.76 / 10.0),
                number(*height / 10.0),
                number(angle.cos()),
                number(angle.sin()),
                sanitize_label(value)
            );
        }
    }
}

fn write_move(output: &mut String, command: &str, point: Point, units_per_mm: f64) {
    output.push_str(command);
    write_coordinate(output, point, units_per_mm);
    output.push(';');
}

fn write_coordinate(output: &mut String, point: Point, units_per_mm: f64) {
    let _ = write!(
        output,
        "{},{}",
        coordinate(point.x, units_per_mm),
        coordinate(point.y, units_per_mm)
    );
}

fn coordinate(value: f64, units_per_mm: f64) -> i64 {
    (value * units_per_mm).round() as i64
}

fn number(value: f64) -> String {
    let mut formatted = format!("{value:.6}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.push('0');
    }
    formatted
}

fn sanitize_label(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\x03' => ' ',
            '\r' | '\n' => ' ',
            _ => character,
        })
        .collect()
}
