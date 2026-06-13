use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use plotconvert::{
    ConversionOptions, OutputFormat, PltDialect, convert_file_to, default_output, detect_format,
    output_format_from_path,
};

const HELP: &str = "\
plotconvert - convert between PLT/HP-GL, DXF, and SVG

USAGE:
    plotconvert [OPTIONS] <FILE.plt|FILE.dxf|FILE.svg>...

OPTIONS:
    -o, --output <FILE>             Output path for a single input
    -d, --output-dir <DIRECTORY>  Directory for batch conversion
    -t, --to <FORMAT>               Output: dxf, svg, plt, hpgl, hpgl2, png, or pdf
        --normalize-origin          Move the minimum X/Y to 0,0
        --flip-y                    Flip the Y axis
        --units-per-mm <NUMBER>     HP-GL units per mm (default: 40)
        --units-per-inch <NUMBER>   HP-GL units per inch (default: 1016)
        --png-dpi <NUMBER>          PNG output resolution (default: 96)
        --png-stroke-scale <NUM>    Stroke width in PNG output (default: 3)
        --png-max-size <PIXELS>     Maximum PNG longest side (thumbnail)
        --pdf-stroke-scale <NUM>    Stroke width in PDF output (default: 1)
        --curve-tolerance-mm <MM>   Curve tolerance (default: 0.05)
        --plt-dialect <DIALECT>     PLT output: hpgl2 (default) or hpgl
        --single-layer              Put generated DXF on layer 0
        --strict                    Fail on unsupported commands
        --overwrite                 Replace existing files
    -h, --help                      Show this help
    -V, --version                   Show version
";

#[derive(Default)]
struct Cli {
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    overwrite: bool,
    target: Option<OutputFormat>,
    options: ConversionOptions,
}

fn main() -> ExitCode {
    match run() {
        Ok(had_failure) => {
            if had_failure {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("use --help to see options");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let cli = parse_args(env::args_os().skip(1).collect())?;
    if cli.inputs.is_empty() {
        return Err("provide at least one PLT, DXF, or SVG file".into());
    }
    if cli.output.is_some() && cli.output_dir.is_some() {
        return Err("--output and --output-dir cannot be used together".into());
    }
    if cli.output.is_some() && cli.inputs.len() != 1 {
        return Err("--output accepts only one input".into());
    }
    if let Some(directory) = &cli.output_dir {
        fs::create_dir_all(directory)
            .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    }

    let mut had_failure = false;
    for input in &cli.inputs {
        let output = output_path(input, &cli);
        if output.exists() && !cli.overwrite {
            eprintln!(
                "error: {} already exists; use --overwrite to replace",
                output.display()
            );
            had_failure = true;
            continue;
        }
        let target = target_format(input, &output, &cli)?;
        match convert_file_to(input, &output, target, &cli.options) {
            Ok(report) => {
                println!(
                    "{} -> {} ({} input items, {} entities, {} warnings)",
                    input.display(),
                    output.display(),
                    report.command_count,
                    report.entity_count,
                    report.warning_count
                );
                for warning in report.warnings {
                    eprintln!("warning in {}: {warning}", input.display());
                }
            }
            Err(error) => {
                eprintln!("error converting {}: {error}", input.display());
                had_failure = true;
            }
        }
    }
    Ok(had_failure)
}

fn parse_args(arguments: Vec<OsString>) -> Result<Cli, String> {
    let mut cli = Cli {
        options: ConversionOptions::default(),
        ..Cli::default()
    };
    let mut units_mm_set = false;
    let mut units_inch_set = false;
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        let text = argument.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("plotconvert {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-o" | "--output" => {
                index += 1;
                cli.output = Some(required_value(&arguments, index, &text)?.into());
            }
            "-d" | "--output-dir" => {
                index += 1;
                cli.output_dir = Some(required_value(&arguments, index, &text)?.into());
            }
            "-t" | "--to" => {
                index += 1;
                let value = required_value(&arguments, index, &text)?
                    .to_string_lossy()
                    .to_ascii_lowercase();
                cli.target = Some(match value.as_str() {
                    "dxf" => OutputFormat::Dxf,
                    "plt" => OutputFormat::Hpgl,
                    "hpgl" | "hp-gl" => {
                        cli.options.plt_dialect = PltDialect::Hpgl;
                        OutputFormat::Hpgl
                    }
                    "hpgl2" | "hp-gl2" | "hp-gl/2" => {
                        cli.options.plt_dialect = PltDialect::Hpgl2;
                        OutputFormat::Hpgl
                    }
                    "svg" | "svf" => OutputFormat::Svg,
                    "png" => OutputFormat::Png,
                    "pdf" => OutputFormat::Pdf,
                    _ => return Err(format!("invalid output format: {value}")),
                });
            }
            "--units-per-mm" => {
                if units_inch_set {
                    return Err(
                        "--units-per-mm and --units-per-inch cannot be used together".into(),
                    );
                }
                units_mm_set = true;
                index += 1;
                cli.options.units_per_mm = parse_number(&arguments, index, &text)?;
            }
            "--units-per-inch" => {
                if units_mm_set {
                    return Err(
                        "--units-per-mm and --units-per-inch cannot be used together".into(),
                    );
                }
                units_inch_set = true;
                index += 1;
                cli.options.set_units_per_inch(parse_number(&arguments, index, &text)?);
            }
            "--curve-tolerance-mm" => {
                index += 1;
                cli.options.curve_tolerance_mm = parse_number(&arguments, index, &text)?;
            }
            "--png-dpi" => {
                index += 1;
                cli.options.png_dpi = parse_number(&arguments, index, &text)?;
            }
            "--png-stroke-scale" => {
                index += 1;
                cli.options.png_stroke_scale = parse_number(&arguments, index, &text)?;
            }
            "--png-max-size" => {
                index += 1;
                cli.options.png_max_size = Some(parse_positive_u32(&arguments, index, &text)?);
            }
            "--pdf-stroke-scale" => {
                index += 1;
                cli.options.pdf_stroke_scale = parse_number(&arguments, index, &text)?;
            }
            "--plt-dialect" => {
                index += 1;
                let value = required_value(&arguments, index, &text)?
                    .to_string_lossy()
                    .to_ascii_lowercase();
                cli.options.plt_dialect = match value.as_str() {
                    "hpgl" | "hp-gl" => PltDialect::Hpgl,
                    "hpgl2" | "hp-gl2" | "hp-gl/2" => PltDialect::Hpgl2,
                    _ => return Err(format!("invalid PLT dialect: {value}")),
                };
            }
            "--normalize-origin" => cli.options.normalize_origin = true,
            "--flip-y" => cli.options.flip_y = true,
            "--single-layer" => cli.options.single_layer = true,
            "--strict" => cli.options.strict = true,
            "--overwrite" => cli.overwrite = true,
            "--" => {
                cli.inputs
                    .extend(arguments[index + 1..].iter().map(PathBuf::from));
                break;
            }
            _ if text.starts_with('-') => return Err(format!("unknown option: {text}")),
            _ => cli.inputs.push(PathBuf::from(argument)),
        }
        index += 1;
    }
    Ok(cli)
}

fn required_value<'a>(
    arguments: &'a [OsString],
    index: usize,
    option: &str,
) -> Result<&'a OsString, String> {
    arguments
        .get(index)
        .ok_or_else(|| format!("missing value for {option}"))
}

fn parse_number(arguments: &[OsString], index: usize, option: &str) -> Result<f64, String> {
    let value = required_value(arguments, index, option)?;
    value.to_string_lossy().parse().map_err(|_| {
        format!(
            "invalid numeric value for {option}: {}",
            value.to_string_lossy()
        )
    })
}

fn parse_positive_u32(arguments: &[OsString], index: usize, option: &str) -> Result<u32, String> {
    let value = parse_number(arguments, index, option)?;
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 {
        return Err(format!(
            "invalid positive integer for {option}: {}",
            required_value(arguments, index, option)?.to_string_lossy()
        ));
    }
    Ok(value as u32)
}

fn output_path(input: &Path, cli: &Cli) -> PathBuf {
    if let Some(output) = &cli.output {
        return output.clone();
    }
    let target = cli
        .target
        .or_else(|| cli.output.as_ref().and_then(output_format_from_path))
        .or_else(|| {
            fs::read(input)
                .ok()
                .and_then(|data| detect_format(input, &data).ok())
                .map(default_output)
        });
    let extension = format!(".{}", target.unwrap_or(OutputFormat::Dxf).extension());
    let file_name = input
        .file_stem()
        .map(|stem| {
            let mut name = stem.to_os_string();
            name.push(&extension);
            name
        })
        .unwrap_or_else(|| OsString::from(format!("output{extension}")));
    if let Some(directory) = &cli.output_dir {
        directory.join(file_name)
    } else {
        input.with_file_name(file_name)
    }
}

fn target_format(input: &Path, output: &Path, cli: &Cli) -> Result<OutputFormat, String> {
    if let Some(target) = cli.target {
        return Ok(target);
    }
    if let Some(target) = output_format_from_path(output) {
        return Ok(target);
    }
    let data = fs::read(input)
        .map_err(|error| format!("could not read {}: {error}", input.display()))?;
    detect_format(input, &data)
        .map(default_output)
        .map_err(|error| error.to_string())
}
