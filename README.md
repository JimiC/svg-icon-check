# SVG Icon Check

Small utility to check how an SVG icon looks like on different backgrounds.

The utility has been created to help with the development
of the [vscode-icons](https://github.com/vscode-icons/vscode-icons) project.

## Quick Start

```bash
# Simple usage - renders icon on black and white backgrounds (default)
svg-icon-check icon.svg

# Multiple files
svg-icon-check icon1.svg icon2.svg icon3.svg

# Using a text file with icon paths (one per line)
svg-icon-check paths.txt

# SVG and text file extensions are case-insensitive; compressed SVGZ is supported
svg-icon-check icon.SVG icon.svgz paths.TXT

# Custom output directory
svg-icon-check icon.svg -o ./output_icons

# Generate a manifest alongside the previews
svg-icon-check icon.svg --manifest

# Generate separate dark and light background previews
svg-icon-check icon.svg --theme-mode

# Customize preview filenames
svg-icon-check icon.svg --name-pattern '{name}-preview'

# Combine theme mode with a custom pattern (dark-icon-preview.png, light-icon-preview.png)
svg-icon-check icon.svg --theme-mode --name-pattern '{mode}-{name}-preview'

# Custom icon size
svg-icon-check icon.svg -w 128

# Multiple background colors
svg-icon-check icon.svg -b 30,30,30 -b 253,246,227

# Combine all options
svg-icon-check icon.svg -o ./results -w 64 -b 30,30,30 -b 40,42,54 -b 253,246,227
```

## Options

| Option          | Short | Long             | Type       | Default       | Description                                                             |
| --------------- | ----- | ---------------- | ---------- | ------------- | ----------------------------------------------------------------------- |
| `input`         | -     | -                | Path(s)    | -             | One or more SVG paths, or .txt files containing one SVG path per line   |
| `background`    | `-b`  | `--background`   | RGB value  | black & white | Background color in RGB format, up to 16 times (e.g. 255,0,0)           |
| `manifest`      | `-m`  | `--manifest`     | Flag       | off           | Output a manifest file (name: manifest.tsv)                             |
| `name_pattern`  | `-n`  | `--name-pattern` | Pattern    | `{name}`      | Output filename pattern; requires `{name}` and, in theme mode, `{mode}` |
| `output`        | `-o`  | `--output`       | Path       | `output`      | Path to the output directory                                            |
| `sort_colors`   | `-s`  | `--sort-colors`  | Flag       | off           | Sort backgrounds by relative luminance, darkest first                   |
| `theme_mode`    | `-t`  | `--theme-mode`   | Flag       | off           | Outputs preview icons for dark and light mode                           |
| `icon_width`    | `-w`  | `--icon-width`   | Number     | `200`         | Width of each icon section in pixels (1-256)                            |
| `white_last`    | -     | `--white-last`   | Flag       | off           | Place the default white background last                                 |

## Usage

### Basic Syntax

```bash
svg-icon-check <INPUT>... [OPTIONS]
```

**Input arguments:**

- One or more SVG file paths
- One or more .txt files containing SVG paths (one per line)
- Mix of both SVG and .txt files

SVG (`.svg` and compressed `.svgz`) and TXT file extensions are case-insensitive.
Paths listed in TXT files must also have an `.svg` or `.svgz` extension.

### Path Resolution

Input paths given directly on the command line are resolved relative to the
current working directory. Paths listed in a TXT file are resolved relative to
that TXT file's directory. A relative output path passed with `-o` is resolved
relative to the current working directory.

Output filenames are compared case-insensitively. If inputs
across arguments and text files produce the same name,
the first is used with its original spelling and later duplicates are skipped
with a warning.

Specify one RGB color per `-b`/`--background` occurrence and repeat the option
for additional colors.

The tool generates PNG files with your icons rendered on the default backgrounds.

**Example output:**

![01](docs/example1.png)

### Default Behavior

By default, the icon is rendered on a **black and white background**.
The output is saved as PNG files in the `output` directory. Each PNG contains
the backgrounds in one horizontal strip: two icon-width sections for the
default black and white backgrounds.

## Naming pattern (`-n` option)

Use `-n` or `--name-pattern` to customize preview filenames. The pattern must
contain `{name}`, which is replaced with the input filename without its
extension. Each occurrence is replaced, and `.png` is appended automatically.
Patterns cannot contain path separators or null characters.

```bash
# icon.svg becomes icon-preview.png
svg-icon-check icon.svg --name-pattern '{name}-preview'

# icon.svg becomes thumbnail-icon.png
svg-icon-check icon.svg -n 'thumbnail-{name}'

# Default: icon.svg becomes icon.png
svg-icon-check icon.svg -n '{name}'
```

## Theme mode (`-t` option)

Use `-t` or `--theme-mode` to generate separate previews for dark and light
backgrounds. The default naming pattern becomes `{name}-{mode}`, producing
`icon-dark.png` and `icon-light.png`. A custom pattern must contain both
`{name}` and `{mode}`; each `{mode}` is replaced by `dark` or `light`.

```bash
# icon.svg becomes dark-icon-preview.png and light-icon-preview.png
svg-icon-check icon.svg --theme-mode -n '{mode}-{name}-preview'
```

Backgrounds are classified by sRGB relative luminance: values greater than
0.179128784747792 are light; all others are dark. Each variant preserves the
order of its backgrounds after sorting and `--white-last` are applied. The
default black and white backgrounds ensure both variants are generated.
When a manifest is requested, it contains one entry for each variant.

## Manifest (`-m` option)

Use `-m` or `--manifest` to generate `manifest.tsv` in the output directory.
Each generated preview has one tab-separated line containing the SVG filename
without its extension, the SVG file size in bytes, and the preview filename:

```text
icon<TAB>123<TAB>icon.png
```

Entries follow input order and exclude skipped duplicates. SVG and preview
filenames containing tabs or line breaks cannot be included in the manifest.
The manifest is published only after the previews have been copied
successfully. If manifest creation fails, the previews remain in the output
directory and any existing manifest is left unchanged.

## Background Colors

Add custom background colors with `-b` or `--background`:

> One comma-separated RGB color per occurrence, up to 16 times (components 0-255)

```bash
svg-icon-check fixtures/icons/file_type_access.svg -b 245,245,220 -b 30,30,30
svg-icon-check fixtures/icons/file_type_access.svg --background 245,245,220 --background 30,30,30
```

**Example output:**

![02](docs/example2.png)

Duplicate values count toward the 16-occurrence limit even though they are
skipped. The default black and white backgrounds do not count toward this limit.

The SVG is rendered once, and each background is shown as a separate section
in the same horizontal output image:

```bash
svg-icon-check fixtures/icons/file_type_access.svg \
  -b 30,30,30 -b 40,42,54 -b 46,52,64 -b 47,54,59 -b 39,40,34 \
  -b 60,63,65 -b 88,110,117 -b 89,91,79 -b 120,120,120 \
  -b 253,246,227 -b 236,239,244 -b 242,242,242 -b 250,250,250
```

**Example output:**

![03](docs/example3.png)

Use `-s` or `--sort-colors` to sort custom backgrounds by sRGB relative
luminance, from darkest to lightest. The default black and white backgrounds
remain first. Without sorting, custom backgrounds retain their input order:

```bash
svg-icon-check fixtures/icons/file_type_access.svg --sort-colors \
  -b 253,246,227 -b 46,52,64 -b 242,242,242 -b 39,40,34 \
  -b 30,30,30 -b 250,250,250 -b 236,239,244 -b 40,42,54
```

**Example output:**

![Sorted backgrounds](docs/example4.png)

Use `--white-last` to move the default white background to the end, whether or
not sorting is enabled:

```bash
svg-icon-check fixtures/icons/file_type_access.svg --white-last \
  -b 30,30,30 -b 40,42,54 -b 46,52,64 -b 39,40,34 \
  -b 253,246,227 -b 236,239,244 -b 242,242,242 -b 250,250,250
```

**Example output:**

![White background last](docs/example5.png)

Use `-t` or `--theme-mode` to generate separate previews for dark and light
backgrounds:

```bash
svg-icon-check fixtures/icons/file_type_access.svg --theme-mode \
  -b 30,30,30 -b 40,42,54 -b 46,52,64 -b 39,40,34 \
  -b 253,246,227 -b 236,239,244 -b 242,242,242 -b 250,250,250
```

**Example output:**

![Dark theme backgrounds](docs/example6.png)
![Light theme backgrounds](docs/example7.png)

Using all the VS Code theme colors and combining the output, sizing, naming,
manifest, sorting, and theme options:

```bash
svg-icon-check fixtures/changed_files.txt \
  --output ./results \
  --icon-width 200 \
  --name-pattern '{name}-{mode}' \
  --manifest \
  --sort-colors \
  --theme-mode \
  --white-last \
  -b 30,30,30 -b 40,42,54 -b 46,52,64 -b 47,54,59 -b 39,40,34 \
  -b 60,63,65 -b 88,110,117 -b 89,91,79 -b 120,120,120 \
  -b 253,246,227 -b 236,239,244 -b 242,242,242 -b 250,250,250
```

**Example output:**

![Dark theme backgrounds with all options](docs/example8.png)
![Light theme backgrounds with all options](docs/example9.png)

## Output Directory (`-o` option)

Specify a custom output directory for the generated PNG files:

```bash
svg-icon-check icon.svg -o ./my_output
```

Existing files in the output directory are never removed. Generated PNGs and
the manifest, when requested, overwrite files with the same names, while files
not generated by the current run, including previews from earlier runs, are
left in place. Use a fresh output directory when you need to distinguish the
current run's previews from older ones.

If copying a preview fails, the tool attempts to remove newly copied previews
and restore overwritten files. Rollback failures are reported, and backups are
retained for recovery. A crash or forced termination can still leave partial
output.

## Icon Width (`-w` option)

Customize the width of the output icon using the `-w` option:

```bash
svg-icon-check icon.svg -w 128
```

The width value must be between 1 and 256 and sets the size of each icon section
in pixels. By default,
each section is **200 pixels wide** and **200 pixels tall**. The SVG is scaled
to fit within that square while preserving its aspect ratio.

```bash
svg-icon-check icon.svg -w 64 -b 30,30,30 -b 253,246,227
```

For example, the last command produces a PNG with 256 pixels of width:
four 64-pixel sections (black, white, and the two custom colors).

## Batch Processing with Text Files

You can process multiple icons from a text file by listing one SVG path per line:

```txt
path/to/icon1.svg
path/to/icon2.svg
path/to/icon3.svg
```

Then run:

```bash
svg-icon-check icons.txt -o ./batch_output -w 100
```

## VS Code Theme Background Colors

Reference colors from popular VS Code themes for consistent testing.

### Dark Mode Colors

| Name                                  | RGB            | Hex       |
| ------------------------------------- | -------------- | --------- |
| **Obsidian** (Very dark gray)         | (30, 30, 30)   | `#1E1E1E` |
| **Dracula Dark** (Purple-tinted dark) | (40, 42, 54)   | `#282A36` |
| **Nord Dark** (Cool gray)             | (46, 52, 64)   | `#2E3440` |
| **Solarized Dark** (Muted brown-gray) | (47, 54, 59)   | `#2F363B` |
| **Monokai Dark** (Warm deep gray)     | (39, 40, 34)   | `#272822` |
| **Dim Gray** (Soft gray)              | (60, 63, 65)   | `#3C3F41` |
| **Solarized Base** (Dark teal-gray)   | (88, 110, 117) | `#586E75` |
| **Monokai Medium** (Olive-brown)      | (89, 91, 79)   | `#595B4F` |

## Light Mode Colors

| Name                                  | RGB             | Hex       |
| ------------------------------------- | --------------- | --------- |
| **VS Code Default Gray**              | (120, 120, 120) | `#787878` |
| **Solarized Light** (Pale yellow)     | (253, 246, 227) | `#FDF6E3` |
| **Nord Light** (Icy blue-white)       | (236, 239, 244) | `#ECEFF4` |
| **VS Code Light+** (Soft beige-white) | (242, 242, 242) | `#F2F2F2` |
| **Almost White** (Warm light gray)    | (250, 250, 250) | `#FAFAFA` |

## Test Coverage

To measure test coverage, install `cargo-llvm-cov` and the LLVM tools component
for the active Rust toolchain:

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

Then run:

```bash
cargo llvm-cov --no-cfg-coverage --tests --summary-only --ignore-filename-regex '(^|/)test/'
```
