use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use crate::model::{Drawing, Entity, PenStyle, Point};
use crate::parser::Command;
use crate::{ConversionError, ConversionOptions};

#[derive(Clone, Copy, Debug)]
struct Scale {
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
}

struct Interpreter<'a> {
    options: &'a ConversionOptions,
    drawing: Drawing,
    warnings: Vec<String>,
    warned: BTreeSet<String>,
    current: Point,
    absolute: bool,
    pen_down: bool,
    pen: u16,
    path: Vec<Point>,
    ip1: Point,
    ip2: Point,
    scale: Option<Scale>,
    rotation: i32,
    clip: Option<(Point, Point)>,
    polygon_mode: bool,
    polygon: Vec<Point>,
    char_width_mm: f64,
    char_height_mm: f64,
    char_angle_deg: f64,
    pens: BTreeMap<u16, PenStyle>,
}

pub fn interpret(
    commands: &[Command],
    options: &ConversionOptions,
) -> Result<(Drawing, Vec<String>), ConversionError> {
    let mut interpreter = Interpreter::new(options);
    for command in commands {
        interpreter.execute(command)?;
    }
    interpreter.flush_path(false);
    interpreter.drawing.pens = interpreter.pens.into_values().collect();
    Ok((interpreter.drawing, interpreter.warnings))
}

impl<'a> Interpreter<'a> {
    fn new(options: &'a ConversionOptions) -> Self {
        let mut pens = BTreeMap::new();
        pens.insert(1, PenStyle::new(1));
        Self {
            options,
            drawing: Drawing::default(),
            warnings: Vec::new(),
            warned: BTreeSet::new(),
            current: Point::default(),
            absolute: true,
            pen_down: false,
            pen: 1,
            path: Vec::new(),
            ip1: Point::default(),
            ip2: Point::new(1016.0, 1016.0),
            scale: None,
            rotation: 0,
            clip: None,
            polygon_mode: false,
            polygon: Vec::new(),
            char_width_mm: 2.85,
            char_height_mm: 3.75,
            char_angle_deg: 0.0,
            pens,
        }
    }

    fn execute(&mut self, command: &Command) -> Result<(), ConversionError> {
        let name = command.mnemonic();
        let numbers = command.numbers();
        match name.as_str() {
            "IN" | "DF" => self.reset_state(),
            "PA" => {
                self.absolute = true;
                self.consume_points(&numbers);
            }
            "PR" => {
                self.absolute = false;
                self.consume_points(&numbers);
            }
            "PU" => {
                self.pen_down = false;
                self.flush_path(false);
                self.consume_points(&numbers);
            }
            "PD" => {
                self.pen_down = true;
                self.consume_points(&numbers);
            }
            "SP" => {
                self.flush_path(false);
                self.pen = numbers.first().copied().unwrap_or(0.0).max(0.0) as u16;
                self.pens
                    .entry(self.pen)
                    .or_insert_with(|| PenStyle::new(self.pen));
            }
            "NP" => {
                if let Some(count) = numbers.first() {
                    for pen in 1..=(*count as u16) {
                        self.pens.entry(pen).or_insert_with(|| PenStyle::new(pen));
                    }
                }
            }
            "PC" => self.set_pen_color(&numbers),
            "PW" => self.set_pen_width(&numbers),
            "IP" => self.set_input_points(&numbers),
            "IR" => self.set_relative_input_points(&numbers),
            "SC" => self.set_scale(&numbers),
            "RO" => {
                self.flush_path(false);
                self.rotation = numbers.first().copied().unwrap_or(0.0) as i32;
            }
            "IW" => self.set_clip(&numbers),
            "CI" => self.circle(&numbers),
            "AA" => self.arc(&numbers, false, false),
            "AR" => self.arc(&numbers, true, false),
            "AT" => self.arc_three_points(&numbers, false),
            "RT" => self.arc_three_points(&numbers, true),
            "BZ" => self.bezier(&numbers, false),
            "BR" => self.bezier(&numbers, true),
            "EA" | "RA" => self.rectangle(&numbers, false),
            "ER" | "RR" => self.rectangle(&numbers, true),
            "EW" | "WG" => self.wedge(&numbers),
            "PM" => self.polygon_mode(&numbers),
            "EP" | "FP" => self.emit_polygon(),
            "LB" => self.label(command),
            "SI" => {
                if numbers.len() >= 2 {
                    self.char_width_mm = numbers[0] * 10.0;
                    self.char_height_mm = numbers[1] * 10.0;
                }
            }
            "SR" => {
                if numbers.len() >= 2 {
                    let width = (self.ip2.x - self.ip1.x).abs() / self.options.units_per_mm;
                    let height = (self.ip2.y - self.ip1.y).abs() / self.options.units_per_mm;
                    self.char_width_mm = width * numbers[0] / 100.0;
                    self.char_height_mm = height * numbers[1] / 100.0;
                }
            }
            "DI" => {
                if numbers.len() >= 2 {
                    self.char_angle_deg = numbers[1].atan2(numbers[0]).to_degrees();
                }
            }
            "DR" => {
                if numbers.len() >= 2 {
                    self.char_angle_deg = numbers[1].atan2(numbers[0]).to_degrees();
                }
            }
            "PE" => self.encoded_polyline(command)?,
            "BP" | "PG" | "PS" | "QL" | "LT" | "LA" | "WU" | "VS" | "FT" | "AC" | "RF" | "UL"
            | "MC" | "TR" | "SV" | "TD" | "DT" | "CS" | "CA" | "SS" | "SA" | "SD" | "AD" | "CF"
            | "ES" | "SL" | "LO" | "LM" | "CP" | "DV" | "SM" | "TL" | "XT" | "YT" | "AP" | "DC"
            | "DP" | "IM" | "CO" | "NR" | "AF" | "AH" => {}
            _ => {
                if self.options.strict {
                    return Err(ConversionError::Parse(format!(
                        "comando {name} não suportado no byte {}",
                        command.offset
                    )));
                }
                self.warn(&name, "comando não suportado");
            }
        }
        Ok(())
    }

    fn reset_state(&mut self) {
        self.flush_path(false);
        self.current = Point::default();
        self.absolute = true;
        self.pen_down = false;
        self.pen = 1;
        self.scale = None;
        self.rotation = 0;
        self.clip = None;
        self.polygon_mode = false;
        self.polygon.clear();
    }

    fn consume_points(&mut self, numbers: &[f64]) {
        for pair in numbers.chunks_exact(2) {
            let destination = if self.absolute {
                Point::new(pair[0], pair[1])
            } else {
                Point::new(self.current.x + pair[0], self.current.y + pair[1])
            };
            if self.pen_down {
                self.draw_to(destination);
            } else {
                self.current = destination;
            }
        }
    }

    fn draw_to(&mut self, destination: Point) {
        let start = self.to_mm(self.current);
        let end = self.to_mm(destination);
        self.current = destination;
        if self.polygon_mode {
            if self.polygon.is_empty() {
                self.polygon.push(start);
            }
            self.polygon.push(end);
            return;
        }
        self.add_segment(start, end);
    }

    fn add_segment(&mut self, start: Point, end: Point) {
        let Some((start, end)) = self.clip_segment(start, end) else {
            self.flush_path(false);
            return;
        };
        let contiguous = self
            .path
            .last()
            .is_some_and(|last| last.distance(start) <= 1e-8);
        if !contiguous {
            self.flush_path(false);
            self.path.push(start);
        }
        if self.path.is_empty() {
            self.path.push(start);
        }
        self.path.push(end);
    }

    fn flush_path(&mut self, closed: bool) {
        if self.path.len() >= 2 {
            let points = std::mem::take(&mut self.path);
            self.drawing.entities.push(Entity::Polyline {
                points,
                closed,
                pen: self.pen,
            });
        } else {
            self.path.clear();
        }
    }

    fn to_mm(&self, point: Point) -> Point {
        let plotter = if let Some(scale) = self.scale {
            let x_span = scale.xmax - scale.xmin;
            let y_span = scale.ymax - scale.ymin;
            Point::new(
                self.ip1.x + (point.x - scale.xmin) * (self.ip2.x - self.ip1.x) / x_span,
                self.ip1.y + (point.y - scale.ymin) * (self.ip2.y - self.ip1.y) / y_span,
            )
        } else {
            point
        };
        let rotated = match self.rotation.rem_euclid(360) {
            90 => Point::new(-plotter.y, plotter.x),
            180 => Point::new(-plotter.x, -plotter.y),
            270 => Point::new(plotter.y, -plotter.x),
            _ => plotter,
        };
        Point::new(
            rotated.x / self.options.units_per_mm,
            rotated.y / self.options.units_per_mm,
        )
    }

    fn set_input_points(&mut self, numbers: &[f64]) {
        self.flush_path(false);
        if numbers.len() >= 4 {
            self.ip1 = Point::new(numbers[0], numbers[1]);
            self.ip2 = Point::new(numbers[2], numbers[3]);
        } else {
            self.ip1 = Point::default();
            self.ip2 = Point::new(1016.0, 1016.0);
        }
    }

    fn set_relative_input_points(&mut self, numbers: &[f64]) {
        if numbers.len() >= 4 {
            let width = self.ip2.x - self.ip1.x;
            let height = self.ip2.y - self.ip1.y;
            let origin = self.ip1;
            self.ip1 = Point::new(
                origin.x + width * numbers[0] / 100.0,
                origin.y + height * numbers[1] / 100.0,
            );
            self.ip2 = Point::new(
                origin.x + width * numbers[2] / 100.0,
                origin.y + height * numbers[3] / 100.0,
            );
        }
    }

    fn set_scale(&mut self, numbers: &[f64]) {
        self.flush_path(false);
        self.scale = if numbers.len() >= 4
            && (numbers[1] - numbers[0]).abs() > f64::EPSILON
            && (numbers[3] - numbers[2]).abs() > f64::EPSILON
        {
            Some(Scale {
                xmin: numbers[0],
                xmax: numbers[1],
                ymin: numbers[2],
                ymax: numbers[3],
            })
        } else {
            if numbers.len() >= 4 {
                self.warn("SC", "intervalo de escala nulo foi ignorado");
            }
            None
        };
    }

    fn set_clip(&mut self, numbers: &[f64]) {
        self.flush_path(false);
        self.clip = if numbers.len() >= 4 {
            let first = self.to_mm(Point::new(numbers[0], numbers[1]));
            let second = self.to_mm(Point::new(numbers[2], numbers[3]));
            Some((
                Point::new(first.x.min(second.x), first.y.min(second.y)),
                Point::new(first.x.max(second.x), first.y.max(second.y)),
            ))
        } else {
            None
        };
    }

    fn clip_segment(&self, start: Point, end: Point) -> Option<(Point, Point)> {
        let Some((min, max)) = self.clip else {
            return Some((start, end));
        };
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let p = [-dx, dx, -dy, dy];
        let q = [
            start.x - min.x,
            max.x - start.x,
            start.y - min.y,
            max.y - start.y,
        ];
        let mut lower: f64 = 0.0;
        let mut upper: f64 = 1.0;
        for index in 0..4 {
            if p[index].abs() < f64::EPSILON {
                if q[index] < 0.0 {
                    return None;
                }
            } else {
                let ratio = q[index] / p[index];
                if p[index] < 0.0 {
                    lower = lower.max(ratio);
                } else {
                    upper = upper.min(ratio);
                }
            }
        }
        (lower <= upper).then(|| {
            (
                Point::new(start.x + lower * dx, start.y + lower * dy),
                Point::new(start.x + upper * dx, start.y + upper * dy),
            )
        })
    }

    fn circle(&mut self, numbers: &[f64]) {
        let Some(radius) = numbers.first().copied() else {
            return;
        };
        self.flush_path(false);
        let center = self.to_mm(self.current);
        let edge = self.to_mm(Point::new(self.current.x + radius, self.current.y));
        let radius_mm = center.distance(edge);
        if self.clip.is_none() && self.uniform_scale() {
            self.drawing.entities.push(Entity::Circle {
                center,
                radius: radius_mm,
                pen: self.pen,
            });
        } else {
            let points = sample_circle(center, radius_mm, self.options.curve_tolerance_mm);
            self.drawing.entities.push(Entity::Polyline {
                points,
                closed: true,
                pen: self.pen,
            });
        }
    }

    fn arc(&mut self, numbers: &[f64], relative_center: bool, _unused: bool) {
        if numbers.len() < 3 {
            return;
        }
        self.flush_path(false);
        let center_raw = if relative_center {
            Point::new(self.current.x + numbers[0], self.current.y + numbers[1])
        } else {
            Point::new(numbers[0], numbers[1])
        };
        let sweep = numbers[2];
        let start_raw = self.current;
        let vector = Point::new(start_raw.x - center_raw.x, start_raw.y - center_raw.y);
        let radians = sweep.to_radians();
        let end_raw = Point::new(
            center_raw.x + vector.x * radians.cos() - vector.y * radians.sin(),
            center_raw.y + vector.x * radians.sin() + vector.y * radians.cos(),
        );
        let center = self.to_mm(center_raw);
        let start = self.to_mm(start_raw);
        let radius = center.distance(start);
        if self.clip.is_none() && self.uniform_scale() && sweep.abs() < 360.0 {
            let start_angle = (start.y - center.y).atan2(start.x - center.x).to_degrees();
            let end_angle = start_angle + sweep;
            let (start_deg, end_deg) = if sweep < 0.0 {
                (end_angle, start_angle)
            } else {
                (start_angle, end_angle)
            };
            self.drawing.entities.push(Entity::Arc {
                center,
                radius,
                start_deg,
                end_deg,
                pen: self.pen,
            });
        } else {
            self.emit_arc_polyline(center_raw, start_raw, sweep);
        }
        self.current = end_raw;
    }

    fn emit_arc_polyline(&mut self, center: Point, start: Point, sweep_deg: f64) {
        let radius_mm = self.to_mm(center).distance(self.to_mm(start));
        let segments = arc_segments(radius_mm, sweep_deg, self.options.curve_tolerance_mm);
        let vector = Point::new(start.x - center.x, start.y - center.y);
        let mut previous = self.to_mm(start);
        for index in 1..=segments {
            let angle = sweep_deg.to_radians() * index as f64 / segments as f64;
            let raw = Point::new(
                center.x + vector.x * angle.cos() - vector.y * angle.sin(),
                center.y + vector.x * angle.sin() + vector.y * angle.cos(),
            );
            let next = self.to_mm(raw);
            self.add_segment(previous, next);
            previous = next;
        }
        self.flush_path(false);
    }

    fn arc_three_points(&mut self, numbers: &[f64], relative: bool) {
        if numbers.len() < 4 {
            return;
        }
        let start = self.current;
        let middle = if relative {
            Point::new(start.x + numbers[0], start.y + numbers[1])
        } else {
            Point::new(numbers[0], numbers[1])
        };
        let end = if relative {
            Point::new(start.x + numbers[2], start.y + numbers[3])
        } else {
            Point::new(numbers[2], numbers[3])
        };
        let Some(center) = circumcenter(start, middle, end) else {
            self.draw_to(end);
            return;
        };
        let start_angle = (start.y - center.y).atan2(start.x - center.x);
        let middle_angle = (middle.y - center.y).atan2(middle.x - center.x);
        let end_angle = (end.y - center.y).atan2(end.x - center.x);
        let mut sweep = normalize_angle(end_angle - start_angle);
        let middle_sweep = normalize_angle(middle_angle - start_angle);
        if middle_sweep > sweep {
            sweep -= 2.0 * PI;
        }
        self.flush_path(false);
        self.emit_arc_polyline(center, start, sweep.to_degrees());
        self.current = end;
    }

    fn bezier(&mut self, numbers: &[f64], relative: bool) {
        for values in numbers.chunks_exact(6) {
            let start = self.current;
            let point = |x: f64, y: f64| {
                if relative {
                    Point::new(start.x + x, start.y + y)
                } else {
                    Point::new(x, y)
                }
            };
            let control1 = point(values[0], values[1]);
            let control2 = point(values[2], values[3]);
            let end = point(values[4], values[5]);
            let estimate =
                start.distance(control1) + control1.distance(control2) + control2.distance(end);
            let estimate_mm = estimate / self.options.units_per_mm;
            let segments = ((estimate_mm / self.options.curve_tolerance_mm)
                .sqrt()
                .ceil() as usize)
                .clamp(4, 4096);
            let mut previous = self.to_mm(start);
            for index in 1..=segments {
                let t = index as f64 / segments as f64;
                let inv = 1.0 - t;
                let raw = Point::new(
                    inv.powi(3) * start.x
                        + 3.0 * inv.powi(2) * t * control1.x
                        + 3.0 * inv * t.powi(2) * control2.x
                        + t.powi(3) * end.x,
                    inv.powi(3) * start.y
                        + 3.0 * inv.powi(2) * t * control1.y
                        + 3.0 * inv * t.powi(2) * control2.y
                        + t.powi(3) * end.y,
                );
                let next = self.to_mm(raw);
                self.add_segment(previous, next);
                previous = next;
            }
            self.current = end;
        }
    }

    fn rectangle(&mut self, numbers: &[f64], relative: bool) {
        if numbers.len() < 2 {
            return;
        }
        self.flush_path(false);
        let start = self.current;
        let opposite = if relative {
            Point::new(start.x + numbers[0], start.y + numbers[1])
        } else {
            Point::new(numbers[0], numbers[1])
        };
        let raw = [
            start,
            Point::new(opposite.x, start.y),
            opposite,
            Point::new(start.x, opposite.y),
        ];
        let points = raw.into_iter().map(|point| self.to_mm(point)).collect();
        self.drawing.entities.push(Entity::Polyline {
            points,
            closed: true,
            pen: self.pen,
        });
    }

    fn wedge(&mut self, numbers: &[f64]) {
        if numbers.len() < 3 {
            return;
        }
        self.flush_path(false);
        let radius = numbers[0];
        let start_deg = numbers[1];
        let sweep_deg = numbers[2];
        let center = self.current;
        let segments = arc_segments(
            radius.abs() / self.options.units_per_mm,
            sweep_deg,
            self.options.curve_tolerance_mm,
        );
        let mut points = vec![self.to_mm(center)];
        for index in 0..=segments {
            let angle = (start_deg + sweep_deg * index as f64 / segments as f64).to_radians();
            points.push(self.to_mm(Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )));
        }
        self.drawing.entities.push(Entity::Polyline {
            points,
            closed: true,
            pen: self.pen,
        });
    }

    fn polygon_mode(&mut self, numbers: &[f64]) {
        let mode = numbers.first().copied().unwrap_or(0.0) as i32;
        match mode {
            0 => {
                self.flush_path(false);
                self.polygon_mode = true;
                self.polygon.clear();
                self.polygon.push(self.to_mm(self.current));
            }
            1 => {
                if !self.polygon.is_empty() {
                    self.emit_polygon();
                    self.polygon_mode = true;
                    self.polygon.push(self.to_mm(self.current));
                }
            }
            _ => self.polygon_mode = false,
        }
    }

    fn emit_polygon(&mut self) {
        if self.polygon.len() >= 2 {
            let points = std::mem::take(&mut self.polygon);
            self.drawing.entities.push(Entity::Polyline {
                points,
                closed: true,
                pen: self.pen,
            });
        }
    }

    fn label(&mut self, command: &Command) {
        self.flush_path(false);
        let value = String::from_utf8_lossy(&command.data)
            .replace('\r', "")
            .replace('\n', "\\P");
        if value.is_empty() {
            return;
        }
        self.drawing.entities.push(Entity::Text {
            position: self.to_mm(self.current),
            value,
            height: self.char_height_mm,
            rotation_deg: self.char_angle_deg,
            pen: self.pen,
        });
    }

    fn set_pen_color(&mut self, numbers: &[f64]) {
        if numbers.len() >= 4 {
            let pen = numbers[0].max(0.0) as u16;
            let style = self.pens.entry(pen).or_insert_with(|| PenStyle::new(pen));
            style.color_rgb = Some((
                numbers[1].clamp(0.0, 255.0) as u8,
                numbers[2].clamp(0.0, 255.0) as u8,
                numbers[3].clamp(0.0, 255.0) as u8,
            ));
        }
    }

    fn set_pen_width(&mut self, numbers: &[f64]) {
        if let Some(width) = numbers.first() {
            let pen = numbers.get(1).copied().unwrap_or(self.pen as f64).max(0.0) as u16;
            let style = self.pens.entry(pen).or_insert_with(|| PenStyle::new(pen));
            style.width_mm = Some(*width);
        }
    }

    fn encoded_polyline(&mut self, command: &Command) -> Result<(), ConversionError> {
        let data = &command.data;
        let mut index = 0;
        let mut absolute = false;
        let mut pen_up_next = false;
        let mut fractional_bits = 0_u32;
        let mut data_bits = 5_u32;

        while index < data.len() {
            match data[index] {
                b':' => {
                    index += 1;
                    let Some(value) = decode_pe_number(data, &mut index, data_bits) else {
                        return self.malformed_pe(command.offset);
                    };
                    self.flush_path(false);
                    self.pen = value.max(0) as u16;
                    self.pens
                        .entry(self.pen)
                        .or_insert_with(|| PenStyle::new(self.pen));
                }
                b'<' => {
                    pen_up_next = true;
                    index += 1;
                }
                b'=' => {
                    absolute = true;
                    index += 1;
                }
                b'>' => {
                    index += 1;
                    let Some(value) = decode_pe_number(data, &mut index, data_bits) else {
                        return self.malformed_pe(command.offset);
                    };
                    fractional_bits = value.clamp(0, 30) as u32;
                }
                b'7' => {
                    data_bits = 4;
                    index += 1;
                }
                byte if byte.is_ascii_whitespace() => index += 1,
                _ => {
                    let Some(x) = decode_pe_number(data, &mut index, data_bits) else {
                        return self.malformed_pe(command.offset);
                    };
                    let Some(y) = decode_pe_number(data, &mut index, data_bits) else {
                        return self.malformed_pe(command.offset);
                    };
                    let divisor = (1_u64 << fractional_bits) as f64;
                    let destination = if absolute {
                        Point::new(x as f64 / divisor, y as f64 / divisor)
                    } else {
                        Point::new(
                            self.current.x + x as f64 / divisor,
                            self.current.y + y as f64 / divisor,
                        )
                    };
                    if pen_up_next {
                        self.flush_path(false);
                        self.current = destination;
                        pen_up_next = false;
                    } else {
                        self.draw_to(destination);
                    }
                }
            }
        }
        Ok(())
    }

    fn malformed_pe(&mut self, offset: usize) -> Result<(), ConversionError> {
        if self.options.strict {
            Err(ConversionError::Parse(format!(
                "dados PE malformados no byte {offset}"
            )))
        } else {
            self.warn("PE", "dados compactados truncados ou inválidos");
            Ok(())
        }
    }

    fn uniform_scale(&self) -> bool {
        let Some(scale) = self.scale else {
            return true;
        };
        let x = ((self.ip2.x - self.ip1.x) / (scale.xmax - scale.xmin)).abs();
        let y = ((self.ip2.y - self.ip1.y) / (scale.ymax - scale.ymin)).abs();
        (x - y).abs() <= 1e-9
    }

    fn warn(&mut self, command: &str, message: &str) {
        let warning = format!("{command}: {message}");
        if self.warned.insert(warning.clone()) {
            self.warnings.push(warning);
        }
    }
}

fn arc_segments(radius: f64, sweep_deg: f64, tolerance: f64) -> usize {
    if radius <= tolerance {
        return 4;
    }
    let max_angle = 2.0 * (1.0 - tolerance.min(radius) / radius).acos();
    ((sweep_deg.to_radians().abs() / max_angle.max(0.001)).ceil() as usize).clamp(4, 8192)
}

fn sample_circle(center: Point, radius: f64, tolerance: f64) -> Vec<Point> {
    let segments = arc_segments(radius, 360.0, tolerance);
    (0..segments)
        .map(|index| {
            let angle = 2.0 * PI * index as f64 / segments as f64;
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect()
}

fn circumcenter(first: Point, second: Point, third: Point) -> Option<Point> {
    let denominator = 2.0
        * (first.x * (second.y - third.y)
            + second.x * (third.y - first.y)
            + third.x * (first.y - second.y));
    if denominator.abs() < 1e-12 {
        return None;
    }
    let first_sq = first.x.powi(2) + first.y.powi(2);
    let second_sq = second.x.powi(2) + second.y.powi(2);
    let third_sq = third.x.powi(2) + third.y.powi(2);
    Some(Point::new(
        (first_sq * (second.y - third.y)
            + second_sq * (third.y - first.y)
            + third_sq * (first.y - second.y))
            / denominator,
        (first_sq * (third.x - second.x)
            + second_sq * (first.x - third.x)
            + third_sq * (second.x - first.x))
            / denominator,
    ))
}

fn normalize_angle(angle: f64) -> f64 {
    angle.rem_euclid(2.0 * PI)
}

fn decode_pe_number(data: &[u8], index: &mut usize, data_bits: u32) -> Option<i64> {
    let continuation_bit = 1_u8 << data_bits;
    let data_mask = continuation_bit - 1;
    let mut encoded = 0_u64;
    let mut shift = 0_u32;
    loop {
        let byte = *data.get(*index)?;
        if !(63..=126).contains(&byte) {
            return None;
        }
        *index += 1;
        let value = byte - 63;
        encoded |= ((value & data_mask) as u64) << shift;
        if value & continuation_bit == 0 {
            break;
        }
        shift += data_bits;
        if shift >= 63 {
            return None;
        }
    }
    let magnitude = (encoded >> 1) as i64;
    Some(if encoded & 1 == 0 {
        magnitude
    } else {
        -magnitude - 1
    })
}

pub fn apply_output_transform(drawing: &mut Drawing, options: &ConversionOptions) {
    if !options.flip_y && !options.normalize_origin {
        return;
    }

    if options.flip_y {
        for entity in &mut drawing.entities {
            transform_entity(entity, |point| Point::new(point.x, -point.y));
            if let Entity::Arc {
                start_deg, end_deg, ..
            } = entity
            {
                let old_start = *start_deg;
                *start_deg = -*end_deg;
                *end_deg = -old_start;
            }
        }
    }

    if options.normalize_origin
        && let Some(bounds) = drawing.bounds()
    {
        let offset = bounds.min;
        for entity in &mut drawing.entities {
            transform_entity(entity, |point| {
                Point::new(point.x - offset.x, point.y - offset.y)
            });
        }
    }
}

fn transform_entity(entity: &mut Entity, transform: impl Fn(Point) -> Point) {
    match entity {
        Entity::Polyline { points, .. } => {
            for point in points {
                *point = transform(*point);
            }
        }
        Entity::Circle { center, .. }
        | Entity::Arc { center, .. }
        | Entity::Text {
            position: center, ..
        } => *center = transform(*center),
    }
}
