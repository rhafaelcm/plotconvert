use std::collections::BTreeSet;
use std::fmt::Write;

use crate::ConversionOptions;
use crate::model::{Drawing, Entity, PenStyle, Point};

pub fn write_r12(drawing: &Drawing, options: &ConversionOptions) -> String {
    let bounds = drawing.bounds().unwrap_or_default();
    let mut output = String::with_capacity(drawing.entities.len() * 160);
    pair(&mut output, 0, "SECTION");
    pair(&mut output, 2, "HEADER");
    pair(&mut output, 9, "$ACADVER");
    pair(&mut output, 1, "AC1009");
    pair(&mut output, 9, "$INSUNITS");
    pair(&mut output, 70, "4");
    pair(&mut output, 9, "$EXTMIN");
    point3(&mut output, bounds.min);
    pair(&mut output, 9, "$EXTMAX");
    point3(&mut output, bounds.max);
    pair(&mut output, 0, "ENDSEC");

    pair(&mut output, 0, "SECTION");
    pair(&mut output, 2, "TABLES");
    write_ltype_table(&mut output);
    write_layer_table(&mut output, drawing, options);
    pair(&mut output, 0, "ENDSEC");

    pair(&mut output, 0, "SECTION");
    pair(&mut output, 2, "ENTITIES");
    for entity in &drawing.entities {
        write_entity(&mut output, drawing, entity, options);
    }
    pair(&mut output, 0, "ENDSEC");
    pair(&mut output, 0, "EOF");
    output
}

fn write_ltype_table(output: &mut String) {
    pair(output, 0, "TABLE");
    pair(output, 2, "LTYPE");
    pair(output, 70, "1");
    pair(output, 0, "LTYPE");
    pair(output, 2, "CONTINUOUS");
    pair(output, 70, "0");
    pair(output, 3, "Solid line");
    pair(output, 72, "65");
    pair(output, 73, "0");
    pair(output, 40, "0.0");
    pair(output, 0, "ENDTAB");
}

fn write_layer_table(output: &mut String, drawing: &Drawing, options: &ConversionOptions) {
    let pens: BTreeSet<u16> = drawing.entities.iter().map(Entity::pen).collect();
    pair(output, 0, "TABLE");
    pair(output, 2, "LAYER");
    let layer_count = if options.single_layer {
        "1".to_owned()
    } else {
        pens.len().to_string()
    };
    pair(output, 70, &layer_count);
    if options.single_layer {
        write_layer(output, "0", 7);
    } else {
        for pen in pens {
            let style = drawing.pens.iter().find(|style| style.number == pen);
            write_layer(output, &layer_name(pen), aci_color(style, pen));
        }
    }
    pair(output, 0, "ENDTAB");
}

fn write_layer(output: &mut String, name: &str, color: u8) {
    pair(output, 0, "LAYER");
    pair(output, 2, name);
    pair(output, 70, "0");
    pair(output, 62, &color.to_string());
    pair(output, 6, "CONTINUOUS");
}

fn write_entity(
    output: &mut String,
    drawing: &Drawing,
    entity: &Entity,
    options: &ConversionOptions,
) {
    let layer = if options.single_layer {
        "0".to_owned()
    } else {
        layer_name(entity.pen())
    };
    match entity {
        Entity::Polyline { points, closed, .. } => {
            pair(output, 0, "POLYLINE");
            pair(output, 8, &layer);
            pair(output, 66, "1");
            pair(output, 70, if *closed { "1" } else { "0" });
            if let Some(width) = drawing
                .pens
                .iter()
                .find(|style| style.number == entity.pen())
                .and_then(|style| style.width_mm)
                .filter(|width| *width > 0.0)
            {
                pair(output, 40, &number(width));
                pair(output, 41, &number(width));
            }
            for point in points {
                pair(output, 0, "VERTEX");
                pair(output, 8, &layer);
                point3(output, *point);
                pair(output, 70, "0");
            }
            pair(output, 0, "SEQEND");
            pair(output, 8, &layer);
        }
        Entity::Circle { center, radius, .. } => {
            pair(output, 0, "CIRCLE");
            pair(output, 8, &layer);
            point3(output, *center);
            pair(output, 40, &number(*radius));
        }
        Entity::Arc {
            center,
            radius,
            start_deg,
            end_deg,
            ..
        } => {
            pair(output, 0, "ARC");
            pair(output, 8, &layer);
            point3(output, *center);
            pair(output, 40, &number(*radius));
            pair(output, 50, &number(start_deg.rem_euclid(360.0)));
            pair(output, 51, &number(end_deg.rem_euclid(360.0)));
        }
        Entity::Text {
            position,
            value,
            height,
            rotation_deg,
            ..
        } => {
            pair(output, 0, "TEXT");
            pair(output, 8, &layer);
            point3(output, *position);
            pair(output, 40, &number(*height));
            pair(output, 1, &sanitize_text(value));
            pair(output, 50, &number(*rotation_deg));
            pair(output, 7, "STANDARD");
        }
    }
}

fn point3(output: &mut String, point: Point) {
    pair(output, 10, &number(point.x));
    pair(output, 20, &number(point.y));
    pair(output, 30, "0.0");
}

fn pair(output: &mut String, code: i32, value: &str) {
    let _ = write!(output, "{code:>3}\r\n{value}\r\n");
}

fn number(value: f64) -> String {
    let mut formatted = format!("{value:.9}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.push('0');
    }
    if formatted == "-0.0" {
        "0.0".into()
    } else {
        formatted
    }
}

fn layer_name(pen: u16) -> String {
    format!("PEN_{pen:03}")
}

fn aci_color(style: Option<&PenStyle>, pen: u16) -> u8 {
    let Some((red, green, blue)) = style.and_then(|style| style.color_rgb) else {
        const DEFAULTS: [u8; 7] = [7, 1, 3, 5, 2, 4, 6];
        return DEFAULTS[pen.saturating_sub(1) as usize % DEFAULTS.len()];
    };
    let candidates = [
        (1, (255_i32, 0_i32, 0_i32)),
        (2, (255, 255, 0)),
        (3, (0, 255, 0)),
        (4, (0, 255, 255)),
        (5, (0, 0, 255)),
        (6, (255, 0, 255)),
        (7, (255, 255, 255)),
    ];
    candidates
        .into_iter()
        .min_by_key(|(_, (r, g, b))| {
            (red as i32 - r).pow(2) + (green as i32 - g).pow(2) + (blue as i32 - b).pow(2)
        })
        .map(|(aci, _)| aci)
        .unwrap_or(7)
}

fn sanitize_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character == '\0' || character == '\r' || character == '\n' {
                ' '
            } else {
                character
            }
        })
        .collect()
}
