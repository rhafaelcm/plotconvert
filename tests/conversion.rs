use std::fs;
use std::path::PathBuf;

use plotconvert::{
    ConversionOptions, InputFormat, OutputFormat, PltDialect, convert_between_bytes, convert_bytes,
    convert_dxf_bytes, convert_svg_bytes,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name);
    fs::read(path).unwrap()
}

#[test]
fn converts_classic_hpgl_fixture() {
    let data = fixture("Bolso Jeans 01 ARTE 800.plt");
    let (dxf, report) = convert_bytes(&data, &ConversionOptions::default()).unwrap();
    let text = String::from_utf8(dxf).unwrap();
    assert!(text.contains("AC1009"));
    assert!(text.contains("POLYLINE"));
    assert!(text.contains("PEN_001"));
    assert!(report.command_count > 700);
    assert!(report.entity_count > 20);
    let bounds = report.bounds.unwrap();
    assert!((bounds.max.x - bounds.min.x - 2523.95).abs() < 0.01);
    assert!((bounds.max.y - bounds.min.y - 1195.0).abs() < 0.01);
}

#[test]
fn converts_hpgl2_fixture_without_semicolons() {
    let data = fixture("teste1.plt");
    let (_, report) = convert_bytes(&data, &ConversionOptions::default()).unwrap();
    assert!(report.command_count > 130);
    assert!(report.entity_count > 50);
    let bounds = report.bounds.unwrap();
    assert!((bounds.max.x - bounds.min.x - 2400.4).abs() < 0.01);
    assert!((bounds.max.y - bounds.min.y - 1227.425).abs() < 0.01);
}

#[test]
fn converts_large_classic_fixture() {
    let data = fixture("CAMISETE MOLDE 1 TRADICIONAL  COSTA INTEIRA MC.plt");
    let (_, report) = convert_bytes(&data, &ConversionOptions::default()).unwrap();
    assert!(report.command_count > 7_000);
    assert!(report.entity_count > 1_800);
    let bounds = report.bounds.unwrap();
    assert!((bounds.max.x - bounds.min.x - 3086.65).abs() < 0.01);
    assert!((bounds.max.y - bounds.min.y - 1827.675).abs() < 0.01);
}

#[test]
fn converts_large_hpgl2_fixture() {
    let data = fixture("teste2.plt");
    let (_, report) = convert_bytes(&data, &ConversionOptions::default()).unwrap();
    assert!(report.command_count > 9_900);
    assert!(report.entity_count > 4_900);
    let bounds = report.bounds.unwrap();
    assert!((bounds.max.x - bounds.min.x - 19053.4).abs() < 0.01);
    assert!((bounds.max.y - bounds.min.y - 1939.2).abs() < 0.01);
}

#[test]
fn supports_relative_coordinates_and_output_transforms() {
    let options = ConversionOptions {
        normalize_origin: true,
        flip_y: true,
        ..ConversionOptions::default()
    };
    let (_, report) =
        convert_bytes(b"IN;SP1;PU400,800;PD;PR400,0,0,400,-400,0;", &options).unwrap();
    let bounds = report.bounds.unwrap();
    assert_eq!(bounds.min.x, 0.0);
    assert_eq!(bounds.min.y, 0.0);
    assert!((bounds.max.x - 10.0).abs() < 1e-9);
    assert!((bounds.max.y - 10.0).abs() < 1e-9);
}

#[test]
fn writes_native_circle_and_arc() {
    let (dxf, report) = convert_bytes(
        b"IN;SP1;PU400,400;CI200;PU600,400;AA400,400,90;",
        &ConversionOptions::default(),
    )
    .unwrap();
    let text = String::from_utf8(dxf).unwrap();
    assert!(text.contains("\r\nCIRCLE\r\n"));
    assert!(text.contains("\r\nARC\r\n"));
    assert_eq!(report.entity_count, 2);
}

#[test]
fn decodes_pe_relative_coordinates() {
    // Zig-zag/base-64: 400 -> "_X", 0 -> "?", -400 -> "~W".
    let (_, report) =
        convert_bytes(b"IN;PU0,0;PE_X?_X?~W?;", &ConversionOptions::default()).unwrap();
    let bounds = report.bounds.unwrap();
    assert!((bounds.min.x - 0.0).abs() < 1e-9);
    assert!((bounds.max.x - 20.0).abs() < 1e-9);
    assert_eq!(report.entity_count, 1);
}

#[test]
fn strict_mode_rejects_unknown_commands() {
    let options = ConversionOptions {
        strict: true,
        ..ConversionOptions::default()
    };
    let error = convert_bytes(b"IN;ZZ1,2;", &options).unwrap_err();
    assert!(error.to_string().contains("ZZ"));
}

#[test]
fn exports_pen_width_on_polylines() {
    let (dxf, _) = convert_bytes(
        b"IN;PW0.35,1;SP1;PU0,0;PD400,0;",
        &ConversionOptions::default(),
    )
    .unwrap();
    let text = String::from_utf8(dxf).unwrap();
    assert!(text.contains(" 40\r\n0.35\r\n"));
    assert!(text.contains(" 41\r\n0.35\r\n"));
}

const SAMPLE_DXF: &str = "\
0\r
SECTION\r
2\r
HEADER\r
9\r
$ACADVER\r
1\r
AC1014\r
9\r
$INSUNITS\r
70\r
4\r
0\r
ENDSEC\r
0\r
SECTION\r
2\r
TABLES\r
0\r
TABLE\r
2\r
LAYER\r
0\r
LAYER\r
2\r
CUT\r
62\r
1\r
0\r
ENDTAB\r
0\r
ENDSEC\r
0\r
SECTION\r
2\r
BLOCKS\r
0\r
BLOCK\r
2\r
MARK\r
10\r
5\r
20\r
5\r
0\r
LINE\r
8\r
CUT\r
10\r
5\r
20\r
5\r
11\r
15\r
21\r
5\r
0\r
ENDBLK\r
0\r
ENDSEC\r
0\r
SECTION\r
2\r
ENTITIES\r
0\r
LINE\r
8\r
CUT\r
10\r
0\r
20\r
0\r
11\r
100\r
21\r
0\r
0\r
CIRCLE\r
8\r
CUT\r
10\r
50\r
20\r
50\r
40\r
10\r
0\r
ARC\r
8\r
CUT\r
10\r
50\r
20\r
50\r
40\r
20\r
50\r
0\r
51\r
90\r
0\r
LWPOLYLINE\r
8\r
CUT\r
70\r
1\r
10\r
0\r
20\r
20\r
42\r
0.414213562\r
10\r
20\r
20\r
20\r
10\r
20\r
20\r
40\r
0\r
TEXT\r
8\r
CUT\r
10\r
10\r
20\r
60\r
40\r
5\r
1\r
TESTE\r
0\r
INSERT\r
8\r
CUT\r
2\r
MARK\r
10\r
200\r
20\r
100\r
41\r
2\r
42\r
2\r
50\r
90\r
0\r
ENDSEC\r
0\r
EOF\r
";

#[test]
fn converts_dxf_to_hpgl2() {
    let (plt, report) =
        convert_dxf_bytes(SAMPLE_DXF.as_bytes(), &ConversionOptions::default()).unwrap();
    let text = String::from_utf8(plt).unwrap();
    assert!(text.starts_with("\u{1b}%-1BBP;IN;"));
    assert!(text.contains("PC1,255,0,0;"));
    assert!(text.contains("CI400;"));
    assert!(text.contains("AA2000,2000,90.0;"));
    assert!(text.contains("LBTESTE\u{3}"));
    assert_eq!(report.warning_count, 0);
    assert!(report.entity_count >= 6);
}

#[test]
fn roundtrips_dxf_through_hpgl2() {
    let options = ConversionOptions::default();
    let (plt, first_report) = convert_dxf_bytes(SAMPLE_DXF.as_bytes(), &options).unwrap();
    let (dxf, second_report) = convert_bytes(&plt, &options).unwrap();
    let text = String::from_utf8(dxf).unwrap();
    assert!(text.contains("AC1009"));
    assert!(text.contains("CIRCLE"));
    assert!(text.contains("ARC"));
    assert!(text.contains("TEXT"));
    assert!(second_report.entity_count >= first_report.entity_count);
    let bounds = second_report.bounds.unwrap();
    assert!(bounds.max.x >= 200.0);
    assert!(bounds.max.y >= 120.0);
}

#[test]
fn converts_ps800_dxf_fixture() {
    let data = fixture("PS800/Arquivos/Desenho1.dxf");
    let (plt, report) = convert_dxf_bytes(&data, &ConversionOptions::default()).unwrap();
    assert!(plt.starts_with(b"\x1b%-1B"));
    assert_eq!(report.entity_count, 2);
    assert_eq!(report.warning_count, 0);
}

#[test]
fn writes_classic_hpgl_when_requested() {
    let options = ConversionOptions {
        plt_dialect: PltDialect::Hpgl,
        ..ConversionOptions::default()
    };
    let (plt, _) = convert_dxf_bytes(SAMPLE_DXF.as_bytes(), &options).unwrap();
    let text = String::from_utf8(plt).unwrap();
    assert!(text.starts_with("IN;SP1;"));
    assert!(!text.contains("\u{1b}%-1B"));
    assert!(!text.contains("PC1,"));
    assert!(!text.contains("PG;"));
}

#[test]
fn honors_dxf_inch_units() {
    let dxf = SAMPLE_DXF.replace("$INSUNITS\r\n70\r\n4", "$INSUNITS\r\n70\r\n1");
    let (plt, report) = convert_dxf_bytes(dxf.as_bytes(), &ConversionOptions::default()).unwrap();
    let text = String::from_utf8(plt).unwrap();
    assert!(text.contains("PU0,0;PD101600,0;"));
    assert!(report.bounds.unwrap().max.x >= 5_000.0);
}

const SAMPLE_SVG: &str = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="80mm"
     viewBox="0 0 100 80">
  <g transform="translate(5 5)" fill="none" stroke="#ff0000" stroke-width="0.5">
    <line x1="0" y1="0" x2="20" y2="0"/>
    <rect x="0" y="10" width="20" height="10"/>
    <circle cx="40" cy="15" r="5"/>
    <ellipse cx="60" cy="15" rx="8" ry="4"/>
    <polyline points="0,30 10,35 20,30"/>
    <polygon points="30,30 40,35 50,30"/>
    <path d="M 0 50 C 10 40 20 60 30 50 Q 40 40 50 50 A 8 5 0 0 1 70 50"/>
    <text x="0" y="65" font-size="5">SVG TEST</text>
  </g>
</svg>"##;

#[test]
fn converts_dxf_to_svg() {
    let (svg, report) = convert_between_bytes(
        SAMPLE_DXF.as_bytes(),
        InputFormat::Dxf,
        OutputFormat::Svg,
        &ConversionOptions::default(),
    )
    .unwrap();
    let text = String::from_utf8(svg).unwrap();
    assert!(text.contains("<svg"));
    assert!(text.contains("<circle"));
    assert!(text.contains("<path"));
    assert!(text.contains("<text"));
    assert!(report.entity_count >= 6);
}

#[test]
fn converts_plt_to_svg() {
    let (svg, report) = convert_between_bytes(
        b"IN;SP1;PU0,0;PD400,0,400,400;PU800,800;CI200;",
        InputFormat::Hpgl,
        OutputFormat::Svg,
        &ConversionOptions::default(),
    )
    .unwrap();
    let text = String::from_utf8(svg).unwrap();
    assert!(text.contains("<polyline"));
    assert!(text.contains("<circle"));
    assert_eq!(report.entity_count, 2);
}

#[test]
fn converts_svg_to_dxf() {
    let (dxf, report) = convert_svg_bytes(
        SAMPLE_SVG.as_bytes(),
        OutputFormat::Dxf,
        &ConversionOptions::default(),
    )
    .unwrap();
    let text = String::from_utf8(dxf).unwrap();
    assert!(text.contains("AC1009"));
    assert!(text.contains("POLYLINE"));
    assert!(text.contains("CIRCLE"));
    assert!(text.contains("TEXT"));
    assert!(report.entity_count >= 8);
    assert_eq!(report.warning_count, 0);
}

#[test]
fn converts_svg_or_svf_to_plt() {
    let (plt, report) = convert_svg_bytes(
        SAMPLE_SVG.as_bytes(),
        OutputFormat::Hpgl,
        &ConversionOptions::default(),
    )
    .unwrap();
    let text = String::from_utf8(plt).unwrap();
    assert!(text.starts_with("\u{1b}%-1BBP;IN;"));
    assert!(text.contains("PD"));
    assert!(text.contains("CI"));
    assert!(text.contains("LBSVG TEST\u{3}"));
    assert!(report.entity_count >= 8);
}

#[test]
fn roundtrips_svg_through_dxf() {
    let options = ConversionOptions::default();
    let (dxf, first) = convert_between_bytes(
        SAMPLE_SVG.as_bytes(),
        InputFormat::Svg,
        OutputFormat::Dxf,
        &options,
    )
    .unwrap();
    let (svg, second) =
        convert_between_bytes(&dxf, InputFormat::Dxf, OutputFormat::Svg, &options).unwrap();
    assert!(String::from_utf8(svg).unwrap().contains("<svg"));
    assert!(second.entity_count >= first.entity_count);
}
