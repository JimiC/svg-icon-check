use super::*;

#[test]
fn parses_and_validates_name_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::try_parse_from(["svg-icon-check", "icon.svg"])?;
    assert_eq!(cli.name_pattern, "{name}");
    assert!(cli.validate_name_pattern().is_ok());

    for pattern in ["", "folder/{name}", "folder\\{name}", "bad\0{name}", "icon"] {
        let cli = Cli::try_parse_from(["svg-icon-check", "icon.svg", "--name-pattern", pattern])?;
        assert!(cli.validate_name_pattern().is_err(), "{pattern:?}");
    }

    let cli = Cli::try_parse_from([
        "svg-icon-check",
        "icon.svg",
        "--theme-mode",
        "--name-pattern",
        "{name}",
    ])?;
    assert!(cli.validate_name_pattern().is_err());
    let cli = Cli::try_parse_from([
        "svg-icon-check",
        "icon.svg",
        "--theme-mode",
        "--name-pattern",
        "{name}-{mode}",
    ])?;
    assert!(cli.validate_name_pattern().is_ok());
    Ok(())
}

#[test]
fn parses_icon_width_bounds_and_invalid_values() {
    assert_eq!(parse_icon_width("1"), Ok(1));
    assert_eq!(parse_icon_width("256"), Ok(256));
    for value in ["0", "257", "-1", "1.5", ""] {
        assert!(parse_icon_width(value).is_err(), "{value:?}");
    }
}

#[test]
fn parses_rgb_components_and_rejects_malformed_values() -> Result<(), String> {
    let color = parse_rgb(" 0, 127,255")?;
    assert_eq!(
        (color.red(), color.green(), color.blue(), color.alpha()),
        (0, 127, 255, 255)
    );

    for value in [
        "", "1,2", "1,2,3,4", "-1,0,0", "256,0,0", "x,0,0", "1,,3", "1,2,x",
    ] {
        assert!(parse_rgb(value).is_err(), "{value:?}");
    }
    Ok(())
}

#[test]
fn resolves_default_sorted_deduplicated_and_white_last_colors() -> anyhow::Result<()> {
    let red = ColorU8::from_rgba(255, 0, 0, 255);
    let blue = ColorU8::from_rgba(0, 0, 255, 255);
    let white = ColorU8::from_rgba(255, 255, 255, 255);
    let black = ColorU8::from_rgba(0, 0, 0, 255);

    assert_eq!(
        resolve_background_colors(&[], false, false)?,
        [black, white]
    );
    let colors = resolve_background_colors(&[red, blue, red, white, black], true, true)?;
    assert_eq!(colors.first(), Some(&black));
    assert_eq!(colors.get(1), Some(&blue));
    assert_eq!(colors.last(), Some(&white));
    assert_eq!(colors.iter().filter(|color| **color == red).count(), 1);
    assert_eq!(colors.iter().filter(|color| **color == black).count(), 1);
    assert_eq!(colors.iter().filter(|color| **color == white).count(), 1);
    Ok(())
}

#[test]
fn rejects_more_than_sixteen_backgrounds() {
    let colors = vec![ColorU8::from_rgba(1, 2, 3, 255); MAX_BACKGROUNDS + 1];
    assert!(resolve_background_colors(&colors, false, false).is_err());
}
