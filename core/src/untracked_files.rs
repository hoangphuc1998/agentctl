use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

pub const LARGE_UNTRACKED_COPY_BYTES: u64 = 100 * 1024 * 1024;
pub const LARGE_UNTRACKED_COPY_FILE_COUNT: usize = 10_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UntrackedFilesPreview {
    pub file_count: usize,
    pub total_bytes: u64,
}

impl UntrackedFilesPreview {
    pub fn requires_confirmation(&self) -> bool {
        self.file_count >= LARGE_UNTRACKED_COPY_FILE_COUNT
            || self.total_bytes >= LARGE_UNTRACKED_COPY_BYTES
    }
}

pub fn preview_untracked_files(
    source_root: &Path,
    untracked_files: &str,
) -> io::Result<UntrackedFilesPreview> {
    let mut preview = UntrackedFilesPreview {
        file_count: 0,
        total_bytes: 0,
    };

    for relative_path in parse_untracked_paths(untracked_files)? {
        let metadata = fs::symlink_metadata(source_root.join(relative_path))?;
        if !metadata.is_file() {
            continue;
        }
        preview.file_count += 1;
        preview.total_bytes = preview
            .total_bytes
            .checked_add(metadata.len())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "file sizes overflow u64"))?;
    }

    Ok(preview)
}

pub fn copy_untracked_files(
    source_root: &Path,
    worktree_root: &Path,
    untracked_files: &str,
) -> io::Result<()> {
    for relative_path in parse_untracked_paths(untracked_files)? {
        let source_path = source_root.join(&relative_path);
        if !fs::symlink_metadata(&source_path)?.is_file() {
            continue;
        }

        let target_path = worktree_root.join(&relative_path);
        if target_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("refusing to overwrite {}", target_path.display()),
            ));
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source_path, target_path)?;
    }
    Ok(())
}

pub fn delete_untracked_files(worktree_root: &Path, untracked_files: &str) -> io::Result<()> {
    for relative_path in parse_untracked_paths(untracked_files)? {
        let target_path = worktree_root.join(&relative_path);
        match fs::symlink_metadata(&target_path) {
            Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => {
                fs::remove_file(&target_path)?;
                remove_empty_parent_dirs(worktree_root, target_path.parent())?;
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn parse_untracked_paths(untracked_files: &str) -> io::Result<Vec<PathBuf>> {
    untracked_files
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(safe_relative_path)
        .collect()
}

fn safe_relative_path(path: &str) -> io::Result<PathBuf> {
    let mut relative_path = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(segment) => relative_path.push(segment),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unsafe untracked file path: {path}"),
                ));
            }
        }
    }

    if relative_path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "empty untracked file path",
        ));
    }

    Ok(relative_path)
}

fn remove_empty_parent_dirs(root: &Path, start: Option<&Path>) -> io::Result<()> {
    let Some(mut current) = start else {
        return Ok(());
    };

    while current != root && current.starts_with(root) {
        match fs::remove_dir(current) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) if err.kind() == io::ErrorKind::DirectoryNotEmpty => break,
            Err(err) => return Err(err),
        }

        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        copy_untracked_files, delete_untracked_files, preview_untracked_files,
        UntrackedFilesPreview, LARGE_UNTRACKED_COPY_BYTES, LARGE_UNTRACKED_COPY_FILE_COUNT,
    };

    #[test]
    fn copy_untracked_files_preserves_relative_paths() {
        let source_root = tempfile::tempdir().expect("source root");
        let worktree_root = tempfile::tempdir().expect("worktree root");
        let source_file = source_root.path().join("notes").join("scratch.txt");
        std::fs::create_dir_all(source_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&source_file, "draft").expect("write source");

        copy_untracked_files(
            source_root.path(),
            worktree_root.path(),
            "notes/scratch.txt\0",
        )
        .expect("copied files");

        assert_eq!(
            std::fs::read_to_string(worktree_root.path().join("notes").join("scratch.txt"))
                .expect("read copied file"),
            "draft"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_untracked_files_skips_symlinks() {
        let source_root = tempfile::tempdir().expect("source root");
        let worktree_root = tempfile::tempdir().expect("worktree root");
        let source_file = source_root.path().join("target.txt");
        let source_link = source_root.path().join("linked.txt");
        std::fs::write(&source_file, "target").expect("write source");
        std::os::unix::fs::symlink(&source_file, &source_link).expect("symlink");

        copy_untracked_files(source_root.path(), worktree_root.path(), "linked.txt\0")
            .expect("copied files");

        assert!(!worktree_root.path().join("linked.txt").exists());
    }

    #[test]
    fn delete_untracked_files_removes_empty_parent_directories() {
        let worktree_root = tempfile::tempdir().expect("worktree root");
        let copied_file = worktree_root.path().join("notes").join("scratch.txt");
        std::fs::create_dir_all(copied_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&copied_file, "draft").expect("write copied");

        delete_untracked_files(worktree_root.path(), "notes/scratch.txt\0").expect("deleted files");

        assert!(!copied_file.exists());
        assert!(!worktree_root.path().join("notes").exists());
    }

    #[test]
    fn preview_untracked_files_counts_regular_files_and_bytes() {
        let source_root = tempfile::tempdir().expect("source root");
        let env_file = source_root.path().join(".env");
        let generated_file = source_root.path().join("generated").join("client.js");
        std::fs::create_dir_all(generated_file.parent().expect("parent")).expect("mkdir");
        std::fs::write(&env_file, "TOKEN=secret").expect("write env");
        std::fs::write(&generated_file, "client").expect("write generated");

        let preview = preview_untracked_files(source_root.path(), ".env\0generated/client.js\0")
            .expect("preview");

        assert_eq!(
            preview,
            UntrackedFilesPreview {
                file_count: 2,
                total_bytes: 18,
            }
        );
        assert!(!preview.requires_confirmation());
    }

    #[test]
    fn preview_requires_confirmation_at_either_large_copy_threshold() {
        assert!(UntrackedFilesPreview {
            file_count: LARGE_UNTRACKED_COPY_FILE_COUNT,
            total_bytes: 1,
        }
        .requires_confirmation());
        assert!(UntrackedFilesPreview {
            file_count: 1,
            total_bytes: LARGE_UNTRACKED_COPY_BYTES,
        }
        .requires_confirmation());
        assert!(!UntrackedFilesPreview {
            file_count: LARGE_UNTRACKED_COPY_FILE_COUNT - 1,
            total_bytes: LARGE_UNTRACKED_COPY_BYTES - 1,
        }
        .requires_confirmation());
    }

    #[test]
    fn preview_untracked_files_rejects_unsafe_paths() {
        let source_root = tempfile::tempdir().expect("source root");

        let err = preview_untracked_files(source_root.path(), "../outside\0")
            .expect_err("unsafe path must fail");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
