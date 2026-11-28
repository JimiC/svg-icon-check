use std::path::PathBuf;

use clap::{ArgAction, Args, CommandFactory, Error, Parser, crate_authors, error};
use tiny_skia::ColorU8;

use crate::constants::MAX_BACKGROUNDS;
use crate::utils::relative_luminance;

#[derive(Args)]
pub(crate) struct PreviewOptions {
    #[arg(
        short = 'm',
        long = "manifest",
        display_order = 2,
        help = "Output a manifest file (name: manifest.tsv)"
    )]
    pub(crate) manifest: bool,
    #[arg(
        short = 't',
        long = "theme-mode",
        display_order = 6,
        help = "Output preview icons for dark and light mode"
    )]
    pub(crate) theme_mode: bool,
}

#[derive(Args)]
pub(crate) struct BackgroundOptions {
    #[arg(
        short = 's',
        long = "sort-colors",
        display_order = 5,
        help = "Sort background colors by relative luminance, darkest first"
    )]
    pub(crate) sort_colors: bool,
    #[arg(
        long = "white-last",
        display_order = 8,
        help = "Place the white background color last in the list of backgrounds"
    )]
    pub(crate) white_last: bool,
}

#[derive(Parser)]
#[command(
    about,
    author = crate_authors!("\n  "),
    version,
    override_usage = "svg-icon-check <INPUT>... [OPTIONS]",
    help_template = concat!(
        "Description:\n {about}\n\n",
        "Authors:\n  {author}\n\n",
        "Usage:\n  {usage}\n\n",
        "Arguments:\n{positionals}\n\n",
        "Options:\n{options}"
    )
)]
pub(crate) struct Cli {
    #[arg(
        required = true,
        help = "One or more SVG paths, or .txt files containing one SVG path per line"
    )]
    pub(crate) input: Vec<PathBuf>,

    #[arg(
        short = 'b',
        long = "background",
        display_order = 1,
        action = ArgAction::Append,
        num_args = 1,
        value_name = "R,G,B",
        value_parser = parse_rgb,
        help = format!("Background color in RGB format, up to {MAX_BACKGROUNDS} times (e.g. 255,0,0)")
    )]
    pub(crate) backgrounds: Vec<ColorU8>,

    #[arg(
        short = 'n',
        long = "name-pattern",
        display_order = 3,
        default_value = "{name}",
        default_value_if("theme_mode", "true", "{name}-{mode}"),
        help = "Output filename pattern; requires '{name}' and, in theme mode, '{mode}'"
    )]
    pub(crate) name_pattern: String,

    #[arg(
        short = 'o',
        long = "output",
        display_order = 4,
        default_value = "output",
        help = "Path to the output directory"
    )]
    pub(crate) output: PathBuf,

    #[arg(
        short = 'w',
        long = "icon-width",
        display_order = 7,
        default_value = "200",
        value_parser = parse_icon_width,
        help = "Width of each icon section in pixels (1-256)"
    )]
    pub(crate) icon_width: u32,

    #[command(flatten)]
    pub(crate) preview: PreviewOptions,

    #[command(flatten)]
    pub(crate) background: BackgroundOptions,
}

impl Cli {
    pub(crate) fn validate_name_pattern(&self) -> Result<(), Error> {
        parse_name_pattern(&self.name_pattern, self.preview.theme_mode).map_err(|message| {
            Self::command().error(
                error::ErrorKind::ValueValidation,
                format!(
                    "invalid value '{}' for '--name-pattern <NAME_PATTERN>': {message}",
                    self.name_pattern
                ),
            )
        })
    }
}

fn parse_name_pattern(value: &str, theme_mode: bool) -> Result<(), String> {
    if value.is_empty() || value.contains(['/', '\\', '\0']) {
        return Err(
            "pattern must not, be empty, contain path separators or null characters".to_owned(),
        );
    }
    if !value.contains("{name}") {
        return Err("pattern must contain {name}".to_owned());
    }
    if theme_mode && !value.contains("{mode}") {
        return Err("pattern must contain {mode} when --theme-mode is enabled".to_owned());
    }
    Ok(())
}

fn parse_icon_width(value: &str) -> Result<u32, String> {
    let width = value
        .parse::<u32>()
        .map_err(|_| "enter a whole number from 1 to 256")?;
    if !(1..=256).contains(&width) {
        return Err("width must be between 1 and 256 pixels".to_owned());
    }
    Ok(width)
}

fn parse_rgb(value: &str) -> Result<ColorU8, String> {
    let mut components = value.split(',');
    let (Some(red), Some(green), Some(blue), None) = (
        components.next(),
        components.next(),
        components.next(),
        components.next(),
    ) else {
        return Err("expected exactly three comma-separated RGB values".to_owned());
    };

    let parse_component = |component: &str| {
        component
            .trim()
            .parse::<u8>()
            .map_err(|_| "RGB values must be integers from 0 to 255".to_owned())
    };
    let red = parse_component(red)?;
    let green = parse_component(green)?;
    let blue = parse_component(blue)?;

    Ok(ColorU8::from_rgba(red, green, blue, u8::MAX))
}

pub(crate) fn resolve_background_colors(
    backgrounds: &[ColorU8],
    sort_colors: bool,
    white_last: bool,
) -> anyhow::Result<Vec<ColorU8>, Error> {
    if backgrounds.len() > MAX_BACKGROUNDS {
        let command = Cli::command();
        let literal = *command.get_styles().get_literal();
        return Err(Error::raw(
            error::ErrorKind::TooManyValues,
            format!(
                "too many values for '{literal}--background <R,G,B>{literal:#}': \
                 at most {MAX_BACKGROUNDS} background option occurrences are allowed\n\n\
                 For more information, try '{literal}--help{literal:#}'.\n"
            ),
        )
        .with_cmd(&command));
    }

    let mut backgrounds = backgrounds.to_vec();
    if sort_colors {
        backgrounds.sort_by(|a, b| relative_luminance(*a).total_cmp(&relative_luminance(*b)));
    }

    let default_colors = [
        ColorU8::from_rgba(0, 0, 0, u8::MAX),
        ColorU8::from_rgba(255, 255, 255, u8::MAX),
    ];
    let mut background_colors = default_colors.to_vec();

    for background in backgrounds {
        let skip_reason = if default_colors.contains(&background) {
            Some("already included by default")
        } else if background_colors.contains(&background) {
            Some("duplicate")
        } else {
            None
        };

        if let Some(reason) = skip_reason {
            eprintln!(
                "⚠ Skipping {reason} background color: {},{},{}",
                background.red(),
                background.green(),
                background.blue()
            );
            continue;
        }

        background_colors.push(background);
    }

    if white_last {
        let white = default_colors[1];
        background_colors.retain(|background| *background != white);
        background_colors.push(white);
    }

    Ok(background_colors)
}
