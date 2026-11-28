use super::*;

#[test]
fn extracts_path_parts_and_reports_missing_parts() {
    let path = Path::new("folder/Icon.SVG");
    assert!(get_filestem(path).is_ok_and(|value| value == "Icon"));
    assert!(get_filename(path).is_ok_and(|value| value == "Icon.SVG"));
    assert!(get_file_extension(path).is_ok_and(|value| value == "SVG"));
    assert!(get_filestem(Path::new(".")).is_err());
    assert!(get_filename(Path::new(".")).is_err());
    assert!(get_file_extension(Path::new("icon")).is_err());
}

#[test]
fn normalizes_paths_and_matches_svg_extensions_without_case_sensitivity() {
    let path = Path::new("./folder/../icon.svg");
    assert_eq!(
        normalize_display_path(path),
        PathBuf::from("./folder/../icon.svg")
    );
    assert!(is_svg_extension("svg"));
    assert!(is_svg_extension("SVG"));
    assert!(is_svg_extension("SvgZ"));
    assert!(!is_svg_extension("png"));
}

#[test]
fn calculates_relative_luminance_and_background_classification() {
    let black = ColorU8::from_rgba(0, 0, 0, 255);
    let white = ColorU8::from_rgba(255, 255, 255, 255);
    let below_threshold = ColorU8::from_rgba(117, 117, 117, 255);

    assert_eq!(relative_luminance(black), 0.0);
    assert_eq!(relative_luminance(white), 1.0);
    assert!(!is_light_background(black));
    assert!(is_light_background(white));
    assert!(!is_light_background(below_threshold));
    assert!(relative_luminance(below_threshold) > 0.17);
}
