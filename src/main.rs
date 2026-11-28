use anyhow::{Context, Result};

mod cli;
mod constants;
mod jobs;
mod output;
mod render;
mod utils;

use clap::Parser;

use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.validate_name_pattern()
        .unwrap_or_else(|error| error.exit());
    let preview_jobs =
        jobs::get_preview_jobs(&cli.input, &cli.name_pattern, cli.preview.theme_mode)?;
    let background_colors = cli::resolve_background_colors(
        &cli.backgrounds,
        cli.background.sort_colors,
        cli.background.white_last,
    )
    .unwrap_or_else(|error| error.exit());
    let temp_dir = tempfile::tempdir().context("Cannot create temporary directory")?;
    render::create_previews(
        &preview_jobs,
        &background_colors,
        cli.icon_width,
        temp_dir.path(),
    )?;
    let output_dir = output::finalize_output(temp_dir.path(), cli.output)?;
    if cli.preview.manifest {
        output::create_manifest(&preview_jobs, &output_dir)?;
        println!(
            "✔ Created manifest file at {}",
            utils::normalize_display_path(&output_dir.join("manifest.tsv")).display()
        );
    }

    if let Err(error) = temp_dir.close() {
        eprintln!("⚠ Cannot remove temp directory: {error}");
    }

    println!(
        "✔ Generated {} preview{} in {}",
        preview_jobs.len(),
        if preview_jobs.len() == 1 { "" } else { "s" },
        utils::normalize_display_path(&output_dir).display()
    );

    Ok(())
}
