use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use crate::model::{Drawing, Entity, PenStyle, Point};
use crate::{ConversionError, ConversionOptions};

#[derive(Clone, Copy, Debug)]
struct Matrix {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Matrix {
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    fn apply(self, point: Point) -> Point {
        Point::new(
            self.a * point.x + self.c * point.y + self.e,
            self.b * point.x + self.d * point.y + self.f,
        )
    }

    fn multiply(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    fn scale_magnitude(self) -> f64 {
        ((self.a.hypot(self.b) + self.c.hypot(self.d)) / 2.0).abs()
    }
}

#[derive(Clone, Debug)]
struct Tag {
    name: String,
    attributes: BTreeMap<String, String>,
    closing: bool,
    self_closing: bool,
    text: String,
}

struct Reader<'a> {
    options: &'a ConversionOptions,
    drawing: Drawing,
    warnings: Vec<String>,
    warned: BTreeSet<String>,
    pens: BTreeMap<String, u16>,
    next_pen: u16,
    root_matrix: Matrix,
}

pub fn read(
    input: &[u8],
    options: &ConversionOptions,
) -> Result<(Drawing, usize, Vec<String>), ConversionError> {
    let text = String::from_utf8_lossy(input);
    if !text.to_ascii_lowercase().contains("<svg") {
        return Err(ConversionError::Parse(
            "file does not appear to be SVG XML".into(),
        ));
    }
    let tags = parse_tags(&text)?;
    let root = tags
        .iter()
        .find(|tag| tag.name == "svg" && !tag.closing)
        .ok_or_else(|| ConversionError::Parse("<svg> element not found".into()))?;
    let root_matrix = root_matrix(root);
    let mut reader = Reader {
        options,
        drawing: Drawing::default(),
        warnings: Vec::new(),
        warned: BTreeSet::new(),
        pens: BTreeMap::new(),
        next_pen: 1,
        root_matrix,
    };
    reader.emit_tags(&tags, root)?;
    reader.drawing.pens.sort_by_key(|style| style.number);
    Ok((reader.drawing, tags.len(), reader.warnings))
}

fn parse_tags(xml: &str) -> Result<Vec<Tag>, ConversionError> {
    let mut tags = Vec::new();
    let mut index = 0;
    while let Some(relative_start) = xml[index..].find('<') {
        let start = index + relative_start;
        if xml[start..].starts_with("<!--") {
            let end = xml[start + 4..]
                .find("-->")
                .ok_or_else(|| ConversionError::Parse("truncated SVG comment".into()))?;
            index = start + 4 + end + 3;
            continue;
        }
        let Some(relative_end) = xml[start..].find('>') else {
            return Err(ConversionError::Parse("truncated SVG tag".into()));
        };
        let end = start + relative_end;
        let raw = xml[start + 1..end].trim();
        index = end + 1;
        if raw.starts_with('?') || raw.starts_with('!') {
            continue;
        }
        let closing = raw.starts_with('/');
        let body = raw.trim_start_matches('/').trim();
        let self_closing = body.ends_with('/');
        let body = body.trim_end_matches('/').trim();
        let name_end = body
            .find(|character: char| character.is_ascii_whitespace())
            .unwrap_or(body.len());
        let name = body[..name_end]
            .rsplit(':')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let attributes = parse_attributes(&body[name_end..]);
        let text_content = if !closing && !self_closing && name == "text" {
            xml[index..]
                .find('<')
                .map(|length| decode_xml(xml[index..index + length].trim()))
                .unwrap_or_default()
        } else {
            String::new()
        };
        tags.push(Tag {
            name,
            attributes,
            closing,
            self_closing,
            text: text_content,
        });
    }
    Ok(tags)
}

fn parse_attributes(input: &str) -> BTreeMap<String, String> {
    let mut attributes = BTreeMap::new();
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let start = index;
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() && bytes[index] != b'=' {
            index += 1;
        }
        if start == index {
            index += 1;
            continue;
        }
        let key = input[start..index]
            .rsplit(':')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            continue;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let quote = bytes.get(index).copied();
        let value = if quote == Some(b'"') || quote == Some(b'\'') {
            index += 1;
            let value_start = index;
            while index < bytes.len() && Some(bytes[index]) != quote {
                index += 1;
            }
            let value = decode_xml(&input[value_start..index]);
            index += usize::from(index < bytes.len());
            value
        } else {
            let value_start = index;
            while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
                index += 1;
            }
            input[value_start..index].to_owned()
        };
        attributes.insert(key, value);
    }
    attributes
}

impl Reader<'_> {
    fn emit_tags(&mut self, tags: &[Tag], root: &Tag) -> Result<(), ConversionError> {
        let mut matrices = vec![self.root_matrix];
        let mut styles: Vec<BTreeMap<String, String>> = vec![merged_style(None, &root.attributes)];
        let mut ignored_depth = 0_usize;
        for tag in tags {
            if tag.name == "svg" {
                continue;
            }
            if tag.closing && tag.name == "defs" {
                ignored_depth = ignored_depth.saturating_sub(1);
                continue;
            }
            if ignored_depth > 0 {
                if !tag.closing && tag.name == "defs" {
                    ignored_depth += 1;
                }
                continue;
            }
            if !tag.closing && tag.name == "defs" {
                ignored_depth = 1;
                continue;
            }
            if tag.closing {
                if matches!(tag.name.as_str(), "g" | "a" | "symbol") {
                    matrices.pop();
                    styles.pop();
                }
                continue;
            }
            let parent_matrix = *matrices.last().unwrap_or(&self.root_matrix);
            let matrix = parent_matrix.multiply(parse_transform(
                tag.attributes
                    .get("transform")
                    .map(String::as_str)
                    .unwrap_or(""),
            ));
            let style = merged_style(styles.last(), &tag.attributes);
            if matches!(tag.name.as_str(), "g" | "a" | "symbol") {
                matrices.push(matrix);
                styles.push(style);
                if tag.self_closing {
                    matrices.pop();
                    styles.pop();
                }
                continue;
            }
            if is_hidden(&style) {
                continue;
            }
            let pen = self.pen_for(&style, &tag.attributes);
            match tag.name.as_str() {
                "line" => self.line(tag, matrix, pen),
                "polyline" => self.poly_points(tag, matrix, pen, false),
                "polygon" => self.poly_points(tag, matrix, pen, true),
                "rect" => self.rect(tag, matrix, pen),
                "circle" => self.circle(tag, matrix, pen),
                "ellipse" => self.ellipse(tag, matrix, pen),
                "path" => self.path(tag, matrix, pen)?,
                "text" => self.text(tag, matrix, pen),
                "svg" | "defs" | "title" | "desc" | "metadata" | "style" => {}
                "use" => self.warn("use", "<use> references are not expanded"),
                name => self.unsupported(name)?,
            }
        }
        Ok(())
    }

    fn line(&mut self, tag: &Tag, matrix: Matrix, pen: u16) {
        let start = Point::new(attr_number(tag, "x1", 0.0), attr_number(tag, "y1", 0.0));
        let end = Point::new(attr_number(tag, "x2", 0.0), attr_number(tag, "y2", 0.0));
        self.polyline(vec![matrix.apply(start), matrix.apply(end)], false, pen);
    }

    fn poly_points(&mut self, tag: &Tag, matrix: Matrix, pen: u16, closed: bool) {
        let numbers = parse_numbers(
            tag.attributes
                .get("points")
                .map(String::as_str)
                .unwrap_or(""),
        );
        let points = numbers
            .chunks_exact(2)
            .map(|pair| matrix.apply(Point::new(pair[0], pair[1])))
            .collect();
        self.polyline(points, closed, pen);
    }

    fn rect(&mut self, tag: &Tag, matrix: Matrix, pen: u16) {
        let x = attr_number(tag, "x", 0.0);
        let y = attr_number(tag, "y", 0.0);
        let width = attr_number(tag, "width", 0.0);
        let height = attr_number(tag, "height", 0.0);
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let rx = attr_number(tag, "rx", 0.0).min(width / 2.0);
        let ry = attr_number(tag, "ry", rx).min(height / 2.0);
        if rx <= 0.0 || ry <= 0.0 {
            self.polyline(
                [
                    Point::new(x, y),
                    Point::new(x + width, y),
                    Point::new(x + width, y + height),
                    Point::new(x, y + height),
                ]
                .into_iter()
                .map(|point| matrix.apply(point))
                .collect(),
                true,
                pen,
            );
            return;
        }
        let mut points = Vec::new();
        for (center, start) in [
            (Point::new(x + width - rx, y + ry), -PI / 2.0),
            (Point::new(x + width - rx, y + height - ry), 0.0),
            (Point::new(x + rx, y + height - ry), PI / 2.0),
            (Point::new(x + rx, y + ry), PI),
        ] {
            for step in 0..=9 {
                let angle = start + PI / 2.0 * step as f64 / 9.0;
                points.push(matrix.apply(Point::new(
                    center.x + rx * angle.cos(),
                    center.y + ry * angle.sin(),
                )));
            }
        }
        self.polyline(points, true, pen);
    }

    fn circle(&mut self, tag: &Tag, matrix: Matrix, pen: u16) {
        let center = Point::new(attr_number(tag, "cx", 0.0), attr_number(tag, "cy", 0.0));
        let radius = attr_number(tag, "r", 0.0);
        if radius <= 0.0 {
            return;
        }
        if is_uniform_rotation(matrix) {
            self.drawing.entities.push(Entity::Circle {
                center: matrix.apply(center),
                radius: radius * matrix.scale_magnitude(),
                pen,
            });
        } else {
            self.polyline(sample_ellipse(center, radius, radius, matrix), true, pen);
        }
    }

    fn ellipse(&mut self, tag: &Tag, matrix: Matrix, pen: u16) {
        let center = Point::new(attr_number(tag, "cx", 0.0), attr_number(tag, "cy", 0.0));
        let rx = attr_number(tag, "rx", 0.0);
        let ry = attr_number(tag, "ry", 0.0);
        if rx > 0.0 && ry > 0.0 {
            self.polyline(sample_ellipse(center, rx, ry, matrix), true, pen);
        }
    }

    fn path(&mut self, tag: &Tag, matrix: Matrix, pen: u16) -> Result<(), ConversionError> {
        let data = tag.attributes.get("d").map(String::as_str).unwrap_or("");
        match parse_path(data, matrix, self.options.curve_tolerance_mm) {
            Ok(paths) => {
                for (points, closed) in paths {
                    self.polyline(points, closed, pen);
                }
                Ok(())
            }
            Err(message) if self.options.strict => Err(ConversionError::Parse(message)),
            Err(message) => {
                self.warn("path", &message);
                Ok(())
            }
        }
    }

    fn text(&mut self, tag: &Tag, matrix: Matrix, pen: u16) {
        if tag.text.is_empty() {
            return;
        }
        let position = matrix.apply(Point::new(
            attr_number(tag, "x", 0.0),
            attr_number(tag, "y", 0.0),
        ));
        let height = tag
            .attributes
            .get("font-size")
            .map(|value| length_to_user(value))
            .unwrap_or(12.0)
            * matrix.scale_magnitude();
        let rotation = matrix.b.atan2(matrix.a).to_degrees();
        self.drawing.entities.push(Entity::Text {
            position,
            value: tag.text.clone(),
            height,
            rotation_deg: rotation,
            pen,
        });
    }

    fn pen_for(
        &mut self,
        style: &BTreeMap<String, String>,
        attributes: &BTreeMap<String, String>,
    ) -> u16 {
        if let Some(number) = attributes
            .get("data-pen")
            .and_then(|value| value.parse::<u16>().ok())
        {
            self.ensure_pen(number, parse_color(style), stroke_width(style));
            return number;
        }
        let color = parse_color(style).unwrap_or((0, 0, 0));
        let width = stroke_width(style);
        let key = format!("{:02x}{:02x}{:02x}:{:.6}", color.0, color.1, color.2, width);
        if let Some(pen) = self.pens.get(&key) {
            return *pen;
        }
        let pen = self.next_pen.min(255);
        self.next_pen = self.next_pen.saturating_add(1);
        self.pens.insert(key, pen);
        self.ensure_pen(pen, Some(color), width);
        pen
    }

    fn ensure_pen(&mut self, number: u16, color: Option<(u8, u8, u8)>, width: f64) {
        if let Some(style) = self
            .drawing
            .pens
            .iter_mut()
            .find(|style| style.number == number)
        {
            if style.color_rgb.is_none() {
                style.color_rgb = color;
            }
            return;
        }
        let mut style = PenStyle::new(number);
        style.color_rgb = color;
        style.width_mm = (width > 0.0).then_some(width);
        self.drawing.pens.push(style);
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

    fn unsupported(&mut self, name: &str) -> Result<(), ConversionError> {
        if self.options.strict {
            Err(ConversionError::Parse(format!(
                "unsupported SVG element <{name}>"
            )))
        } else {
            self.warn(name, "unsupported SVG element");
            Ok(())
        }
    }

    fn warn(&mut self, name: &str, detail: &str) {
        let warning = format!("{name}: {detail}");
        if self.warned.insert(warning.clone()) {
            self.warnings.push(warning);
        }
    }
}

fn root_matrix(root: &Tag) -> Matrix {
    let view_box = root
        .attributes
        .get("viewbox")
        .map(|value| parse_numbers(value))
        .unwrap_or_default();
    let width_mm = root
        .attributes
        .get("width")
        .map(|value| length_to_mm(value))
        .unwrap_or_else(|| view_box.get(2).copied().unwrap_or(100.0) * 25.4 / 96.0);
    let height_mm = root
        .attributes
        .get("height")
        .map(|value| length_to_mm(value))
        .unwrap_or_else(|| view_box.get(3).copied().unwrap_or(100.0) * 25.4 / 96.0);
    let (min_x, min_y, width, height) = if view_box.len() >= 4 {
        (view_box[0], view_box[1], view_box[2], view_box[3])
    } else {
        (0.0, 0.0, width_mm * 96.0 / 25.4, height_mm * 96.0 / 25.4)
    };
    Matrix {
        a: width_mm / width.max(f64::EPSILON),
        b: 0.0,
        c: 0.0,
        d: -height_mm / height.max(f64::EPSILON),
        e: -min_x * width_mm / width.max(f64::EPSILON),
        f: (min_y + height) * height_mm / height.max(f64::EPSILON),
    }
}

fn merged_style(
    parent: Option<&BTreeMap<String, String>>,
    attributes: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut style = parent.cloned().unwrap_or_default();
    for key in [
        "stroke",
        "stroke-width",
        "fill",
        "display",
        "visibility",
        "font-size",
    ] {
        if let Some(value) = attributes.get(key) {
            style.insert(key.into(), value.clone());
        }
    }
    if let Some(inline) = attributes.get("style") {
        for declaration in inline.split(';') {
            if let Some((key, value)) = declaration.split_once(':') {
                style.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
            }
        }
    }
    style
}

fn is_hidden(style: &BTreeMap<String, String>) -> bool {
    style.get("display").is_some_and(|value| value == "none")
        || style
            .get("visibility")
            .is_some_and(|value| value == "hidden")
}

fn parse_color(style: &BTreeMap<String, String>) -> Option<(u8, u8, u8)> {
    let value = style
        .get("stroke")
        .filter(|value| value.as_str() != "none")
        .or_else(|| style.get("fill").filter(|value| value.as_str() != "none"))?;
    color_value(value)
}

fn color_value(value: &str) -> Option<(u8, u8, u8)> {
    let value = value.trim().to_ascii_lowercase();
    if let Some(hex) = value.strip_prefix('#') {
        return match hex.len() {
            3 => Some((
                u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?,
                u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?,
                u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?,
            )),
            6 => Some((
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            )),
            _ => None,
        };
    }
    if value.starts_with("rgb(") {
        let numbers = parse_numbers(&value);
        if numbers.len() >= 3 {
            return Some((
                numbers[0].clamp(0.0, 255.0) as u8,
                numbers[1].clamp(0.0, 255.0) as u8,
                numbers[2].clamp(0.0, 255.0) as u8,
            ));
        }
    }
    Some(match value.as_str() {
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "green" => (0, 128, 0),
        "blue" => (0, 0, 255),
        "yellow" => (255, 255, 0),
        "cyan" | "aqua" => (0, 255, 255),
        "magenta" | "fuchsia" => (255, 0, 255),
        "gray" | "grey" => (128, 128, 128),
        _ => return None,
    })
}

fn stroke_width(style: &BTreeMap<String, String>) -> f64 {
    style
        .get("stroke-width")
        .map(|value| length_to_mm(value))
        .unwrap_or(0.25)
}

fn attr_number(tag: &Tag, name: &str, default: f64) -> f64 {
    tag.attributes
        .get(name)
        .map(|value| length_to_user(value))
        .unwrap_or(default)
}

fn length_to_user(value: &str) -> f64 {
    parse_first_number(value).unwrap_or(0.0)
}

fn length_to_mm(value: &str) -> f64 {
    let number = parse_first_number(value).unwrap_or(0.0);
    let lower = value.trim().to_ascii_lowercase();
    if lower.ends_with("mm") {
        number
    } else if lower.ends_with("cm") {
        number * 10.0
    } else if lower.ends_with("in") {
        number * 25.4
    } else if lower.ends_with("pt") {
        number * 25.4 / 72.0
    } else if lower.ends_with("pc") {
        number * 25.4 / 6.0
    } else {
        number * 25.4 / 96.0
    }
}

fn parse_transform(value: &str) -> Matrix {
    let mut matrix = Matrix::IDENTITY;
    let mut rest = value.trim();
    while let Some(open) = rest.find('(') {
        let name = rest[..open].trim().to_ascii_lowercase();
        let Some(close) = rest[open + 1..].find(')') else {
            break;
        };
        let arguments = parse_numbers(&rest[open + 1..open + 1 + close]);
        let transform = match name.as_str() {
            "matrix" if arguments.len() >= 6 => Matrix {
                a: arguments[0],
                b: arguments[1],
                c: arguments[2],
                d: arguments[3],
                e: arguments[4],
                f: arguments[5],
            },
            "translate" => Matrix {
                e: arguments.first().copied().unwrap_or(0.0),
                f: arguments.get(1).copied().unwrap_or(0.0),
                ..Matrix::IDENTITY
            },
            "scale" => Matrix {
                a: arguments.first().copied().unwrap_or(1.0),
                d: arguments
                    .get(1)
                    .copied()
                    .unwrap_or_else(|| arguments.first().copied().unwrap_or(1.0)),
                ..Matrix::IDENTITY
            },
            "rotate" => {
                let angle = arguments.first().copied().unwrap_or(0.0).to_radians();
                let rotation = Matrix {
                    a: angle.cos(),
                    b: angle.sin(),
                    c: -angle.sin(),
                    d: angle.cos(),
                    e: 0.0,
                    f: 0.0,
                };
                if arguments.len() >= 3 {
                    Matrix {
                        e: arguments[1],
                        f: arguments[2],
                        ..Matrix::IDENTITY
                    }
                    .multiply(rotation)
                    .multiply(Matrix {
                        e: -arguments[1],
                        f: -arguments[2],
                        ..Matrix::IDENTITY
                    })
                } else {
                    rotation
                }
            }
            "skewx" => Matrix {
                c: arguments.first().copied().unwrap_or(0.0).to_radians().tan(),
                ..Matrix::IDENTITY
            },
            "skewy" => Matrix {
                b: arguments.first().copied().unwrap_or(0.0).to_radians().tan(),
                ..Matrix::IDENTITY
            },
            _ => Matrix::IDENTITY,
        };
        matrix = matrix.multiply(transform);
        rest = &rest[open + close + 2..];
    }
    matrix
}

fn is_uniform_rotation(matrix: Matrix) -> bool {
    let x = matrix.a.hypot(matrix.b);
    let y = matrix.c.hypot(matrix.d);
    (x - y).abs() < 1e-9 && (matrix.a * matrix.c + matrix.b * matrix.d).abs() < 1e-9
}

fn sample_ellipse(center: Point, rx: f64, ry: f64, matrix: Matrix) -> Vec<Point> {
    (0..72)
        .map(|index| {
            let angle = 2.0 * PI * index as f64 / 72.0;
            matrix.apply(Point::new(
                center.x + rx * angle.cos(),
                center.y + ry * angle.sin(),
            ))
        })
        .collect()
}

fn parse_path(
    data: &str,
    matrix: Matrix,
    tolerance: f64,
) -> Result<Vec<(Vec<Point>, bool)>, String> {
    let tokens = path_tokens(data);
    let mut output = Vec::new();
    let mut path = Vec::new();
    let mut index = 0;
    let mut command = ' ';
    let mut current = Point::default();
    let mut start = Point::default();
    let mut last_control = None;
    while index < tokens.len() {
        if let PathToken::Command(value) = tokens[index] {
            command = value;
            index += 1;
        } else if command == ' ' {
            return Err("SVG path missing initial command".into());
        }
        let relative = command.is_ascii_lowercase();
        match command.to_ascii_uppercase() {
            'M' => {
                let point = read_point(&tokens, &mut index)?;
                current = absolute_point(point, current, relative);
                if !path.is_empty() {
                    output.push((std::mem::take(&mut path), false));
                }
                start = current;
                path.push(matrix.apply(current));
                command = if relative { 'l' } else { 'L' };
                last_control = None;
            }
            'L' => {
                let point = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                current = point;
                path.push(matrix.apply(point));
                last_control = None;
            }
            'H' => {
                let value = read_number(&tokens, &mut index)?;
                current.x = if relative { current.x + value } else { value };
                path.push(matrix.apply(current));
                last_control = None;
            }
            'V' => {
                let value = read_number(&tokens, &mut index)?;
                current.y = if relative { current.y + value } else { value };
                path.push(matrix.apply(current));
                last_control = None;
            }
            'C' => {
                let c1 = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                let c2 = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                let end = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                append_cubic(&mut path, current, c1, c2, end, matrix, tolerance);
                current = end;
                last_control = Some(c2);
            }
            'S' => {
                let c1 = last_control
                    .map(|control| {
                        Point::new(2.0 * current.x - control.x, 2.0 * current.y - control.y)
                    })
                    .unwrap_or(current);
                let c2 = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                let end = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                append_cubic(&mut path, current, c1, c2, end, matrix, tolerance);
                current = end;
                last_control = Some(c2);
            }
            'Q' => {
                let control = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                let end = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                append_quadratic(&mut path, current, control, end, matrix, tolerance);
                current = end;
                last_control = Some(control);
            }
            'T' => {
                let control = last_control
                    .map(|control| {
                        Point::new(2.0 * current.x - control.x, 2.0 * current.y - control.y)
                    })
                    .unwrap_or(current);
                let end = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                append_quadratic(&mut path, current, control, end, matrix, tolerance);
                current = end;
                last_control = Some(control);
            }
            'A' => {
                let rx = read_number(&tokens, &mut index)?.abs();
                let ry = read_number(&tokens, &mut index)?.abs();
                let rotation = read_number(&tokens, &mut index)?;
                let large = read_number(&tokens, &mut index)? != 0.0;
                let sweep = read_number(&tokens, &mut index)? != 0.0;
                let end = absolute_point(read_point(&tokens, &mut index)?, current, relative);
                append_arc(
                    &mut path, current, end, rx, ry, rotation, large, sweep, matrix,
                );
                current = end;
                last_control = None;
            }
            'Z' => {
                current = start;
                if !path.is_empty() {
                    output.push((std::mem::take(&mut path), true));
                }
                command = ' ';
                last_control = None;
            }
            other => return Err(format!("unsupported SVG path command {other}")),
        }
    }
    if !path.is_empty() {
        output.push((path, false));
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug)]
enum PathToken {
    Command(char),
    Number(f64),
}

fn path_tokens(data: &str) -> Vec<PathToken> {
    let mut tokens = Vec::new();
    let bytes = data.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let character = bytes[index] as char;
        if character.is_ascii_alphabetic() {
            tokens.push(PathToken::Command(character));
            index += 1;
        } else if character.is_ascii_whitespace() || character == ',' {
            index += 1;
        } else {
            let start = index;
            if matches!(bytes[index], b'+' | b'-') {
                index += 1;
            }
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if bytes.get(index) == Some(&b'.') {
                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
            }
            if matches!(bytes.get(index), Some(b'e' | b'E')) {
                index += 1;
                if matches!(bytes.get(index), Some(b'+' | b'-')) {
                    index += 1;
                }
                while index < bytes.len() && bytes[index].is_ascii_digit() {
                    index += 1;
                }
            }
            if let Ok(value) = data[start..index].parse() {
                tokens.push(PathToken::Number(value));
            } else {
                index += usize::from(index == start);
            }
        }
    }
    tokens
}

fn read_number(tokens: &[PathToken], index: &mut usize) -> Result<f64, String> {
    match tokens.get(*index) {
        Some(PathToken::Number(value)) => {
            *index += 1;
            Ok(*value)
        }
        _ => Err("insufficient parameters in SVG path".into()),
    }
}

fn read_point(tokens: &[PathToken], index: &mut usize) -> Result<Point, String> {
    Ok(Point::new(
        read_number(tokens, index)?,
        read_number(tokens, index)?,
    ))
}

fn absolute_point(point: Point, current: Point, relative: bool) -> Point {
    if relative {
        Point::new(current.x + point.x, current.y + point.y)
    } else {
        point
    }
}

fn append_cubic(
    output: &mut Vec<Point>,
    start: Point,
    c1: Point,
    c2: Point,
    end: Point,
    matrix: Matrix,
    tolerance: f64,
) {
    let length = start.distance(c1) + c1.distance(c2) + c2.distance(end);
    let segments = ((length * matrix.scale_magnitude() / tolerance.max(0.01))
        .sqrt()
        .ceil() as usize)
        .clamp(4, 1024);
    for step in 1..=segments {
        let t = step as f64 / segments as f64;
        let inv = 1.0 - t;
        output.push(matrix.apply(Point::new(
            inv.powi(3) * start.x
                + 3.0 * inv.powi(2) * t * c1.x
                + 3.0 * inv * t.powi(2) * c2.x
                + t.powi(3) * end.x,
            inv.powi(3) * start.y
                + 3.0 * inv.powi(2) * t * c1.y
                + 3.0 * inv * t.powi(2) * c2.y
                + t.powi(3) * end.y,
        )));
    }
}

fn append_quadratic(
    output: &mut Vec<Point>,
    start: Point,
    control: Point,
    end: Point,
    matrix: Matrix,
    tolerance: f64,
) {
    let length = start.distance(control) + control.distance(end);
    let segments = ((length * matrix.scale_magnitude() / tolerance.max(0.01))
        .sqrt()
        .ceil() as usize)
        .clamp(4, 1024);
    for step in 1..=segments {
        let t = step as f64 / segments as f64;
        let inv = 1.0 - t;
        output.push(matrix.apply(Point::new(
            inv * inv * start.x + 2.0 * inv * t * control.x + t * t * end.x,
            inv * inv * start.y + 2.0 * inv * t * control.y + t * t * end.y,
        )));
    }
}

#[allow(clippy::too_many_arguments)]
fn append_arc(
    output: &mut Vec<Point>,
    start: Point,
    end: Point,
    mut rx: f64,
    mut ry: f64,
    rotation_deg: f64,
    large: bool,
    sweep: bool,
    matrix: Matrix,
) {
    if rx <= 0.0 || ry <= 0.0 || start.distance(end) <= f64::EPSILON {
        output.push(matrix.apply(end));
        return;
    }
    let rotation = rotation_deg.to_radians();
    let dx = (start.x - end.x) / 2.0;
    let dy = (start.y - end.y) / 2.0;
    let x1 = rotation.cos() * dx + rotation.sin() * dy;
    let y1 = -rotation.sin() * dx + rotation.cos() * dy;
    let lambda = x1.powi(2) / rx.powi(2) + y1.powi(2) / ry.powi(2);
    if lambda > 1.0 {
        let scale = lambda.sqrt();
        rx *= scale;
        ry *= scale;
    }
    let numerator = (rx * ry).powi(2) - (rx * y1).powi(2) - (ry * x1).powi(2);
    let denominator = (rx * y1).powi(2) + (ry * x1).powi(2);
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let factor = sign * (numerator.max(0.0) / denominator.max(f64::EPSILON)).sqrt();
    let cx1 = factor * rx * y1 / ry;
    let cy1 = factor * -ry * x1 / rx;
    let center = Point::new(
        rotation.cos() * cx1 - rotation.sin() * cy1 + (start.x + end.x) / 2.0,
        rotation.sin() * cx1 + rotation.cos() * cy1 + (start.y + end.y) / 2.0,
    );
    let vector_angle =
        |ux: f64, uy: f64, vx: f64, vy: f64| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let ux = (x1 - cx1) / rx;
    let uy = (y1 - cy1) / ry;
    let vx = (-x1 - cx1) / rx;
    let vy = (-y1 - cy1) / ry;
    let start_angle = vector_angle(1.0, 0.0, ux, uy);
    let mut delta = vector_angle(ux, uy, vx, vy);
    if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    } else if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    }
    let segments = ((delta.abs() / (PI / 36.0)).ceil() as usize).clamp(4, 4096);
    for step in 1..=segments {
        let angle = start_angle + delta * step as f64 / segments as f64;
        let x = rx * angle.cos();
        let y = ry * angle.sin();
        output.push(matrix.apply(Point::new(
            center.x + rotation.cos() * x - rotation.sin() * y,
            center.y + rotation.sin() * x + rotation.cos() * y,
        )));
    }
}

fn parse_numbers(value: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len() && (bytes[index].is_ascii_whitespace() || bytes[index] == b',') {
            index += 1;
        }
        let start = index;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        }
        if matches!(bytes.get(index), Some(b'e' | b'E')) {
            index += 1;
            if matches!(bytes.get(index), Some(b'+' | b'-')) {
                index += 1;
            }
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        }
        if start < index {
            if let Ok(number) = value[start..index].parse() {
                numbers.push(number);
            }
        } else {
            index += 1;
        }
    }
    numbers
}

fn parse_first_number(value: &str) -> Option<f64> {
    parse_numbers(value).into_iter().next()
}

fn decode_xml(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}
