use super::*;

const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="2" height="1" fill="#ff0000"/></svg>"##;

#[test]
fn fits_landscape_and_portrait_svg_to_square_canvas() {
    let landscape = Transform::from_scale(50.0, 50.0).post_translate(0.0, 25.0);
    let portrait = Transform::from_scale(50.0, 50.0).post_translate(25.0, 0.0);
    assert_eq!(fit_transform(2.0, 1.0, 100.0), landscape);
    assert_eq!(fit_transform(1.0, 2.0, 100.0), portrait);
}

#[test]
fn rejects_overflowing_preview_dimensions() -> anyhow::Result<()> {
    let count = usize::try_from(u32::MAX / 256 + 1)?;
    let error = preview_width(256, count)
        .err()
        .context("overflowing width must fail")?;
    assert!(error.to_string().contains("dimensions are too large"));

    if let Ok(count) = usize::try_from(u64::from(u32::MAX) + 1) {
        let error = preview_width(1, count)
            .err()
            .context("too many colors must fail")?;
        assert!(error.to_string().contains("Too many background colors"));
    }

    let tree = Tree::from_data(SVG, &Options::default())?;
    let count = usize::try_from(u32::MAX / 1024 + 1)?;
    let colors = vec![ColorU8::from_rgba(0, 0, 0, 255); count];
    let error = render_with_backgrounds(&tree, &colors, 1024)
        .err()
        .context("overflowing rendered width must fail")?;
    assert!(error.to_string().contains("dimensions are too large"));
    Ok(())
}

#[test]
fn reports_invalid_stripe_dimensions() -> anyhow::Result<()> {
    let icon = Pixmap::new(1, 1).context("cannot create test icon")?;
    let combined = Pixmap::new(1, 1).context("cannot create test preview")?;
    let color = ColorU8::from_rgba(0, 0, 0, 255);

    let error = draw_backgrounds(&icon, combined.clone(), &[color], u32::MAX, 1.0)
        .err()
        .context("oversized stripe must fail")?;
    assert!(error.to_string().contains("Icon width is too large"));

    let error = draw_backgrounds(&icon, combined, &[color], 1, f32::NAN)
        .err()
        .context("non-finite stripe must fail")?;
    assert!(error.to_string().contains("Invalid background rectangle"));
    Ok(())
}

#[test]
fn renders_background_stripes_and_creates_theme_previews() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let svg = dir.path().join("icon.svg");
    fs::write(&svg, SVG)?;
    let jobs = [
        PreviewJob {
            svg: svg.clone(),
            png_name: "dark.png".to_owned(),
            theme_mode: Some(ThemeMode::Dark),
        },
        PreviewJob {
            svg: svg.clone(),
            png_name: "light.png".to_owned(),
            theme_mode: Some(ThemeMode::Light),
        },
        PreviewJob {
            svg,
            png_name: "all.png".to_owned(),
            theme_mode: None,
        },
    ];
    let colors = [
        ColorU8::from_rgba(0, 0, 0, 255),
        ColorU8::from_rgba(255, 255, 255, 255),
        ColorU8::from_rgba(200, 200, 200, 255),
    ];
    let tree = Tree::from_data(SVG, &Options::default())?;
    let rendered = render_with_backgrounds(&tree, &colors, 8)?;
    assert_eq!((rendered.width(), rendered.height()), (24, 8));

    create_previews(&jobs, &colors, 8, dir.path())?;
    for name in ["dark.png", "light.png", "all.png"] {
        assert!(dir.path().join(name).is_file());
    }
    Ok(())
}

#[test]
fn reports_invalid_inputs_and_zero_sized_images() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let invalid = dir.path().join("invalid.svg");
    fs::write(&invalid, "not SVG")?;
    let job = PreviewJob {
        svg: invalid,
        png_name: "invalid.png".to_owned(),
        theme_mode: None,
    };
    assert!(create_previews(&[job], &[], 8, dir.path()).is_err());

    let tree = Tree::from_data(SVG, &Options::default())?;
    assert!(render_with_backgrounds(&tree, &[], 8).is_err());
    assert!(render_with_backgrounds(&tree, &[], 0).is_err());
    let error = render_with_backgrounds(&tree, &[], u32::MAX)
        .err()
        .context("an oversized icon must fail")?;
    assert!(error.to_string().contains("icon pixmap"));

    let missing = PreviewJob {
        svg: dir.path().join("missing.svg"),
        png_name: "missing.png".to_owned(),
        theme_mode: None,
    };
    assert!(create_previews(&[missing], &[], 8, dir.path()).is_err());

    let svg = dir.path().join("valid.svg");
    fs::write(&svg, SVG)?;
    let valid_job = || PreviewJob {
        svg: svg.clone(),
        png_name: "valid.png".to_owned(),
        theme_mode: None,
    };
    let colors = [ColorU8::from_rgba(0, 0, 0, 255)];
    let error = create_previews(&[valid_job()], &colors, 0, dir.path())
        .err()
        .context("zero width must fail")?;
    assert!(error.to_string().contains("icon pixmap"));
    let error = create_previews(&[valid_job()], &colors, 8, &dir.path().join("missing"))
        .err()
        .context("missing output directory must fail")?;
    assert!(error.to_string().contains("Cannot save image"));
    Ok(())
}
