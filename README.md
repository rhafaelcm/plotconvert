# plotconvert

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

English documentation. For Portuguese, see [README-BR.md](README-BR.md).

Converter between ASCII DXF, HP-GL/HP-GL2 (`.plt`, `.hpgl`), and SVG (`.svg`, `.svf`),
written in Rust.

The converter supports six cross-format vector conversion routes,
plus **PNG** and **PDF** as export-only outputs from any input:

- **DXF** → PLT, SVG, PNG, PDF
- **PLT/HP-GL** → DXF, SVG, PNG, PDF
- **SVG/SVF** → DXF, PLT, PNG, PDF

| Input | Outputs |
| --- | --- |
| DXF (`.dxf`) | PLT, SVG, PNG, or PDF |
| PLT/HP-GL (`.plt`, `.hpgl`) | DXF, SVG, PNG, or PDF |
| SVG/SVF (`.svg`, `.svf`) | DXF, PLT, PNG, or PDF |

## Usage

```bash
plotconvert drawing.plt
plotconvert drawing.dxf
plotconvert drawing.svg
plotconvert --to svg drawing.dxf
plotconvert --to svg drawing.plt
plotconvert --to plt drawing.svg
plotconvert --to dxf drawing.svg
plotconvert --to png drawing.dxf
plotconvert --to pdf drawing.dxf
```

The input format is detected from the file extension and, when needed, from
the file content.

**Supported conversions** — use `--to` or the extension given in `--output`
to choose the output format:

- DXF → PLT, SVG, PNG, or PDF;
- PLT/HP-GL → DXF, SVG, PNG, or PDF;
- SVG/SVF → DXF, PLT, PNG, or PDF.

PNG and PDF are **output only**; `.png` and `.pdf` files are not accepted as input.

**Default output** (without `--to` or an explicit extension in `--output`):

- `.plt` or `.hpgl` input → `.dxf`;
- `.dxf` input → `.plt` (HP-GL/2);
- `.svg` or `.svf` input → `.dxf`.

Without `--output` or `--output-dir`, the converted file is created next to the
input, with the same base name and the new extension.

Run `plotconvert --help` to see all options.

## Options

### `-t, --to <FORMAT>`

Explicitly chooses the output format. This is the main option for selecting
the conversion destination.

Accepted values:

- `dxf`: produces ASCII DXF R12;
- `svg` or `svf`: produces SVG;
- `png`: produces a rasterized PNG image;
- `pdf`: produces a vector PDF document;
- `plt`: produces PLT using the `--plt-dialect` value;
- `hpgl`: produces PLT in classic HP-GL;
- `hpgl2`: produces PLT in HP-GL/2.

```bash
# DXF → PLT (HP-GL/2)
plotconvert --to plt drawing.dxf

# DXF → SVG
plotconvert --to svg drawing.dxf

# DXF → PNG
plotconvert --to png drawing.dxf

# PLT → DXF
plotconvert --to dxf drawing.plt

# PLT → SVG
plotconvert --to svg drawing.plt

# PLT → PNG
plotconvert --to png drawing.plt

# SVG → DXF
plotconvert --to dxf drawing.svg

# SVG → PLT (classic HP-GL)
plotconvert --to hpgl drawing.svg

# SVG → PNG
plotconvert --to png drawing.svg

# DXF → PDF
plotconvert --to pdf drawing.dxf

# PLT → PDF
plotconvert --to pdf pattern.plt

# SVG → PDF
plotconvert --to pdf drawing.svg
```

When `--to` is used, it takes precedence over the extension given in
`--output`.

### `-o, --output <FILE>`

Sets the exact output file path. Can only be used with a single input. When
`--to` is not given, the output extension selects the format.

```bash
plotconvert drawing.plt --output result.dxf
plotconvert drawing.dxf -o result.plt
plotconvert drawing.dxf -o result.svg
plotconvert drawing.plt -o result.svg
plotconvert drawing.svg -o result.dxf
plotconvert drawing.svg -o result.plt
plotconvert drawing.dxf -o preview.png
plotconvert drawing.dxf -o preview.pdf
```

Cannot be combined with `--output-dir`.

### `-d, --output-dir <DIRECTORY>`

Sets the destination directory. Can be used with one or many inputs; the
directory is created automatically when it does not exist.

```bash
plotconvert --to svg --output-dir converted pattern.plt drawing.dxf
```

Each output keeps the input base name. For example, `pattern.plt` produces
`converted/pattern.svg` when used with `--to svg`.

For batch conversions with a specific output, always use `--to`.

### `--plt-dialect <DIALECT>`

Chooses the dialect used for any conversion whose output is PLT.

Accepted values:

- `hpgl2`, `hp-gl2`, or `hp-gl/2`: produces HP-GL/2 with PCL preamble, page
  declaration, pen count, colors, and widths. This is the default.
- `hpgl` or `hp-gl`: produces classic HP-GL, with commands terminated by `;`
  and without the HP-GL/2 preamble.

```bash
# HP-GL/2 output, default behavior
plotconvert drawing.dxf
plotconvert --plt-dialect hpgl2 drawing.dxf

# Classic HP-GL output
plotconvert --plt-dialect hpgl drawing.dxf
```

This option does not change conversions whose input is PLT, nor DXF or SVG
outputs.

### `--units-per-mm <NUMBER>`

Sets how many HP-GL units represent one millimeter. The default is `40`,
equivalent to `1016` units per inch.

When reading PLT, HP-GL coordinates are divided by this value. When generating
PLT, coordinates in millimeters are multiplied by this value. It does not
directly change the scale of DXF to SVG or SVG to DXF.

```bash
plotconvert --units-per-mm 40 drawing.plt
plotconvert --units-per-mm 100 drawing.dxf
```

The value must be greater than zero. You usually do not need to change it.

Cannot be combined with `--units-per-inch`.

### `--units-per-inch <NUMBER>`

Sets how many HP-GL units represent one inch. The implicit default is `1016`,
equivalent to `--units-per-mm 40`.

When reading PLT, HP-GL coordinates are divided by `value / 25.4`. When
generating PLT, coordinates in millimeters are multiplied by `value / 25.4`.
It does not directly change the scale of DXF to SVG or SVG to DXF.

```bash
plotconvert --units-per-inch 1016 drawing.plt
plotconvert --units-per-inch 1016 drawing.dxf
plotconvert --units-per-inch 2032 drawing.plt
```

The value must be greater than zero. Use this option when the plotter resolution
is known in units per inch.

Cannot be combined with `--units-per-mm`.

### `--png-dpi <NUMBER>`

Sets the PNG image resolution in dots per inch. The default is `96`.

Applies only to conversions whose output is PNG. Higher values produce images
with more pixels and potentially larger files.

```bash
plotconvert --to png drawing.dxf
plotconvert --to png --png-dpi 150 drawing.plt
plotconvert --to png --png-dpi 300 drawing.svg
```

The value must be greater than zero.

### `--png-stroke-scale <NUMBER>`

Multiplies stroke width in PNG output. The default is `3`, making outlines more
visible when rasterized (thin millimeter strokes become only a few pixels at
96 DPI).

Applies only to conversions whose output is PNG. SVG export is not affected.

```bash
plotconvert --to png drawing.dxf
plotconvert --to png --png-stroke-scale 2 drawing.dxf
plotconvert --to png --png-stroke-scale 4 --png-dpi 150 drawing.plt
```

The value must be greater than zero. Use `1` to keep the same relative stroke
width as SVG export.

### `--png-max-size <PIXELS>`

Limits the **longest side** (width or height) of the PNG image, in pixels. The
image is scaled down proportionally when it exceeds this value; drawings that
are already smaller are **not upscaled**.

Applies only to conversions whose output is PNG. Useful for generating
thumbnails without creating very large images from extensive drawings.

The base size comes from `--png-dpi` and the drawing dimensions; `--png-max-size`
applies a cap after that rasterization sizing.

```bash
plotconvert --to png --png-max-size 512 drawing.dxf
plotconvert --to png --png-max-size 256 --png-dpi 96 pattern.plt
plotconvert --to png --png-max-size 1024 drawing.svg
```

The value must be a positive integer. Without this option, there is no size
limit.

### `--pdf-stroke-scale <NUMBER>`

Multiplies stroke width in PDF output. The default is `1`, preserving the
millimeter stroke widths from the intermediate SVG export.

Applies only to conversions whose output is PDF. PNG export is not affected.

```bash
plotconvert --to pdf drawing.dxf
plotconvert --to pdf --pdf-stroke-scale 2 drawing.dxf
plotconvert --to pdf --pdf-stroke-scale 1.5 pattern.plt
```

The value must be greater than zero.

### `--curve-tolerance-mm <MM>`

Sets the tolerance, in millimeters, used to approximate curves that have no
direct representation in the destination format. The default is `0.05`.

Mainly affects `SPLINE`, `ELLIPSE`, Bézier curves, SVG paths, and
circles/arcs under non-uniform transforms.

```bash
# More precision and potentially larger files
plotconvert --curve-tolerance-mm 0.01 drawing.dxf

# Fewer points and smaller files
plotconvert --curve-tolerance-mm 0.2 drawing.dxf
```

Smaller values produce more segments. The value must be greater than zero.

### `--normalize-origin`

Moves all geometry so the minimum X and minimum Y are `0,0`, preserving the
drawing dimensions.

```bash
plotconvert --normalize-origin drawing.dxf
```

Useful for machines or programs that do not handle negative coordinates or
drawings far from the origin well.

### `--flip-y`

Inverts the Y axis sign before generating output.

```bash
plotconvert --flip-y drawing.plt
plotconvert --flip-y --normalize-origin drawing.dxf
```

When combined with `--normalize-origin`, the flip is applied first and the
resulting geometry is repositioned at `0,0`.

### `--single-layer`

Applies to any conversion with DXF output. Places all entities on layer `0`.

```bash
plotconvert --single-layer drawing.plt
```

Without this option, each pen or stroke style is exported to a layer:
`PEN_001`, `PEN_002`, etc.

### `--strict`

Stops conversion when an unsupported or malformed HP-GL command, DXF entity, or
SVG element is encountered.

```bash
plotconvert --strict drawing.dxf
```

Without this option, the converter continues processing the rest of the file
and shows warnings on `stderr`. In DXF, for example, `DIMENSION` and `HATCH`
fill are skipped with a warning.

### `--overwrite`

Allows replacing existing output files.

```bash
plotconvert --overwrite drawing.plt
plotconvert --to svg --output-dir converted --overwrite *.dxf
```

Without this option, an existing output is not changed and that conversion is
reported as an error.

### `-h, --help`

Shows the command-line help summary.

```bash
plotconvert --help
```

### `-V, --version`

Shows the converter version.

```bash
plotconvert --version
```

### `--`

Ends option processing. Required to convert a file whose name starts with a
hyphen.

```bash
plotconvert -- -drawing.plt
```

## Examples

### DXF → PLT

Convert a DXF to HP-GL/2 (default output):

```bash
plotconvert pattern.dxf
plotconvert --to hpgl2 pattern.dxf
```

Convert a DXF to classic HP-GL:

```bash
plotconvert --plt-dialect hpgl pattern.dxf
```

### DXF → SVG

```bash
plotconvert --to svg pattern.dxf
```

### DXF → PNG

```bash
plotconvert --to png pattern.dxf
plotconvert --to png --png-dpi 300 pattern.dxf
```

### DXF → PDF

```bash
plotconvert --to pdf pattern.dxf
plotconvert --to pdf --pdf-stroke-scale 2 pattern.dxf
```

### PLT → DXF

Convert a PLT to DXF R12 (default output):

```bash
plotconvert pattern.plt
```

Generate a simple DXF without per-pen layers:

```bash
plotconvert --single-layer pattern.plt
```

### PLT → SVG

```bash
plotconvert --to svg pattern.plt
```

### PLT → PNG

```bash
plotconvert --to png pattern.plt
```

### PLT → PDF

```bash
plotconvert --to pdf pattern.plt
```

### SVG → DXF

Convert SVG to DXF (default output):

```bash
plotconvert drawing.svg
plotconvert --to dxf drawing.svg
```

Files with the `.svf` extension are also accepted as SVG input.

### SVG → PLT

```bash
plotconvert --to plt drawing.svg
plotconvert --to hpgl drawing.svf
```

### SVG → PNG

```bash
plotconvert --to png drawing.svg
plotconvert drawing.svg -o preview.png
```

### SVG → PDF

```bash
plotconvert --to pdf drawing.svg
plotconvert drawing.svg -o preview.pdf
```

### Common options

Convert several files in batch:

```bash
plotconvert --to svg --output-dir converted pattern.plt drawing.dxf pocket.dxf
```

Normalize origin, flip Y, and replace an existing output:

```bash
plotconvert --normalize-origin --flip-y --overwrite pattern.dxf
```

## DXF input

Possible outputs: **PLT** (default), **SVG**, **PNG**, and **PDF**.

The reader accepts ASCII DXF R12, R14, and later versions with group-code
structure. Supported entities:

- `LINE`, `ARC`, `CIRCLE`, `POINT`;
- `POLYLINE` and `LWPOLYLINE`, including bulge segments;
- `ELLIPSE` and `SPLINE`, approximated as paths;
- `TEXT`, `MTEXT`, `ATTRIB`, and `ATTDEF`;
- `SOLID`, `TRACE`, `3DFACE`, `BLOCK`, and `INSERT`.

Units declared in `$INSUNITS` are converted to millimeters.
DXFs without declared units are interpreted as millimeters.

`DIMENSION` is skipped to avoid duplicating dimension geometry. `HATCH` fill is
also skipped; its original outlines are still converted. Use `--strict` to turn
unsupported entities into errors.

### DXF → PLT

The default output is HP-GL/2. Use `--plt-dialect hpgl` to generate classic
HP-GL.

### DXF → SVG

`ELLIPSE` and `SPLINE` are approximated by segments when needed. Precision is
controlled by `--curve-tolerance-mm`. The generated SVG uses millimeter
dimensions and preserves colors, widths, pens, text, circles, arcs, and
polylines.

### DXF → PNG

Rasterizes the drawing with the same colors and stroke widths as SVG export.
Use `--png-dpi` to control resolution.

### DXF → PDF

Vector export via intermediate SVG. Use `--pdf-stroke-scale` to adjust stroke
width (default `1`).

## PLT/HP-GL input

Possible outputs: **DXF** (default), **SVG**, **PNG**, and **PDF**.

Classic HP-GL and HP-GL/2 are supported, including files with concatenated
commands, PCL preambles, and compressed `PE` coordinates. Files with the
`.hpgl` extension are treated the same as `.plt`.

### PLT → DXF

The generated DXF is ASCII R12 and uses millimeters. By default:

- `40` HP-GL units equal `1 mm` (`1016` units per inch, default);
- pens are exported as layers `PEN_001`, `PEN_002`, etc.;
- pen colors and widths are preserved when declared;
- paths become `POLYLINE`;
- circles, arcs, and text use native DXF entities when possible.

Use `--single-layer` to place all entities on layer `0`.

### PLT → SVG

HP-GL coordinates are converted to millimeters according to `--units-per-mm`
or `--units-per-inch`. The SVG writer preserves colors, widths, pens, text,
circles, arcs, and polylines.

### PLT → PNG

Same visual appearance as SVG export, converted to a bitmap with a transparent
background.

### PLT → PDF

Same vector appearance as SVG export, written to a PDF page.

## SVG/SVF input

Possible outputs: **DXF** (default), **PLT**, **PNG**, and **PDF**.

Files with the `.svf` extension are accepted as an SVG alias for compatibility
with the indicated spelling, as long as the content is SVG XML.

### SVG reading

The SVG reader accepts units in `mm`, `cm`, `in`, `pt`, `pc`, and CSS pixels at
96 DPI. `width`, `height`, and `viewBox` are used to convert the drawing to
millimeters.

Supported elements:

- `line`, `polyline`, `polygon`, and `rect`, including rounded corners;
- `circle` and `ellipse`;
- `path` with commands `M`, `L`, `H`, `V`, `C`, `S`, `Q`, `T`, `A`, and `Z`,
  absolute or relative;
- `text`;
- `g` groups, `a` links, and `symbol`;
- `matrix`, `translate`, `scale`, `rotate`, `skewX`, and `skewY` transforms;
- hexadecimal colors, basic names, `rgb()`, inline styles, and
  `stroke-width`.

Bézier curves, elliptical arcs, and ellipses are approximated as polylines when
the destination has no equivalent entity. Precision is controlled by
`--curve-tolerance-mm`.

### SVG → DXF

The generated DXF is ASCII R12 and uses millimeters. Pens and stroke styles
become layers `PEN_001`, `PEN_002`, etc., unless `--single-layer` is used.

### SVG → PLT

Curves without a direct HP-GL equivalent are approximated as paths. The default
output is HP-GL/2; use `--plt-dialect hpgl` for classic HP-GL.

### SVG → PNG

Rasterizes the drawing interpreted from the input SVG. Text depends on fonts
installed on the system.

### SVG → PDF

Vector export from the interpreted SVG input. Text depends on fonts installed
on the system.

## PNG output

PNG is available **only as an output format**, from DXF, PLT, or SVG input.

The converter generates an intermediate SVG using the same logic as
[`svg_writer.rs`](src/svg_writer.rs) and rasterizes it with resvg. The resulting
image has a transparent background, preserves colors, and respects `--png-dpi`
(default `96`), `--png-stroke-scale` (default `3`), and optionally
`--png-max-size` to limit the longest side (ideal for thumbnails).

```bash
plotconvert --to png drawing.dxf
plotconvert --to png --png-dpi 300 pattern.plt
plotconvert --to png --png-max-size 512 drawing.dxf
plotconvert drawing.svg -o preview.png
```

## PDF output

PDF is available **only as an output format**, from DXF, PLT, or SVG input.

The converter generates an intermediate SVG using the same logic as
[`svg_writer.rs`](src/svg_writer.rs) and converts it to vector PDF with
svg2pdf. Strokes and colors remain scalable; use `--pdf-stroke-scale`
(default `1`) to adjust stroke width.

```bash
plotconvert --to pdf drawing.dxf
plotconvert --to pdf --pdf-stroke-scale 2 pattern.plt
plotconvert drawing.svg -o preview.pdf
```

## Building

```bash
cargo build --release
cargo test
```

The Linux binary is created at `target/release/plotconvert`. For Windows,
build the same project for the `x86_64-pc-windows-gnu` target or use the
release workflow.

Ready-made artifacts are produced at:

```text
dist/plotconvert-linux-x86_64
dist/plotconvert-windows-x86_64.exe
```

## GitHub Releases

Pre-built Linux and Windows binaries are published automatically when the
`version` field in [`Cargo.toml`](Cargo.toml) changes and the commit is pushed
to `main`.

1. Bump `version` in `Cargo.toml`.
2. Commit and push to `main`.
3. GitHub Actions builds both platforms and creates release `vX.Y.Z` if it does
   not exist yet.
4. Download the binaries from the repository [Releases](https://github.com/rhafaelcm/plotconvert/releases) page:

   - `plotconvert-linux-x86_64`
   - `plotconvert-windows-x86_64.exe`

You can also trigger the workflow manually from the Actions tab
(`workflow_dispatch`). If a release for the current version already exists, the
workflow skips building and publishing.

## License

This project is distributed under the [MIT](LICENSE) license.
