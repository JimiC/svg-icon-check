use super::*;

#[test]
fn builds_svg_and_theme_jobs_from_paths_and_lists() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let svg = dir.path().join("icon.SvG");
    fs::write(&svg, "<svg/>")?;
    let list = dir.path().join("icons.txt");
    fs::write(&list, "\u{feff} \n icon.SvG \n\n")?;

    let jobs = get_preview_jobs(&[svg, list], "{name}-{mode}", true)?;
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].png_name, "icon-dark.png");
    assert!(matches!(jobs[0].theme_mode, Some(ThemeMode::Dark)));
    assert_eq!(jobs[1].png_name, "icon-light.png");
    assert!(matches!(jobs[1].theme_mode, Some(ThemeMode::Light)));
    assert!(jobs.iter().all(|job| job.svg.is_absolute()));
    Ok(())
}

#[test]
fn handles_svgz_and_case_insensitive_output_collisions() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let first = dir.path().join("Icon.svgz");
    let second = dir.path().join("icon.svg");
    fs::write(&first, "")?;
    fs::write(&second, "")?;
    let jobs = get_preview_jobs(&[first, second], "{name}", false)?;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].png_name, "Icon.png");
    assert!(jobs[0].theme_mode.is_none());
    Ok(())
}

#[test]
fn propagates_filestem_failures() -> anyhow::Result<()> {
    let input = PathBuf::from("icon.svg");
    let fail = |path: &Path| -> anyhow::Result<String> {
        assert_eq!(path, input);
        anyhow::bail!("injected filestem failure")
    };
    let result = get_preview_jobs_with(std::slice::from_ref(&input), "{name}", false, &fail);
    let Err(error) = result else {
        anyhow::bail!("filestem failure must be reported");
    };
    assert!(error.to_string().contains("injected filestem failure"));
    Ok(())
}

#[test]
fn rejects_unsupported_paths_missing_files_and_invalid_list_entries() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let unsupported = dir.path().join("icon.png");
    fs::write(&unsupported, "")?;
    assert!(get_preview_jobs(&[unsupported], "{name}", false).is_err());

    let no_extension = dir.path().join("icon");
    assert!(get_preview_jobs(&[no_extension], "{name}", false).is_err());

    let missing_list = dir.path().join("missing.txt");
    assert!(get_preview_jobs(&[missing_list], "{name}", false).is_err());

    let list = dir.path().join("bad.txt");
    fs::write(&list, "icon.png\n")?;
    assert!(get_preview_jobs(std::slice::from_ref(&list), "{name}", false).is_err());

    fs::write(&list, std::path::MAIN_SEPARATOR_STR)?;
    let error = get_preview_jobs(&[list], "{name}", false)
        .err()
        .context("a root path in the list must fail")?;
    assert!(error.to_string().contains("Path has no extension"));
    Ok(())
}

#[test]
fn rejects_list_svg_with_no_filename_extension() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let list = dir.path().join("icons.txt");
    fs::write(&list, "extensionless\n")?;
    assert!(get_preview_jobs(&[list], "{name}", false).is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn reports_failure_to_make_a_relative_path_absolute() -> anyhow::Result<()> {
    const CHILD: &str = "SVG_ICON_CHECK_CWD_FAILURE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "jobs::tests::reports_failure_to_make_a_relative_path_absolute",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()?;
        assert!(
            output.status.success(),
            "child test failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(());
    }

    let original_dir = std::env::current_dir()?;
    let removed_dir = tempfile::tempdir()?;
    std::env::set_current_dir(removed_dir.path())?;
    fs::remove_dir(removed_dir.path())?;

    let result = get_preview_jobs(&[PathBuf::from("icon.svg")], "{name}", false);
    let output_result = crate::output::finalize_output(Path::new("."), PathBuf::from("output"));
    std::env::set_current_dir(original_dir)?;

    assert!(result.is_err());
    assert!(output_result.is_err());
    Ok(())
}
