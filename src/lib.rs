mod dxf;
mod dxf_reader;
mod hpgl;
mod hpgl_writer;
mod model;
mod parser;
mod svg_reader;
mod svg_writer;

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

pub use model::{Bounds, Drawing, Entity, PenStyle, Point};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputFormat {
    Hpgl,
    Dxf,
    Svg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Hpgl,
    Dxf,
    Svg,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Hpgl => "plt",
            Self::Dxf => "dxf",
            Self::Svg => "svg",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PltDialect {
    Hpgl,
    #[default]
    Hpgl2,
}

#[derive(Clone, Debug)]
pub struct ConversionOptions {
    pub units_per_mm: f64,
    pub curve_tolerance_mm: f64,
    pub normalize_origin: bool,
    pub flip_y: bool,
    pub single_layer: bool,
    pub strict: bool,
    pub plt_dialect: PltDialect,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            units_per_mm: 40.0,
            curve_tolerance_mm: 0.05,
            normalize_origin: false,
            flip_y: false,
            single_layer: false,
            strict: false,
            plt_dialect: PltDialect::Hpgl2,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConversionReport {
    pub command_count: usize,
    pub entity_count: usize,
    pub warning_count: usize,
    pub warnings: Vec<String>,
    pub bounds: Option<Bounds>,
}

#[derive(Debug)]
pub enum ConversionError {
    Io(io::Error),
    InvalidOption(String),
    Parse(String),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::InvalidOption(message) | Self::Parse(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ConversionError {}

impl From<io::Error> for ConversionError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn convert_bytes(
    input: &[u8],
    options: &ConversionOptions,
) -> Result<(Vec<u8>, ConversionReport), ConversionError> {
    validate_options(options)?;
    let commands = parser::tokenize(input);
    let command_count = commands.len();
    let (mut drawing, warnings) = hpgl::interpret(&commands, options)?;
    hpgl::apply_output_transform(&mut drawing, options);
    let bounds = drawing.bounds();
    let entity_count = drawing.entities.len();
    let output = dxf::write_r12(&drawing, options);
    Ok((
        output.into_bytes(),
        ConversionReport {
            command_count,
            entity_count,
            warning_count: warnings.len(),
            warnings,
            bounds,
        },
    ))
}

pub fn convert_dxf_bytes(
    input: &[u8],
    options: &ConversionOptions,
) -> Result<(Vec<u8>, ConversionReport), ConversionError> {
    validate_options(options)?;
    let (mut drawing, source_count, warnings) = dxf_reader::read(input, options)?;
    hpgl::apply_output_transform(&mut drawing, options);
    let bounds = drawing.bounds();
    let entity_count = drawing.entities.len();
    let output = hpgl_writer::write_hpgl2(&drawing, options);
    Ok((
        output.into_bytes(),
        ConversionReport {
            command_count: source_count,
            entity_count,
            warning_count: warnings.len(),
            warnings,
            bounds,
        },
    ))
}

pub fn convert_svg_bytes(
    input: &[u8],
    output: OutputFormat,
    options: &ConversionOptions,
) -> Result<(Vec<u8>, ConversionReport), ConversionError> {
    convert_between_bytes(input, InputFormat::Svg, output, options)
}

pub fn convert_between_bytes(
    input: &[u8],
    source: InputFormat,
    output: OutputFormat,
    options: &ConversionOptions,
) -> Result<(Vec<u8>, ConversionReport), ConversionError> {
    validate_options(options)?;
    let (mut drawing, source_count, warnings) = match source {
        InputFormat::Hpgl => {
            let commands = parser::tokenize(input);
            let count = commands.len();
            let (drawing, warnings) = hpgl::interpret(&commands, options)?;
            (drawing, count, warnings)
        }
        InputFormat::Dxf => dxf_reader::read(input, options)?,
        InputFormat::Svg => svg_reader::read(input, options)?,
    };
    hpgl::apply_output_transform(&mut drawing, options);
    let bounds = drawing.bounds();
    let entity_count = drawing.entities.len();
    let converted = match output {
        OutputFormat::Hpgl => hpgl_writer::write_hpgl2(&drawing, options).into_bytes(),
        OutputFormat::Dxf => dxf::write_r12(&drawing, options).into_bytes(),
        OutputFormat::Svg => svg_writer::write_svg(&drawing).into_bytes(),
    };
    Ok((
        converted,
        ConversionReport {
            command_count: source_count,
            entity_count,
            warning_count: warnings.len(),
            warnings,
            bounds,
        },
    ))
}

pub fn convert_file(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    options: &ConversionOptions,
) -> Result<ConversionReport, ConversionError> {
    let input = input.as_ref();
    let output = output.as_ref();
    let data = fs::read(input)?;
    let source = detect_format(input, &data)?;
    let target = output_format_from_path(output).unwrap_or_else(|| default_output(source));
    let (converted, report) = convert_between_bytes(&data, source, target, options)?;
    fs::write(output, converted)?;
    Ok(report)
}

pub fn convert_file_to(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    target: OutputFormat,
    options: &ConversionOptions,
) -> Result<ConversionReport, ConversionError> {
    let input = input.as_ref();
    let data = fs::read(input)?;
    let source = detect_format(input, &data)?;
    let (converted, report) = convert_between_bytes(&data, source, target, options)?;
    fs::write(output, converted)?;
    Ok(report)
}

pub fn detect_format(path: impl AsRef<Path>, data: &[u8]) -> Result<InputFormat, ConversionError> {
    if let Some(extension) = path.as_ref().extension().and_then(|value| value.to_str()) {
        if extension.eq_ignore_ascii_case("dxf") {
            return Ok(InputFormat::Dxf);
        }
        if extension.eq_ignore_ascii_case("plt") || extension.eq_ignore_ascii_case("hpgl") {
            return Ok(InputFormat::Hpgl);
        }
        if extension.eq_ignore_ascii_case("svg") || extension.eq_ignore_ascii_case("svf") {
            return Ok(InputFormat::Svg);
        }
    }
    let prefix = String::from_utf8_lossy(&data[..data.len().min(256)]);
    let lower = prefix.to_ascii_lowercase();
    if lower.contains("<svg") {
        Ok(InputFormat::Svg)
    } else if prefix.contains("SECTION") || prefix.contains("$ACADVER") {
        Ok(InputFormat::Dxf)
    } else if data.windows(2).any(|window| {
        matches!(
            window,
            b"IN" | b"BP" | b"PU" | b"PD" | b"PA" | b"PR" | b"SP"
        )
    }) {
        Ok(InputFormat::Hpgl)
    } else {
        Err(ConversionError::Parse(
            "não foi possível detectar se a entrada é DXF, PLT ou SVG".into(),
        ))
    }
}

pub fn output_format_from_path(path: impl AsRef<Path>) -> Option<OutputFormat> {
    let extension = path.as_ref().extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("dxf") {
        Some(OutputFormat::Dxf)
    } else if extension.eq_ignore_ascii_case("plt") || extension.eq_ignore_ascii_case("hpgl") {
        Some(OutputFormat::Hpgl)
    } else if extension.eq_ignore_ascii_case("svg") || extension.eq_ignore_ascii_case("svf") {
        Some(OutputFormat::Svg)
    } else {
        None
    }
}

pub fn default_output(source: InputFormat) -> OutputFormat {
    match source {
        InputFormat::Hpgl => OutputFormat::Dxf,
        InputFormat::Dxf => OutputFormat::Hpgl,
        InputFormat::Svg => OutputFormat::Dxf,
    }
}

fn validate_options(options: &ConversionOptions) -> Result<(), ConversionError> {
    if !options.units_per_mm.is_finite() || options.units_per_mm <= 0.0 {
        return Err(ConversionError::InvalidOption(
            "units-per-mm deve ser maior que zero".into(),
        ));
    }
    if !options.curve_tolerance_mm.is_finite() || options.curve_tolerance_mm <= 0.0 {
        return Err(ConversionError::InvalidOption(
            "curve-tolerance-mm deve ser maior que zero".into(),
        ));
    }
    Ok(())
}
