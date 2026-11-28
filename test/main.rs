use std::{fs, process::Command};

#[test]
fn cli_generates_single_and_multiple_previews_and_manifest() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let first = dir.path().join("first.svg");
    let second = dir.path().join("second.svg");
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="2" height="1" fill="#ff0000"/></svg>"##;
    fs::write(&first, svg)?;
    fs::write(&second, svg)?;
    let output = dir.path().join("output");

    let single = Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
        .arg(&first)
        .arg("--output")
        .arg(&output)
        .arg("--manifest")
        .output()?;
    assert!(
        single.status.success(),
        "{}",
        String::from_utf8_lossy(&single.stderr)
    );
    assert!(String::from_utf8_lossy(&single.stdout).contains("Generated 1 preview in"));
    assert!(output.join("first.png").is_file());
    assert!(output.join("manifest.tsv").is_file());

    fs::write(dir.path().join("icons.txt"), "first.svg\n")?;
    let relative_list = Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
        .current_dir(dir.path())
        .args(["icons.txt", "--output", "relative-output"])
        .output()?;
    assert!(
        relative_list.status.success(),
        "{}",
        String::from_utf8_lossy(&relative_list.stderr)
    );
    assert!(dir.path().join("relative-output/first.png").is_file());

    let multiple = Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
        .arg(&first)
        .arg(&second)
        .arg("--output")
        .arg(&output)
        .arg("--theme-mode")
        .arg("--name-pattern")
        .arg("{name}-{mode}")
        .output()?;
    assert!(
        multiple.status.success(),
        "{}",
        String::from_utf8_lossy(&multiple.stderr)
    );
    assert!(String::from_utf8_lossy(&multiple.stdout).contains("Generated 4 previews in"));
    assert!(output.join("first-dark.png").is_file());
    assert!(output.join("first-light.png").is_file());

    let invalid = Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
        .arg(&first)
        .arg("--name-pattern")
        .arg("invalid")
        .output()?;
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("pattern must contain {name}"));

    let mut too_many_backgrounds = Command::new(env!("CARGO_BIN_EXE_svg-icon-check"));
    too_many_backgrounds.arg(&first);
    for _ in 0..17 {
        too_many_backgrounds.args(["--background", "1,2,3"]);
    }
    let rejected_backgrounds = too_many_backgrounds.output()?;
    assert!(!rejected_backgrounds.status.success());
    assert!(
        String::from_utf8_lossy(&rejected_backgrounds.stderr)
            .contains("at most 16 background option occurrences are allowed")
    );
    Ok(())
}

#[test]
fn cli_reports_processing_errors() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let svg = dir.path().join("icon.svg");
    fs::write(
        &svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#,
    )?;
    let check_failure = |command: &mut Command, expected: &str| -> anyhow::Result<()> {
        let result = command.output()?;
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        Ok(())
    };

    check_failure(
        Command::new(env!("CARGO_BIN_EXE_svg-icon-check")).arg(dir.path().join("invalid.png")),
        "Unsupported file type",
    )?;
    check_failure(
        Command::new(env!("CARGO_BIN_EXE_svg-icon-check")).arg(dir.path().join("missing.svg")),
        "Cannot read SVG file",
    )?;

    let output = dir.path().join("not-a-directory");
    fs::write(&output, "")?;
    check_failure(
        Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
            .arg(&svg)
            .arg("--output")
            .arg(&output),
        "Cannot create output directory",
    )?;

    let invalid_manifest_svg = dir.path().join("invalid\tname.svg");
    fs::copy(&svg, &invalid_manifest_svg)?;
    check_failure(
        Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
            .arg(invalid_manifest_svg)
            .arg("--manifest")
            .arg("--output")
            .arg(dir.path().join("manifest-output")),
        "Cannot include SVG filename in manifest",
    )?;

    #[cfg(unix)]
    check_failure(
        Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
            .arg(&svg)
            .env("TMPDIR", dir.path().join("missing-temp")),
        "Cannot create temporary directory",
    )?;
    Ok(())
}
