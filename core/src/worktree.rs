use std::path::{Path, PathBuf};

pub fn default_sibling_worktree_path(repo_path: &Path, run_slug: &str) -> PathBuf {
    let repo_name = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    let parent = repo_path.parent().unwrap_or_else(|| Path::new("."));
    parent
        .join(format!("{repo_name}-worktrees"))
        .join(sanitize_slug(run_slug))
}

pub fn sanitize_slug(value: &str) -> String {
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
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "agent-run".to_string()
    } else {
        slug
    }
}

pub fn default_branch_name(run_name: &str) -> String {
    sanitize_slug(run_name)
}
