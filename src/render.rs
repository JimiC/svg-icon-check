use std::{fs, path::Path};

use anyhow::{Context, Result};
use rayon::prelude::*;
use resvg::usvg::{Options, Transform, Tree};
use tiny_skia::{Color, ColorU8, Paint, Pixmap, PixmapPaint, Rect};

use crate::{
    jobs::{PreviewJob, ThemeMode},
    utils::{is_light_background, normalize_display_path},
};

pub(crate) fn create_previews(
    preview_jobs: &[PreviewJob],
    background_colors: &[ColorU8],
    icon_width: u32,
    temp_dir: &Path,
) -> Result<()> {
    let (light_backgrounds, dark_backgrounds): (Vec<_>, Vec<_>) = background_colors
        .iter()
        .copied()
        .partition(|color| is_light_background(*color));
    let fontdb = {
        let mut options = Options::default();
        options.fontdb_mut().load_system_fonts();
        options.fontdb
    };

    preview_jobs.par_iter().try_for_each(|preview_job| {
        let svg_data = fs::read(&preview_job.svg).with_context(|| {
            format!(
                "Cannot read SVG file: {}",
                normalize_display_path(&preview_job.svg).display()
            )
        })?;

        let options = Options {
            resources_dir: preview_job.svg.parent().map(Path::to_path_buf),
            fontdb: fontdb.clone(),
            ..Options::default()
        };

        let tree = Tree::from_data(&svg_data, &options).with_context(|| {
            format!(
                "Cannot parse SVG file: {}",
                normalize_display_path(&preview_job.svg).display()
            )
        })?;

        let temp_path = temp_dir.join(&preview_job.png_name);
        let backgrounds = match preview_job.theme_mode {
            Some(ThemeMode::Dark) => &dark_backgrounds,
            Some(ThemeMode::Light) => &light_backgrounds,
            None => background_colors,
        };
        render_with_backgrounds(&tree, backgrounds, icon_width)?
            .save_png(temp_path)
            .context("Cannot save image")?;

        Ok(())
    })
}

fn fit_transform(svg_width: f32, svg_height: f32, canvas_width: f32) -> Transform {
    let scale = (canvas_width / svg_width).min(canvas_width / svg_height);
    let offset_x = (canvas_width - svg_width * scale) / 2.0;
    let offset_y = (canvas_width - svg_height * scale) / 2.0;

    Transform::from_scale(scale, scale).post_translate(offset_x, offset_y)
}

fn preview_width(icon_width: u32, color_count: usize) -> Result<u32> {
    let color_count =
        u32::try_from(color_count).context("Too many background colors for preview image")?;
    icon_width
        .checked_mul(color_count)
        .context("Preview image dimensions are too large")
}

fn render_with_backgrounds(
    tree: &Tree,
    background_colors: &[ColorU8],
    icon_width: u32,
) -> Result<Pixmap> {
    #[expect(clippy::cast_precision_loss)]
    let icon_size = icon_width as f32;
    let mut icon = Pixmap::new(icon_width, icon_width)
        .with_context(|| format!("Cannot create {icon_width}x{icon_width} icon pixmap"))?;
    let transform = fit_transform(tree.size().width(), tree.size().height(), icon_size);
    resvg::render(tree, transform, &mut icon.as_mut());

    let combined_width = preview_width(icon_width, background_colors.len())?;
    let combined = Pixmap::new(combined_width, icon_width)
        .with_context(|| format!("Cannot create {combined_width}x{icon_width} preview pixmap"))?;
    draw_backgrounds(&icon, combined, background_colors, icon_width, icon_size)
}

fn draw_backgrounds(
    icon: &Pixmap,
    mut combined: Pixmap,
    background_colors: &[ColorU8],
    icon_width: u32,
    icon_size: f32,
) -> Result<Pixmap> {
    let stripe_width_pixels =
        i32::try_from(icon_width).context("Icon width is too large for pixmap drawing")?;
    let mut paint = Paint {
        anti_alias: false,
        ..Paint::default()
    };
    let pixmap_paint = PixmapPaint::default();
    let mut pixmap_x = 0;

    for background in background_colors {
        #[expect(clippy::cast_precision_loss)]
        let background_rect_left = pixmap_x as f32;
        paint.set_color(Color::from_rgba8(
            background.red(),
            background.green(),
            background.blue(),
            background.alpha(),
        ));
        let rect = Rect::from_xywh(background_rect_left, 0.0, icon_size, icon_size)
            .context("Invalid background rectangle")?;
        combined.fill_rect(rect, &paint, Transform::identity(), None);
        combined.draw_pixmap(
            pixmap_x,
            0,
            icon.as_ref(),
            &pixmap_paint,
            Transform::identity(),
            None,
        );
        pixmap_x += stripe_width_pixels;
    }

    Ok(combined)
}
