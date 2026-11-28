use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tiny_skia::ColorU8;

use crate::constants::LIGHT_THRESHOLD;

pub(crate) fn get_filestem(path: &Path) -> Result<String> {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .with_context(|| {
            format!(
                "Path has no filename: {}",
                normalize_display_path(path).display()
            )
        })
}

pub(crate) fn get_filename(path: &Path) -> Result<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .with_context(|| {
            format!(
                "Path has no filename: {}",
                normalize_display_path(path).display()
            )
        })
}

pub(crate) fn get_file_extension(path: &Path) -> Result<String> {
    path.extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .with_context(|| {
            format!(
                "Path has no extension: {}",
                normalize_display_path(path).display()
            )
        })
}

pub(crate) fn normalize_display_path(path: &Path) -> PathBuf {
    path.components().collect()
}

pub(crate) fn is_svg_extension(extension: &str) -> bool {
    extension.eq_ignore_ascii_case("svg") || extension.eq_ignore_ascii_case("svgz")
}

pub(crate) fn relative_luminance(color: ColorU8) -> f64 {
    let linear = |component: u8| {
        let value = f64::from(component) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * linear(color.red()) + 0.7152 * linear(color.green()) + 0.0722 * linear(color.blue())
}

pub(crate) fn is_light_background(color: ColorU8) -> bool {
    relative_luminance(color) > LIGHT_THRESHOLD
}
