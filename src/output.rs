use std::{
    env, fs, io,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{
    jobs::PreviewJob,
    utils::{get_filename, get_filestem, normalize_display_path},
};

trait FileOperations {
    fn metadata(&self, path: &Path) -> io::Result<fs::Metadata>;
    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata>;
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;
    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    fn entry_path(&self, entry: io::Result<fs::DirEntry>) -> io::Result<PathBuf>;
    fn backup_dir(&self) -> io::Result<tempfile::TempDir>;
    fn manifest_writer<'a>(&'a self, file: &'a mut tempfile::NamedTempFile) -> Box<dyn Write + 'a>;
}

struct RealFileOperations;

impl FileOperations for RealFileOperations {
    fn metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        fs::read_link(path)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        fs::copy(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn entry_path(&self, entry: io::Result<fs::DirEntry>) -> io::Result<PathBuf> {
        entry.map(|entry| entry.path())
    }

    fn backup_dir(&self) -> io::Result<tempfile::TempDir> {
        tempfile::tempdir()
    }

    fn manifest_writer<'a>(&'a self, file: &'a mut tempfile::NamedTempFile) -> Box<dyn Write + 'a> {
        Box::new(file.as_file_mut())
    }
}

pub(crate) fn create_manifest(preview_jobs: &[PreviewJob], output_dir: &Path) -> Result<()> {
    create_manifest_with(&RealFileOperations, preview_jobs, output_dir)
}

fn create_manifest_with(
    files: &dyn FileOperations,
    preview_jobs: &[PreviewJob],
    output_dir: &Path,
) -> Result<()> {
    let target = resolve_copy_target(files, &output_dir.join("manifest.tsv"))?;
    let parent = target.parent().context("Manifest path has no parent")?;
    let mut file =
        tempfile::NamedTempFile::new_in(parent).context("Cannot create temporary manifest file")?;
    let mut writer = BufWriter::new(files.manifest_writer(&mut file));

    for job in preview_jobs {
        let stem = get_filestem(&job.svg)?;
        if stem.contains(['\t', '\n', '\r']) {
            bail!(
                "Cannot include SVG filename in manifest: {}",
                normalize_display_path(&job.svg).display()
            );
        }
        if job.png_name.contains(['\t', '\n', '\r']) {
            bail!(
                "Cannot include preview filename in manifest: {}",
                job.png_name
            );
        }
        let size = fs::metadata(&job.svg)
            .with_context(|| {
                format!(
                    "Cannot read SVG file metadata: {}",
                    normalize_display_path(&job.svg).display()
                )
            })?
            .len();
        writeln!(writer, "{stem}\t{size}\t{}", job.png_name)
            .context("Cannot write manifest file")?;
    }

    writer.flush().context("Cannot write manifest file")?;
    drop(writer);
    file.persist(target)
        .context("Cannot publish manifest file")?;
    Ok(())
}

pub(crate) fn finalize_output(temp_dir: &Path, output: PathBuf) -> Result<PathBuf> {
    finalize_output_with(&RealFileOperations, temp_dir, output)
}

fn finalize_output_with(
    files: &dyn FileOperations,
    temp_dir: &Path,
    output: PathBuf,
) -> Result<PathBuf> {
    let cwd = env::current_dir()?;
    let output_abs = match fs::canonicalize(&output) {
        Ok(path) => path,
        Err(_) => cwd.join(output),
    };

    fs::create_dir_all(&output_abs).with_context(|| {
        format!(
            "Cannot create output directory: {}",
            normalize_display_path(&output_abs).display()
        )
    })?;

    let entries = fs::read_dir(temp_dir)
        .context("Cannot read temp directory")?
        .map(|entry| files.entry_path(entry))
        .collect::<io::Result<Vec<_>>>()
        .context("Cannot read temp directory entry")?;
    let backups = files
        .backup_dir()
        .context("Cannot create backup directory")?;
    if let Err((error, rollback_failed)) =
        copy_previews(files, &entries, &output_abs, backups.path())
    {
        if rollback_failed {
            let backup_path = backups.keep();
            return Err(error.context(format!(
                "Rollback failed; backups retained in {}",
                normalize_display_path(&backup_path).display()
            )));
        }
        return Err(error);
    }

    Ok(output_abs)
}

fn copy_previews(
    files: &dyn FileOperations,
    entries: &[PathBuf],
    output: &Path,
    backup_dir: &Path,
) -> Result<(), (anyhow::Error, bool)> {
    let mut copied: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let result = (|| -> Result<()> {
        for entry in entries {
            let filename = get_filename(entry)?;
            let target = output.join(&filename);
            let resolved = resolve_copy_target(files, &target)?;
            let backup = match files.metadata(&resolved) {
                Ok(metadata) if metadata.is_file() => {
                    let backup = backup_dir.join(copied.len().to_string());
                    files.copy(&resolved, &backup).with_context(|| {
                        format!(
                            "Cannot backup output file: {}",
                            normalize_display_path(&target).display()
                        )
                    })?;
                    Some(backup)
                }
                Ok(metadata) if metadata.is_dir() => {
                    bail!("Cannot copy file: {filename}: destination is a directory")
                }
                Ok(_) => bail!("Cannot copy file: {filename}: destination is not a regular file"),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "Cannot inspect output file: {}",
                            normalize_display_path(&target).display()
                        )
                    });
                }
            };

            copied.push((resolved, backup));
            files
                .copy(entry, &target)
                .with_context(|| format!("Cannot copy file: {filename}"))?;
        }
        Ok(())
    })();

    if let Err(error) = result {
        let mut rollback_errors = Vec::new();
        for (target, backup) in copied.into_iter().rev() {
            let result = match &backup {
                Some(backup) => files.copy(backup, &target).map(|_| ()),
                None => match files.remove_file(&target) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                    result => result,
                },
            };
            if let Err(rollback_error) = result {
                let backup_info = backup.map_or_else(String::new, |path| {
                    format!(" (backup: {})", normalize_display_path(&path).display())
                });
                rollback_errors.push(format!(
                    "{}: {rollback_error}{backup_info}",
                    normalize_display_path(&target).display()
                ));
            }
        }
        if rollback_errors.is_empty() {
            return Err((error, false));
        }
        return Err((
            error.context(format!("Rollback errors: {}", rollback_errors.join("; "))),
            true,
        ));
    }
    Ok(())
}

fn resolve_copy_target(files: &dyn FileOperations, target: &Path) -> Result<PathBuf> {
    let mut resolved = target.to_path_buf();
    for _ in 0..40 {
        match files.symlink_metadata(&resolved) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let link = files.read_link(&resolved).with_context(|| {
                    format!(
                        "Cannot read symlink: {}",
                        normalize_display_path(&resolved).display()
                    )
                })?;
                resolved = resolved.parent().unwrap_or(Path::new(".")).join(link);
            }
            Ok(_) => return Ok(resolved),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(resolved),
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "Cannot inspect output file: {}",
                        normalize_display_path(&resolved).display()
                    )
                });
            }
        }
    }
    bail!(
        "Too many symlinks resolving output file: {}",
        normalize_display_path(target).display()
    )
}
