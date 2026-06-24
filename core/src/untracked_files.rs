use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

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
    use super::{copy_untracked_files, delete_untracked_files};

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
}
