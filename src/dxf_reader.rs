use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use crate::model::{Drawing, Entity, PenStyle, Point};
use crate::{ConversionError, ConversionOptions};

#[derive(Clone, Debug)]
struct Pair {
    code: i32,
    value: String,
}

#[derive(Clone, Debug)]
struct Record {
    kind: String,
    pairs: Vec<Pair>,
}

#[derive(Clone, Debug)]
struct Block {
    base: Point,
    records: Vec<Record>,
}

#[derive(Clone, Copy, Debug)]
struct Transform {
    offset: Point,
    scale_x: f64,
    scale_y: f64,
    rotation_deg: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            offset: Point::default(),
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_deg: 0.0,
        }
    }
}

impl Transform {
    fn apply(self, point: Point) -> Point {
        let x = point.x * self.scale_x;
        let y = point.y * self.scale_y;
        let angle = self.rotation_deg.to_radians();
        Point::new(
            self.offset.x + x * angle.cos() - y * angle.sin(),
            self.offset.y + x * angle.sin() + y * angle.cos(),
        )
    }

    fn combine(self, child: Self) -> Self {
        let offset = self.apply(child.offset);
        Self {
            offset,
            scale_x: self.scale_x * child.scale_x,
            scale_y: self.scale_y * child.scale_y,
            rotation_deg: self.rotation_deg + child.rotation_deg,
        }
    }

    fn is_uniform(self) -> bool {
        (self.scale_x.abs() - self.scale_y.abs()).abs() < 1e-9
    }
}

#[derive(Default)]
struct DxfDocument {
    entities: Vec<Record>,
    blocks: BTreeMap<String, Block>,
    layer_colors: BTreeMap<String, i16>,
    units_to_mm: f64,
}

struct Reader<'a> {
    options: &'a ConversionOptions,
    document: DxfDocument,
    drawing: Drawing,
    warnings: Vec<String>,
    warned: BTreeSet<String>,
    pen_by_key: BTreeMap<String, u16>,
    next_pen: u16,
}

pub fn read(
    input: &[u8],
    options: &ConversionOptions,
) -> Result<(Drawing, usize, Vec<String>), ConversionError> {
    let pairs = parse_pairs(input)?;
    let document = parse_document(&pairs);
    let source_count = document.entities.len();
    let mut reader = Reader {
        options,
        document,
        drawing: Drawing::default(),
        warnings: Vec::new(),
        warned: BTreeSet::new(),
        pen_by_key: BTreeMap::new(),
        next_pen: 1,
    };
    let entities = reader.document.entities.clone();
    let units = reader.document.units_to_mm;
    reader.emit_records(
        &entities,
        Transform {
            scale_x: units,
            scale_y: units,
            ..Transform::default()
        },
        0,
    )?;
    reader.drawing.pens.sort_by_key(|style| style.number);
    Ok((reader.drawing, source_count, reader.warnings))
}

fn parse_pairs(input: &[u8]) -> Result<Vec<Pair>, ConversionError> {
    let text = String::from_utf8_lossy(input);
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 2 {
        return Err(ConversionError::Parse("DXF vazio ou truncado".into()));
    }
    let mut pairs = Vec::with_capacity(lines.len() / 2);
    for chunk in lines.chunks(2) {
        if chunk.len() != 2 {
            break;
        }
        let code = chunk[0].trim().parse::<i32>().map_err(|_| {
            ConversionError::Parse(format!("group code DXF inválido: {}", chunk[0]))
        })?;
        pairs.push(Pair {
            code,
            value: chunk[1].trim_end_matches('\r').to_owned(),
        });
    }
    if !pairs
        .iter()
        .any(|pair| pair.code == 0 && pair.value == "SECTION")
    {
        return Err(ConversionError::Parse(
            "arquivo não parece ser um DXF ASCII".into(),
        ));
    }
    Ok(pairs)
}

fn parse_document(pairs: &[Pair]) -> DxfDocument {
    let mut document = DxfDocument {
        units_to_mm: 1.0,
        ..DxfDocument::default()
    };
    let mut index = 0;
    while index + 1 < pairs.len() {
        if pairs[index].code == 0 && pairs[index].value == "SECTION" && pairs[index + 1].code == 2 {
            let section = pairs[index + 1].value.as_str();
            index += 2;
            let start = index;
            while index < pairs.len() && !(pairs[index].code == 0 && pairs[index].value == "ENDSEC")
            {
                index += 1;
            }
            match section {
                "HEADER" => parse_header(&pairs[start..index], &mut document),
                "TABLES" => parse_layers(&pairs[start..index], &mut document),
                "BLOCKS" => parse_blocks(&pairs[start..index], &mut document),
                "ENTITIES" => document.entities = records(&pairs[start..index]),
                _ => {}
            }
        }
        index += 1;
    }
    document
}

fn parse_header(pairs: &[Pair], document: &mut DxfDocument) {
    for window in pairs.windows(2) {
        if window[0].code == 9 && window[0].value == "$INSUNITS" {
            let units = window[1].value.trim().parse::<i32>().unwrap_or(0);
            document.units_to_mm = match units {
                1 => 25.4,
                2 => 304.8,
                3 => 1609_344.0,
                4 => 1.0,
                5 => 10.0,
                6 => 1000.0,
                7 => 1_000_000.0,
                8 => 0.000_025_4,
                9 => 0.0254,
                10 => 914.4,
                11 => 1.0e-7,
                12 => 1.0e-6,
                13 => 0.001,
                14 => 100.0,
                _ => 1.0,
            };
        }
    }
}

fn parse_layers(pairs: &[Pair], document: &mut DxfDocument) {
    for record in records(pairs) {
        if record.kind == "LAYER" {
            let name = string_value(&record, 2).unwrap_or_else(|| "0".into());
            let color = int_value(&record, 62).unwrap_or(7) as i16;
            document.layer_colors.insert(name, color.abs());
        }
    }
}

fn parse_blocks(pairs: &[Pair], document: &mut DxfDocument) {
    let mut index = 0;
    while index < pairs.len() {
        if pairs[index].code == 0 && pairs[index].value == "BLOCK" {
            let header_start = index + 1;
            index += 1;
            while index < pairs.len() && pairs[index].code != 0 {
                index += 1;
            }
            let header = Record {
                kind: "BLOCK".into(),
                pairs: pairs[header_start..index].to_vec(),
            };
            let name = string_value(&header, 2)
                .or_else(|| string_value(&header, 3))
                .unwrap_or_default();
            let base = point_value(&header, 10, 20).unwrap_or_default();
            let entity_start = index;
            while index < pairs.len() && !(pairs[index].code == 0 && pairs[index].value == "ENDBLK")
            {
                index += 1;
            }
            document.blocks.insert(
                name,
                Block {
                    base,
                    records: records(&pairs[entity_start..index]),
                },
            );
        }
        index += 1;
    }
}

fn records(pairs: &[Pair]) -> Vec<Record> {
    let mut output = Vec::new();
    let mut index = 0;
    while index < pairs.len() {
        if pairs[index].code != 0 {
            index += 1;
            continue;
        }
        let kind = pairs[index].value.clone();
        index += 1;
        let start = index;
        while index < pairs.len() && pairs[index].code != 0 {
            index += 1;
        }
        output.push(Record {
            kind,
            pairs: pairs[start..index].to_vec(),
        });
    }
    output
}

impl Reader<'_> {
    fn emit_records(
        &mut self,
        records: &[Record],
        transform: Transform,
        depth: usize,
    ) -> Result<(), ConversionError> {
        if depth > 16 {
            return self.unsupported("INSERT", "profundidade máxima de blocos excedida");
        }
        let mut index = 0;
        while index < records.len() {
            let record = &records[index];
            if record.kind == "POLYLINE" {
                let start = index + 1;
                index += 1;
                while index < records.len() && records[index].kind != "SEQEND" {
                    index += 1;
                }
                self.emit_polyline(record, &records[start..index], transform);
            } else {
                self.emit_record(record, transform, depth)?;
            }
            index += 1;
        }
        Ok(())
    }

    fn emit_record(
        &mut self,
        record: &Record,
        transform: Transform,
        depth: usize,
    ) -> Result<(), ConversionError> {
        let pen = self.pen_for(record);
        match record.kind.as_str() {
            "LINE" => {
                if let (Some(start), Some(end)) =
                    (point_value(record, 10, 20), point_value(record, 11, 21))
                {
                    self.polyline(
                        vec![transform.apply(start), transform.apply(end)],
                        false,
                        pen,
                    );
                }
            }
            "CIRCLE" => self.emit_circle(record, transform, pen),
            "ARC" => self.emit_arc(record, transform, pen),
            "LWPOLYLINE" => self.emit_lwpolyline(record, transform, pen),
            "ELLIPSE" => self.emit_ellipse(record, transform, pen),
            "SPLINE" => self.emit_spline(record, transform, pen),
            "POINT" => {
                if let Some(center) = point_value(record, 10, 20) {
                    self.drawing.entities.push(Entity::Circle {
                        center: transform.apply(center),
                        radius: self.options.curve_tolerance_mm.max(0.05),
                        pen,
                    });
                }
            }
            "TEXT" | "MTEXT" | "ATTRIB" | "ATTDEF" => self.emit_text(record, transform, pen),
            "SOLID" | "TRACE" | "3DFACE" => self.emit_solid(record, transform, pen),
            "INSERT" => self.emit_insert(record, transform, depth)?,
            "DIMENSION" => {
                self.warn("DIMENSION", "cotas foram ignoradas");
            }
            "HATCH" => {
                self.warn(
                    "HATCH",
                    "preenchimento foi ignorado; use o contorno original",
                );
            }
            "SEQEND" | "VERTEX" | "ENDBLK" => {}
            kind => self.unsupported(kind, "entidade DXF não suportada")?,
        }
        Ok(())
    }

    fn emit_circle(&mut self, record: &Record, transform: Transform, pen: u16) {
        let (Some(center), Some(radius)) = (point_value(record, 10, 20), number_value(record, 40))
        else {
            return;
        };
        let center = transform.apply(center);
        if transform.is_uniform() {
            self.drawing.entities.push(Entity::Circle {
                center,
                radius: radius.abs() * transform.scale_x.abs(),
                pen,
            });
        } else {
            let points = sample_parametric(0.0, 2.0 * PI, |angle| {
                transform.apply(Point::new(
                    number_value(record, 10).unwrap_or(0.0) + radius * angle.cos(),
                    number_value(record, 20).unwrap_or(0.0) + radius * angle.sin(),
                ))
            });
            self.polyline(points, true, pen);
        }
    }

    fn emit_arc(&mut self, record: &Record, transform: Transform, pen: u16) {
        let (Some(center), Some(radius), Some(start), Some(end)) = (
            point_value(record, 10, 20),
            number_value(record, 40),
            number_value(record, 50),
            number_value(record, 51),
        ) else {
            return;
        };
        let sweep = (end - start).rem_euclid(360.0);
        if transform.is_uniform() && transform.scale_x > 0.0 && transform.scale_y > 0.0 {
            self.drawing.entities.push(Entity::Arc {
                center: transform.apply(center),
                radius: radius.abs() * transform.scale_x.abs(),
                start_deg: start + transform.rotation_deg,
                end_deg: start + transform.rotation_deg + sweep,
                pen,
            });
        } else {
            let points =
                sample_parametric(start.to_radians(), (start + sweep).to_radians(), |angle| {
                    transform.apply(Point::new(
                        center.x + radius * angle.cos(),
                        center.y + radius * angle.sin(),
                    ))
                });
            self.polyline(points, false, pen);
        }
    }

    fn emit_lwpolyline(&mut self, record: &Record, transform: Transform, pen: u16) {
        let vertices = repeated_vertices(record, 10, 20, 42);
        let closed = int_value(record, 70).unwrap_or(0) & 1 != 0;
        let points = expand_bulges(&vertices, closed)
            .into_iter()
            .map(|point| transform.apply(point))
            .collect();
        self.polyline(points, closed, pen);
    }

    fn emit_polyline(&mut self, header: &Record, vertices: &[Record], transform: Transform) {
        let pen = self.pen_for(header);
        let vertices: Vec<(Point, f64)> = vertices
            .iter()
            .filter(|record| record.kind == "VERTEX")
            .filter_map(|record| {
                point_value(record, 10, 20)
                    .map(|point| (point, number_value(record, 42).unwrap_or(0.0)))
            })
            .collect();
        let closed = int_value(header, 70).unwrap_or(0) & 1 != 0;
        let points = expand_bulges(&vertices, closed)
            .into_iter()
            .map(|point| transform.apply(point))
            .collect();
        self.polyline(points, closed, pen);
    }

    fn emit_ellipse(&mut self, record: &Record, transform: Transform, pen: u16) {
        let (Some(center), Some(major), Some(ratio)) = (
            point_value(record, 10, 20),
            point_value(record, 11, 21),
            number_value(record, 40),
        ) else {
            return;
        };
        let start = number_value(record, 41).unwrap_or(0.0);
        let end = number_value(record, 42).unwrap_or(2.0 * PI);
        let minor = Point::new(-major.y * ratio, major.x * ratio);
        let points = sample_parametric(start, end, |parameter| {
            transform.apply(Point::new(
                center.x + major.x * parameter.cos() + minor.x * parameter.sin(),
                center.y + major.y * parameter.cos() + minor.y * parameter.sin(),
            ))
        });
        self.polyline(points, (end - start).abs() >= 2.0 * PI - 1e-6, pen);
    }

    fn emit_spline(&mut self, record: &Record, transform: Transform, pen: u16) {
        let mut points = repeated_points(record, 11, 21);
        if points.len() < 2 {
            points = repeated_points(record, 10, 20);
        }
        if points.len() >= 2 {
            let points = catmull_rom(&points, self.options.curve_tolerance_mm)
                .into_iter()
                .map(|point| transform.apply(point))
                .collect();
            self.polyline(points, int_value(record, 70).unwrap_or(0) & 1 != 0, pen);
        } else {
            self.warn("SPLINE", "spline sem pontos suficientes foi ignorada");
        }
    }

    fn emit_text(&mut self, record: &Record, transform: Transform, pen: u16) {
        let Some(position) = point_value(record, 10, 20) else {
            return;
        };
        let mut value = values(record, 3).join("");
        value.push_str(&values(record, 1).join(""));
        value = strip_mtext_formatting(&value);
        if value.is_empty() {
            return;
        }
        self.drawing.entities.push(Entity::Text {
            position: transform.apply(position),
            value,
            height: number_value(record, 40).unwrap_or(2.5) * transform.scale_y.abs(),
            rotation_deg: number_value(record, 50).unwrap_or(0.0) + transform.rotation_deg,
            pen,
        });
    }

    fn emit_solid(&mut self, record: &Record, transform: Transform, pen: u16) {
        let mut points = Vec::new();
        let order: &[i32] = if record.kind == "3DFACE" {
            &[0, 1, 2, 3]
        } else {
            &[0, 1, 3, 2]
        };
        for index in order {
            if let Some(point) = point_value(record, 10 + index, 20 + index) {
                let point = transform.apply(point);
                if points
                    .last()
                    .is_none_or(|last: &Point| last.distance(point) > 1e-9)
                {
                    points.push(point);
                }
            }
        }
        self.polyline(points, true, pen);
    }

    fn emit_insert(
        &mut self,
        record: &Record,
        transform: Transform,
        depth: usize,
    ) -> Result<(), ConversionError> {
        let Some(name) = string_value(record, 2) else {
            return Ok(());
        };
        let Some(block) = self.document.blocks.get(&name).cloned() else {
            return self.unsupported("INSERT", &format!("bloco {name} não encontrado"));
        };
        let insertion = point_value(record, 10, 20).unwrap_or_default();
        let scale_x = number_value(record, 41).unwrap_or(1.0);
        let scale_y = number_value(record, 42).unwrap_or(1.0);
        let rotation_deg = number_value(record, 50).unwrap_or(0.0);
        let angle = rotation_deg.to_radians();
        let base_x = block.base.x * scale_x;
        let base_y = block.base.y * scale_y;
        let transformed_base = Point::new(
            base_x * angle.cos() - base_y * angle.sin(),
            base_x * angle.sin() + base_y * angle.cos(),
        );
        let child = Transform {
            offset: Point::new(
                insertion.x - transformed_base.x,
                insertion.y - transformed_base.y,
            ),
            scale_x,
            scale_y,
            rotation_deg,
        };
        self.emit_records(&block.records, transform.combine(child), depth + 1)
    }

    fn polyline(&mut self, points: Vec<Point>, closed: bool, pen: u16) {
        if points.len() >= 2 {
            self.drawing.entities.push(Entity::Polyline {
                points,
                closed,
                pen,
            });
        }
    }

    fn pen_for(&mut self, record: &Record) -> u16 {
        let layer = string_value(record, 8).unwrap_or_else(|| "0".into());
        let explicit_color = int_value(record, 62).map(|value| value.abs() as i16);
        let color = explicit_color
            .or_else(|| self.document.layer_colors.get(&layer).copied())
            .unwrap_or(7);
        let key = format!("{layer}:{color}");
        if let Some(pen) = self.pen_by_key.get(&key) {
            return *pen;
        }
        let pen = self.next_pen.min(255);
        self.next_pen = self.next_pen.saturating_add(1);
        self.pen_by_key.insert(key, pen);
        let mut style = PenStyle::new(pen);
        style.color_rgb = Some(aci_to_rgb(color));
        self.drawing.pens.push(style);
        pen
    }

    fn unsupported(&mut self, kind: &str, detail: &str) -> Result<(), ConversionError> {
        if self.options.strict {
            Err(ConversionError::Parse(format!("{kind}: {detail}")))
        } else {
            self.warn(kind, detail);
            Ok(())
        }
    }

    fn warn(&mut self, kind: &str, detail: &str) {
        let warning = format!("{kind}: {detail}");
        if self.warned.insert(warning.clone()) {
            self.warnings.push(warning);
        }
    }
}

fn number_value(record: &Record, code: i32) -> Option<f64> {
    record
        .pairs
        .iter()
        .find(|pair| pair.code == code)
        .and_then(|pair| pair.value.trim().parse().ok())
}

fn int_value(record: &Record, code: i32) -> Option<i32> {
    record
        .pairs
        .iter()
        .find(|pair| pair.code == code)
        .and_then(|pair| pair.value.trim().parse().ok())
}

fn string_value(record: &Record, code: i32) -> Option<String> {
    record
        .pairs
        .iter()
        .find(|pair| pair.code == code)
        .map(|pair| pair.value.clone())
}

fn values(record: &Record, code: i32) -> Vec<String> {
    record
        .pairs
        .iter()
        .filter(|pair| pair.code == code)
        .map(|pair| pair.value.clone())
        .collect()
}

fn point_value(record: &Record, x_code: i32, y_code: i32) -> Option<Point> {
    Some(Point::new(
        number_value(record, x_code)?,
        number_value(record, y_code)?,
    ))
}

fn repeated_points(record: &Record, x_code: i32, y_code: i32) -> Vec<Point> {
    let mut points = Vec::new();
    let mut current_x = None;
    for pair in &record.pairs {
        if pair.code == x_code {
            current_x = pair.value.trim().parse::<f64>().ok();
        } else if pair.code == y_code
            && let (Some(x), Ok(y)) = (current_x.take(), pair.value.trim().parse::<f64>())
        {
            points.push(Point::new(x, y));
        }
    }
    points
}

fn repeated_vertices(
    record: &Record,
    x_code: i32,
    y_code: i32,
    bulge_code: i32,
) -> Vec<(Point, f64)> {
    let mut vertices = Vec::new();
    let mut current_x = None;
    for pair in &record.pairs {
        match pair.code {
            code if code == x_code => current_x = pair.value.trim().parse::<f64>().ok(),
            code if code == y_code => {
                if let (Some(x), Ok(y)) = (current_x.take(), pair.value.trim().parse::<f64>()) {
                    vertices.push((Point::new(x, y), 0.0));
                }
            }
            code if code == bulge_code => {
                if let Some((_, bulge)) = vertices.last_mut() {
                    *bulge = pair.value.trim().parse().unwrap_or(0.0);
                }
            }
            _ => {}
        }
    }
    vertices
}

fn expand_bulges(vertices: &[(Point, f64)], closed: bool) -> Vec<Point> {
    if vertices.len() < 2 {
        return vertices.iter().map(|(point, _)| *point).collect();
    }
    let mut output = Vec::new();
    let segment_count = if closed {
        vertices.len()
    } else {
        vertices.len() - 1
    };
    for index in 0..segment_count {
        let (start, bulge) = vertices[index];
        let end = vertices[(index + 1) % vertices.len()].0;
        output.push(start);
        if bulge.abs() > 1e-12 {
            output.extend(sample_bulge(start, end, bulge).into_iter().skip(1));
        }
    }
    if !closed {
        output.push(vertices.last().unwrap().0);
    }
    output
}

fn sample_bulge(start: Point, end: Point, bulge: f64) -> Vec<Point> {
    let chord = start.distance(end);
    if chord <= f64::EPSILON {
        return vec![start, end];
    }
    let sweep = 4.0 * bulge.atan();
    let radius = chord * (1.0 + bulge * bulge) / (4.0 * bulge.abs());
    let midpoint = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    let distance = radius * (1.0 - 2.0 * bulge * bulge / (1.0 + bulge * bulge));
    let dx = (end.x - start.x) / chord;
    let dy = (end.y - start.y) / chord;
    let center = Point::new(
        midpoint.x - dy * distance * bulge.signum(),
        midpoint.y + dx * distance * bulge.signum(),
    );
    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    sample_parametric(start_angle, start_angle + sweep, |angle| {
        Point::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        )
    })
}

fn sample_parametric(start: f64, end: f64, sample: impl Fn(f64) -> Point) -> Vec<Point> {
    let segments = (((end - start).abs() / (PI / 36.0)).ceil() as usize).clamp(4, 4096);
    (0..=segments)
        .map(|index| sample(start + (end - start) * index as f64 / segments as f64))
        .collect()
}

fn catmull_rom(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut output = vec![points[0]];
    for index in 0..points.len() - 1 {
        let p0 = points[index.saturating_sub(1)];
        let p1 = points[index];
        let p2 = points[index + 1];
        let p3 = points[(index + 2).min(points.len() - 1)];
        let length = p1.distance(p2);
        let segments = ((length / tolerance.max(0.01)).sqrt().ceil() as usize).clamp(4, 128);
        for step in 1..=segments {
            let t = step as f64 / segments as f64;
            let t2 = t * t;
            let t3 = t2 * t;
            output.push(Point::new(
                0.5 * ((2.0 * p1.x)
                    + (-p0.x + p2.x) * t
                    + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
                    + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3),
                0.5 * ((2.0 * p1.y)
                    + (-p0.y + p2.y) * t
                    + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
                    + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3),
            ));
        }
    }
    output
}

fn strip_mtext_formatting(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\\' => match chars.next() {
                Some('P') => output.push('\n'),
                Some(next) => output.push(next),
                None => {}
            },
            '{' | '}' => {}
            ';' if output.ends_with('\\') => {}
            _ => output.push(character),
        }
    }
    output
}

fn aci_to_rgb(color: i16) -> (u8, u8, u8) {
    match color {
        1 => (255, 0, 0),
        2 => (255, 255, 0),
        3 => (0, 255, 0),
        4 => (0, 255, 255),
        5 => (0, 0, 255),
        6 => (255, 0, 255),
        7 => (255, 255, 255),
        8 => (128, 128, 128),
        9 => (192, 192, 192),
        _ => (255, 255, 255),
    }
}
