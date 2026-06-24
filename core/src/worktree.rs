use std::path::{Path, PathBuf};

pub fn default_sibling_worktree_path(repo_path: &Path, run_slug: &str) -> PathBuf {
    let repo_name = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    let parent = repo_path.parent().unwrap_or_else(|| Path::new("."));
    let mut path = parent.join(format!("{repo_name}-worktrees"));
    for segment in path_slug(run_slug).split('/') {
        path = path.join(segment);
    }
    path
}

pub fn sanitize_slug(value: &str) -> String {
    let slug = sanitize_slug_segment(value);
    if slug.is_empty() {
        "agent-run".to_string()
    } else {
        slug
    }
}

fn sanitize_slug_segment(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.trim().chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if matches!(ch, '-' | '_' | '.' | ' ') {
            Some('-')
        } else {
            None
        };

        if let Some(ch) = normalized {
            if ch == '-' {
                if !last_dash && !slug.is_empty() {
                    slug.push(ch);
                }
                last_dash = true;
            } else {
                slug.push(ch);
                last_dash = false;
            }
        }
    }
    slug.trim_matches('-').to_string()
}

fn path_slug(value: &str) -> String {
    let segments: Vec<String> = value
        .split('/')
        .map(sanitize_slug_segment)
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        "agent-run".to_string()
    } else {
        segments.join("/")
    }
}

pub fn default_branch_name(run_name: &str) -> String {
    path_slug(run_name)
}

#[cfg(test)]
mod tests {
    use super::{default_branch_name, default_sibling_worktree_path};
    use std::path::{Path, PathBuf};

    #[test]
    fn branch_name_preserves_slash_hierarchy() {
        assert_eq!(default_branch_name("feature/login"), "feature/login");
    }

    #[test]
    fn sibling_worktree_path_preserves_slash_hierarchy() {
        let path =
            default_sibling_worktree_path(Path::new("/repos/agent-manager"), "feature/login");

        assert_eq!(
            path,
            PathBuf::from("/repos/agent-manager-worktrees/feature/login")
        );
    }
}
