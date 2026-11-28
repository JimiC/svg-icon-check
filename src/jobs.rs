use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf, absolute},
};

use anyhow::{Context, Result, bail};

use crate::utils::{get_file_extension, get_filestem, is_svg_extension, normalize_display_path};

pub(crate) struct PreviewJob {
    pub(crate) svg: PathBuf,
    pub(crate) png_name: String,
    pub(crate) theme_mode: Option<ThemeMode>,
}

#[derive(Clone, Copy)]
pub(crate) enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

pub(crate) fn get_preview_jobs(
    inputs: &[PathBuf],
    name_pattern: &str,
    theme_mode: bool,
) -> Result<Vec<PreviewJob>> {
    get_preview_jobs_with(inputs, name_pattern, theme_mode, &get_filestem)
}

fn get_preview_jobs_with(
    inputs: &[PathBuf],
    name_pattern: &str,
    theme_mode: bool,
    filestem: &dyn Fn(&Path) -> Result<String>,
) -> Result<Vec<PreviewJob>> {
    let mut output_names: HashMap<String, PathBuf> = HashMap::new();
    let mut preview_jobs = Vec::new();
    let mut add_file = |path: &PathBuf| -> Result<()> {
        let extension = get_file_extension(path)?;
        if !is_svg_extension(&extension) {
            bail!(
                "Unsupported file type: {}",
                normalize_display_path(path).display()
            );
        }

        let filestem = filestem(path)?;
        let modes: &[Option<ThemeMode>] = if theme_mode {
            &[Some(ThemeMode::Dark), Some(ThemeMode::Light)]
        } else {
            &[None]
        };
        let names: Vec<_> = modes
            .iter()
            .map(|&mode| {
                let pattern = mode.map_or_else(
                    || name_pattern.to_owned(),
                    |mode| name_pattern.replace("{mode}", mode.name()),
                );
                (
                    format!("{}.png", pattern.replace("{name}", &filestem)),
                    mode,
                )
            })
            .collect();
        for (png_name, _) in &names {
            if let Some(existing) = output_names.get(&png_name.to_lowercase()) {
                eprintln!(
                    "⚠ Skipping {} (same output name as {})",
                    normalize_display_path(path).display(),
                    normalize_display_path(existing).display()
                );
                return Ok(());
            }
        }
        let absolute_path = absolute(path).with_context(|| {
            format!(
                "Cannot resolve path: {}",
                normalize_display_path(path).display()
            )
        })?;
        for (png_name, theme_mode) in names {
            output_names.insert(png_name.to_lowercase(), path.clone());
            preview_jobs.push(PreviewJob {
                svg: absolute_path.clone(),
                png_name,
                theme_mode,
            });
        }
        Ok(())
    };

    for input in inputs {
        let extension = get_file_extension(input)?;
        match extension.as_str() {
            ext if is_svg_extension(ext) => add_file(input)?,
            ext if ext.eq_ignore_ascii_case("txt") => {
                let base_dir = input.parent().unwrap_or(Path::new("."));
                let contents = fs::read_to_string(input).with_context(|| {
                    format!(
                        "Cannot read TXT file: {}",
                        normalize_display_path(input).display()
                    )
                })?;

                contents
                    .trim_start_matches('\u{feff}')
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .try_for_each(|line| add_file(&base_dir.join(line)))?;
            }
            _ => bail!(
                "Unsupported file type: {}",
                normalize_display_path(input).display()
            ),
        }
    }

    Ok(preview_jobs)
}
