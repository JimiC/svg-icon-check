use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Operation {
    Metadata,
    SymlinkMetadata,
    ReadLink,
    Copy,
    RemoveFile,
    DirectoryEntry,
    BackupDir,
    ManifestWrite,
    ManifestFlush,
}

struct FaultyFileOps<F>(F);

struct FaultyWriter<'a, F> {
    files: &'a FaultyFileOps<F>,
    file: &'a mut fs::File,
}

impl<F> Write for FaultyWriter<'_, F>
where
    F: Fn(Operation, &Path, &Path) -> io::Result<()>,
{
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        (self.files.0)(Operation::ManifestWrite, Path::new(""), Path::new(""))?;
        self.file.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        (self.files.0)(Operation::ManifestFlush, Path::new(""), Path::new(""))?;
        self.file.flush()
    }
}

impl<F> FileOperations for FaultyFileOps<F>
where
    F: Fn(Operation, &Path, &Path) -> io::Result<()>,
{
    fn metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        (self.0)(Operation::Metadata, path, path)?;
        RealFileOperations.metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        (self.0)(Operation::SymlinkMetadata, path, path)?;
        RealFileOperations.symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        (self.0)(Operation::ReadLink, path, path)?;
        RealFileOperations.read_link(path)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        (self.0)(Operation::Copy, from, to)?;
        RealFileOperations.copy(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        (self.0)(Operation::RemoveFile, path, path)?;
        RealFileOperations.remove_file(path)
    }

    fn entry_path(&self, entry: io::Result<fs::DirEntry>) -> io::Result<PathBuf> {
        let path = RealFileOperations.entry_path(entry)?;
        (self.0)(Operation::DirectoryEntry, &path, &path)?;
        Ok(path)
    }

    fn backup_dir(&self) -> io::Result<tempfile::TempDir> {
        (self.0)(Operation::BackupDir, Path::new(""), Path::new(""))?;
        RealFileOperations.backup_dir()
    }

    fn manifest_writer<'a>(&'a self, file: &'a mut tempfile::NamedTempFile) -> Box<dyn Write + 'a> {
        Box::new(FaultyWriter {
            files: self,
            file: file.as_file_mut(),
        })
    }
}

fn injected_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "injected filesystem failure",
    )
}

fn job(svg: PathBuf, png_name: &str) -> PreviewJob {
    PreviewJob {
        svg,
        png_name: png_name.to_owned(),
        theme_mode: None,
    }
}

#[test]
fn creates_manifest_and_rejects_bad_names_or_missing_svg() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let svg = dir.path().join("icon.svg");
    fs::write(&svg, "<svg/>")?;
    let output = dir.path().join("output");
    fs::create_dir(&output)?;

    create_manifest(&[job(svg.clone(), "icon.png")], &output)?;
    assert_eq!(
        fs::read_to_string(output.join("manifest.tsv"))?,
        "icon\t6\ticon.png\n"
    );

    assert!(create_manifest(&[job(svg.clone(), "bad\tname.png")], &output).is_err());
    let bad_stem = dir.path().join("bad\tname.svg");
    fs::write(&bad_stem, "")?;
    assert!(create_manifest(&[job(bad_stem, "bad.png")], &output).is_err());
    assert!(
        create_manifest(
            &[job(dir.path().join("missing.svg"), "missing.png")],
            &output
        )
        .is_err()
    );
    fs::remove_file(output.join("manifest.tsv"))?;
    let manifest_dir = output.join("manifest.tsv");
    fs::create_dir(&manifest_dir)?;
    assert!(create_manifest(&[job(svg, "icon.png")], &output).is_err());
    assert!(create_manifest(&[job(PathBuf::from("."), "icon.png")], dir.path()).is_err());
    assert!(create_manifest(&[], &dir.path().join("missing-directory")).is_err());
    Ok(())
}

#[test]
fn finalizes_output_and_rolls_back_copied_previews_on_error() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let temp = dir.path().join("temp");
    let output = dir.path().join("output");
    fs::create_dir(&temp)?;
    fs::create_dir(&output)?;
    fs::write(temp.join("one.png"), "new")?;
    fs::write(output.join("one.png"), "old")?;

    let finalized = finalize_output(&temp, output.clone())?;
    assert_eq!(finalized, output);
    assert_eq!(fs::read_to_string(output.join("one.png"))?, "new");

    fs::write(temp.join("one.png"), "newer")?;
    fs::write(temp.join("new.png"), "new file")?;
    fs::create_dir(output.join("conflict.png"))?;
    let entries = [
        temp.join("one.png"),
        temp.join("new.png"),
        temp.join("conflict.png"),
    ];
    let backup_dir = dir.path().join("backup");
    fs::create_dir(&backup_dir)?;
    let Err((error, rollback_failed)) =
        copy_previews(&RealFileOperations, &entries, &output, &backup_dir)
    else {
        anyhow::bail!("a directory destination must fail");
    };
    assert!(error.to_string().contains("destination is a directory"));
    assert!(!rollback_failed);
    assert_eq!(fs::read_to_string(output.join("one.png"))?, "new");
    assert!(!output.join("new.png").exists());
    let missing = temp.join("missing.png");
    assert!(copy_previews(&RealFileOperations, &[missing], &output, &backup_dir).is_err());
    Ok(())
}

#[test]
fn reports_directory_destinations_and_output_creation_errors() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let temp = dir.path().join("temp");
    let output = dir.path().join("output");
    fs::create_dir(&temp)?;
    fs::create_dir(&output)?;
    fs::write(temp.join("conflict.png"), "image")?;
    fs::create_dir(output.join("conflict.png"))?;
    let backup = dir.path().join("backup");
    fs::create_dir(&backup)?;

    assert!(
        copy_previews(
            &RealFileOperations,
            &[temp.join("conflict.png")],
            &output,
            &backup
        )
        .is_err()
    );
    assert!(finalize_output(&output, dir.path().join("output-file")).is_err());

    fs::write(temp.join("copy.png"), "image")?;
    fs::write(output.join("copy.png"), "old")?;
    let backup_failure = dir.path().join("backup-missing");
    assert!(
        copy_previews(
            &RealFileOperations,
            &[temp.join("copy.png")],
            &output,
            &backup_failure
        )
        .is_err()
    );

    let output_file = dir.path().join("not-a-directory");
    fs::write(&output_file, "")?;
    assert!(finalize_output(&temp, output_file.clone()).is_err());
    assert!(finalize_output(&output_file, dir.path().join("unused-output")).is_err());

    let empty_entry = PathBuf::new();
    assert!(
        copy_previews(
            &RealFileOperations,
            &[empty_entry],
            &output,
            &backup_failure
        )
        .is_err()
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn resolves_symlinks_and_detects_cycles() -> anyhow::Result<()> {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir()?;
    let real = dir.path().join("real.png");
    let link = dir.path().join("link.png");
    fs::write(&real, "")?;
    symlink("real.png", &link)?;
    assert_eq!(resolve_copy_target(&RealFileOperations, &link)?, real);

    let manifest_link = dir.path().join("manifest.tsv");
    symlink("/", &manifest_link)?;
    let error = create_manifest(&[], dir.path())
        .err()
        .context("manifest target with no parent must fail")?;
    assert!(error.to_string().contains("Manifest path has no parent"));
    fs::remove_file(&manifest_link)?;

    let first = dir.path().join("first");
    let second = dir.path().join("second");
    symlink("second", &first)?;
    symlink("first", &second)?;
    assert!(resolve_copy_target(&RealFileOperations, &first).is_err());
    symlink("first", &manifest_link)?;
    let error = create_manifest(&[], dir.path())
        .err()
        .context("cyclic manifest symlink must fail")?;
    assert!(error.to_string().contains("Too many symlinks"));

    let source = dir.path().join("device.png");
    fs::write(&source, "")?;
    let output = dir.path().join("output");
    fs::create_dir(&output)?;
    let output_device_link = output.join("device.png");
    symlink("/dev/null", output_device_link)?;
    let backup = dir.path().join("backup");
    fs::create_dir(&backup)?;
    assert!(copy_previews(&RealFileOperations, &[source], &output, &backup).is_err());

    let file_parent = dir.path().join("not-a-directory");
    fs::write(&file_parent, "")?;
    assert!(resolve_copy_target(&RealFileOperations, &file_parent.join("child")).is_err());
    let output_source = dir.path().join("source.png");
    fs::write(&output_source, "image")?;
    assert!(copy_previews(&RealFileOperations, &[output_source], &file_parent, &backup).is_err());
    Ok(())
}

#[test]
fn reports_metadata_and_rollback_removal_failures() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let output = dir.path().join("output");
    let backup = dir.path().join("backup");
    fs::create_dir(&output)?;
    fs::create_dir(&backup)?;
    let source = dir.path().join("new.png");
    fs::write(&source, "image")?;

    let fail_metadata = FaultyFileOps(|operation, _: &Path, _: &Path| {
        if operation == Operation::Metadata {
            return Err(injected_error());
        }
        Ok(())
    });
    let Err((error, rollback_failed)) = copy_previews(
        &fail_metadata,
        std::slice::from_ref(&source),
        &output,
        &backup,
    ) else {
        anyhow::bail!("metadata failure was not reported");
    };
    assert!(error.to_string().contains("Cannot inspect output file"));
    assert!(format!("{error:#}").contains("injected filesystem failure"));
    assert!(!rollback_failed);

    let fail_remove = FaultyFileOps(|operation, path: &Path, _: &Path| {
        if operation == Operation::RemoveFile && path == output.join("new.png") {
            return Err(injected_error());
        }
        Ok(())
    });
    let Err((error, rollback_failed)) = copy_previews(
        &fail_remove,
        &[source, dir.path().join("missing.png")],
        &output,
        &backup,
    ) else {
        anyhow::bail!("rollback failure was not reported");
    };
    assert!(rollback_failed);
    assert!(error.to_string().contains("Rollback errors:"));
    assert!(error.to_string().contains("injected filesystem failure"));
    assert!(output.join("new.png").is_file());
    Ok(())
}

#[cfg(unix)]
#[test]
fn reports_symlink_read_failure() -> anyhow::Result<()> {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir()?;
    let link = dir.path().join("link");
    symlink("target", &link)?;
    let files = FaultyFileOps(|operation, _: &Path, _: &Path| {
        if operation == Operation::ReadLink {
            return Err(injected_error());
        }
        Ok(())
    });
    let Err(error) = resolve_copy_target(&files, &link) else {
        anyhow::bail!("read_link must fail");
    };
    assert!(error.to_string().contains("Cannot read symlink"));
    assert!(format!("{error:#}").contains("injected filesystem failure"));
    Ok(())
}

#[test]
fn retains_backup_when_restore_fails() -> anyhow::Result<()> {
    use std::cell::RefCell;

    let dir = tempfile::tempdir()?;
    let temp = dir.path().join("temp");
    let output = dir.path().join("output");
    fs::create_dir(&temp)?;
    fs::create_dir(&output)?;
    let source = temp.join("icon.png");
    let target = output.join("icon.png");
    fs::write(&source, "new")?;
    fs::write(&target, "old")?;

    let backup_path = RefCell::new(None);
    let files = FaultyFileOps(|operation, from: &Path, to: &Path| {
        if operation == Operation::Copy && to == target {
            if from != source {
                backup_path.replace(Some(from.to_path_buf()));
            }
            return Err(injected_error());
        }
        Ok(())
    });

    let result = finalize_output_with(&files, &temp, output);
    let backup_path = backup_path
        .into_inner()
        .context("rollback did not attempt to restore a backup")?;
    let backup_dir = backup_path.parent().context("backup path has no parent")?;
    let contents = fs::read_to_string(&backup_path);
    let cleanup = fs::remove_dir_all(backup_dir);
    let Err(error) = result else {
        anyhow::bail!("rollback failure must be reported");
    };
    assert!(
        error
            .to_string()
            .contains("Rollback failed; backups retained")
    );
    assert!(
        error
            .to_string()
            .contains(&backup_dir.display().to_string())
    );
    assert!(format!("{error:#}").contains("injected filesystem failure"));
    assert_eq!(contents?, "old");
    cleanup?;
    Ok(())
}

#[test]
fn reports_directory_entry_and_backup_creation_errors() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let temp = dir.path().join("temp");
    fs::create_dir(&temp)?;
    fs::write(temp.join("icon.png"), "image")?;
    let output = dir.path().join("output");

    for (operation, expected) in [
        (
            Operation::DirectoryEntry,
            "Cannot read temp directory entry",
        ),
        (Operation::BackupDir, "Cannot create backup directory"),
    ] {
        let files = FaultyFileOps(|actual, _: &Path, _: &Path| {
            if actual == operation {
                return Err(injected_error());
            }
            Ok(())
        });
        let error = finalize_output_with(&files, &temp, output.clone())
            .err()
            .context("injected failure must be reported")?;
        assert!(error.to_string().contains(expected));
        assert!(format!("{error:#}").contains("injected filesystem failure"));
    }
    assert!(!output.join("icon.png").exists());
    Ok(())
}

#[test]
fn leaves_existing_manifest_untouched_on_write_or_flush_failure() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let svg = dir.path().join("icon.svg");
    fs::write(&svg, "<svg/>")?;
    let manifest = dir.path().join("manifest.tsv");
    fs::write(&manifest, "existing manifest")?;

    for (operation, name) in [
        (Operation::ManifestWrite, "x".repeat(9000)),
        (Operation::ManifestFlush, "icon.png".to_owned()),
    ] {
        let files = FaultyFileOps(|actual, _: &Path, _: &Path| {
            if actual == operation {
                return Err(injected_error());
            }
            Ok(())
        });
        let error = create_manifest_with(&files, &[job(svg.clone(), &name)], dir.path())
            .err()
            .context("manifest failure must be reported")?;
        assert!(error.to_string().contains("Cannot write manifest file"));
        assert!(format!("{error:#}").contains("injected filesystem failure"));
        assert_eq!(fs::read_to_string(&manifest)?, "existing manifest");
    }
    Ok(())
}
