use svg2pdf::{ConversionOptions as PdfConversionOptions, PageOptions};

use crate::svg_writer;
use crate::{ConversionError, ConversionOptions, Drawing};

pub fn write_pdf(drawing: &Drawing, options: &ConversionOptions) -> Result<Vec<u8>, ConversionError> {
    let svg = svg_writer::write_svg_with_stroke_scale(drawing, options.pdf_stroke_scale);

    let mut parse_options = svg2pdf::usvg::Options::default();
    parse_options.fontdb_mut().load_system_fonts();

    let tree = svg2pdf::usvg::Tree::from_str(&svg, &parse_options).map_err(|error| {
        ConversionError::Parse(format!("failed to parse SVG for PDF: {error}"))
    })?;

    svg2pdf::to_pdf(
        &tree,
        PdfConversionOptions::default(),
        PageOptions::default(),
    )
    .map_err(|error| ConversionError::Parse(format!("failed to encode PDF: {error}")))
}
