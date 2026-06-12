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
plotconvert - converte entre PLT/HP-GL, DXF e SVG

USO:
    plotconvert [OPCOES] <ARQUIVO.plt|ARQUIVO.dxf|ARQUIVO.svg>...

OPCOES:
    -o, --output <ARQUIVO>          Saida para uma unica entrada
    -d, --output-dir <DIRETORIO>    Diretorio para conversao em lote
    -t, --to <FORMATO>              Saida: dxf, svg, plt, hpgl ou hpgl2
        --normalize-origin          Move o menor X/Y para 0,0
        --flip-y                    Inverte o eixo Y
        --units-per-mm <NUMERO>     Unidades HP-GL por mm (padrao: 40)
        --curve-tolerance-mm <MM>   Tolerancia de curvas (padrao: 0.05)
        --plt-dialect <DIALETO>     Saida para PLT: hpgl2 (padrao) ou hpgl
        --single-layer              Coloca o DXF gerado na camada 0
        --strict                    Falha em comandos nao suportados
        --overwrite                 Substitui arquivos existentes
    -h, --help                      Mostra esta ajuda
    -V, --version                   Mostra a versao
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
            eprintln!("erro: {message}");
            eprintln!("use --help para ver as opcoes");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let cli = parse_args(env::args_os().skip(1).collect())?;
    if cli.inputs.is_empty() {
        return Err("informe pelo menos um arquivo PLT, DXF ou SVG".into());
    }
    if cli.output.is_some() && cli.output_dir.is_some() {
        return Err("--output e --output-dir nao podem ser usados juntos".into());
    }
    if cli.output.is_some() && cli.inputs.len() != 1 {
        return Err("--output aceita somente uma entrada".into());
    }
    if let Some(directory) = &cli.output_dir {
        fs::create_dir_all(directory)
            .map_err(|error| format!("nao foi possivel criar {}: {error}", directory.display()))?;
    }

    let mut had_failure = false;
    for input in &cli.inputs {
        let output = output_path(input, &cli);
        if output.exists() && !cli.overwrite {
            eprintln!(
                "erro: {} ja existe; use --overwrite para substituir",
                output.display()
            );
            had_failure = true;
            continue;
        }
        let target = target_format(input, &output, &cli)?;
        match convert_file_to(input, &output, target, &cli.options) {
            Ok(report) => {
                println!(
                    "{} -> {} ({} itens de entrada, {} entidades, {} avisos)",
                    input.display(),
                    output.display(),
                    report.command_count,
                    report.entity_count,
                    report.warning_count
                );
                for warning in report.warnings {
                    eprintln!("aviso em {}: {warning}", input.display());
                }
            }
            Err(error) => {
                eprintln!("erro ao converter {}: {error}", input.display());
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
                    _ => return Err(format!("formato de saída inválido: {value}")),
                });
            }
            "--units-per-mm" => {
                index += 1;
                cli.options.units_per_mm = parse_number(&arguments, index, &text)?;
            }
            "--curve-tolerance-mm" => {
                index += 1;
                cli.options.curve_tolerance_mm = parse_number(&arguments, index, &text)?;
            }
            "--plt-dialect" => {
                index += 1;
                let value = required_value(&arguments, index, &text)?
                    .to_string_lossy()
                    .to_ascii_lowercase();
                cli.options.plt_dialect = match value.as_str() {
                    "hpgl" | "hp-gl" => PltDialect::Hpgl,
                    "hpgl2" | "hp-gl2" | "hp-gl/2" => PltDialect::Hpgl2,
                    _ => return Err(format!("dialeto PLT inválido: {value}")),
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
            _ if text.starts_with('-') => return Err(format!("opcao desconhecida: {text}")),
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
        .ok_or_else(|| format!("falta valor para {option}"))
}

fn parse_number(arguments: &[OsString], index: usize, option: &str) -> Result<f64, String> {
    let value = required_value(arguments, index, option)?;
    value.to_string_lossy().parse().map_err(|_| {
        format!(
            "valor numerico invalido para {option}: {}",
            value.to_string_lossy()
        )
    })
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
        .map_err(|error| format!("não foi possível ler {}: {error}", input.display()))?;
    detect_format(input, &data)
        .map(default_output)
        .map_err(|error| error.to_string())
}
