#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Clone, Debug)]
pub enum Entity {
    Polyline {
        points: Vec<Point>,
        closed: bool,
        pen: u16,
    },
    Circle {
        center: Point,
        radius: f64,
        pen: u16,
    },
    Arc {
        center: Point,
        radius: f64,
        start_deg: f64,
        end_deg: f64,
        pen: u16,
    },
    Text {
        position: Point,
        value: String,
        height: f64,
        rotation_deg: f64,
        pen: u16,
    },
}

impl Entity {
    pub fn pen(&self) -> u16 {
        match self {
            Self::Polyline { pen, .. }
            | Self::Circle { pen, .. }
            | Self::Arc { pen, .. }
            | Self::Text { pen, .. } => *pen,
        }
    }

    pub fn visit_points(&self, mut visit: impl FnMut(Point)) {
        match self {
            Self::Polyline { points, .. } => points.iter().copied().for_each(&mut visit),
            Self::Circle { center, radius, .. } | Self::Arc { center, radius, .. } => {
                visit(Point::new(center.x - radius, center.y - radius));
                visit(Point::new(center.x + radius, center.y + radius));
            }
            Self::Text { position, .. } => visit(*position),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PenStyle {
    pub number: u16,
    pub color_rgb: Option<(u8, u8, u8)>,
    pub width_mm: Option<f64>,
}

impl PenStyle {
    pub fn new(number: u16) -> Self {
        Self {
            number,
            color_rgb: None,
            width_mm: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Drawing {
    pub entities: Vec<Entity>,
    pub pens: Vec<PenStyle>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Bounds {
    pub min: Point,
    pub max: Point,
}

impl Drawing {
    pub fn bounds(&self) -> Option<Bounds> {
        let mut min = Point::new(f64::INFINITY, f64::INFINITY);
        let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for entity in &self.entities {
            entity.visit_points(|point| {
                min.x = min.x.min(point.x);
                min.y = min.y.min(point.y);
                max.x = max.x.max(point.x);
                max.y = max.y.max(point.y);
            });
        }
        min.x.is_finite().then_some(Bounds { min, max })
    }
}
