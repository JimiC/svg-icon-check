use std::{
    hint::black_box,
    io,
    process::{Command, Output},
};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tempfile::tempdir;

fn run_svg_icon_check(svg_path: &str, width: u32) -> io::Result<Output> {
    let temp_dir = tempdir()?;
    Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
        .arg(black_box(svg_path))
        .arg("-o")
        .arg(temp_dir.path())
        .arg("-w")
        .arg(width.to_string())
        .output()
}

fn benchmark_single_icon(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_icon");

    for width in &[100, 200, 400] {
        group.bench_with_input(BenchmarkId::from_parameter(width), width, |b, &width| {
            b.iter(|| run_svg_icon_check("fixtures/icons/default_file.svg", width));
        });
    }
    group.finish();
}

fn benchmark_multiple_icons(c: &mut Criterion) {
    c.bench_function("multiple_icons_width_200", |b| {
        b.iter(|| {
            let temp_dir = tempdir()?;
            Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
                .arg(black_box("fixtures/icons/default_file.svg"))
                .arg(black_box("fixtures/icons/default_folder.svg"))
                .arg(black_box("fixtures/icons/file_type_access.svg"))
                .arg("-o")
                .arg(temp_dir.path())
                .arg("-w")
                .arg("200")
                .output()
        });
    });
}

fn benchmark_with_backgrounds(c: &mut Criterion) {
    c.bench_function("icon_with_custom_backgrounds", |b| {
        b.iter(|| {
            let temp_dir = tempdir()?;
            Command::new(env!("CARGO_BIN_EXE_svg-icon-check"))
                .arg(black_box("fixtures/icons/default_file.svg"))
                .arg("-o")
                .arg(temp_dir.path())
                .arg("-w")
                .arg("200")
                .arg("-b")
                .arg(black_box("255,0,0"))
                .arg("-b")
                .arg(black_box("0,255,0"))
                .arg("-b")
                .arg(black_box("0,0,255"))
                .output()
        });
    });
}

criterion_group!(
    benches,
    benchmark_single_icon,
    benchmark_multiple_icons,
    benchmark_with_backgrounds
);
criterion_main!(benches);
