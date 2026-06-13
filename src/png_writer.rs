use std::sync::Arc;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, Tree};
use resvg::render;

use crate::svg_writer;
use crate::{ConversionError, ConversionOptions, Drawing};

pub fn write_png(drawing: &Drawing, options: &ConversionOptions) -> Result<Vec<u8>, ConversionError> {
    let svg = svg_writer::write_svg_with_stroke_scale(drawing, options.png_stroke_scale);
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let mut parse_options = usvg::Options::default();
    parse_options.dpi = options.png_dpi as f32;
    parse_options.fontdb = Arc::new(fontdb);

    let tree = Tree::from_str(&svg, &parse_options)
        .map_err(|error| ConversionError::Parse(format!("falha ao interpretar SVG para PNG: {error}")))?;

    let size = tree.size();
    let (width, height, transform) = fit_png_scale(size.width(), size.height(), options.png_max_size);
    let mut pixmap = Pixmap::new(width, height).ok_or_else(|| {
        ConversionError::Parse(format!("dimensões PNG inválidas: {width}x{height}"))
    })?;

    render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|error| ConversionError::Parse(format!("falha ao codificar PNG: {error}")))
}

fn fit_png_scale(svg_width: f32, svg_height: f32, max_size: Option<u32>) -> (u32, u32, Transform) {
    let mut scale = 1.0_f32;
    if let Some(max_size) = max_size {
        let longest = svg_width.max(svg_height);
        if longest > 0.0 {
            scale = (max_size as f32 / longest).min(1.0);
        }
    }
    let width = (svg_width * scale).ceil().max(1.0) as u32;
    let height = (svg_height * scale).ceil().max(1.0) as u32;
    (width, height, Transform::from_scale(scale, scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_png_scale_limits_longest_side() {
        let (width, height, _) = fit_png_scale(3200.0, 2400.0, Some(512));
        assert_eq!(width, 512);
        assert_eq!(height, 384);
    }

    #[test]
    fn fit_png_scale_does_not_upscale() {
        let (width, height, transform) = fit_png_scale(100.0, 50.0, Some(4096));
        assert_eq!(width, 100);
        assert_eq!(height, 50);
        assert!((transform.sx - 1.0).abs() < f32::EPSILON);
    }
}
