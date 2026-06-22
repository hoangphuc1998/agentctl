use std::{collections::BTreeSet, error::Error, path::Path, process::Command};

pub type BranchResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub trait BranchLister {
    fn list_branches(&mut self, repo_path: &Path) -> BranchResult<Vec<String>>;
}

#[derive(Default)]
pub struct GitBranchLister;

impl BranchLister for GitBranchLister {
    fn list_branches(&mut self, repo_path: &Path) -> BranchResult<Vec<String>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .args(["branch", "--list", "--all", "--format=%(refname:short)"])
            .output()?;
        if !output.status.success() {
            return Err(format!("failed to list branches in {}", repo_path.display()).into());
        }
        Ok(parse_branch_output(&String::from_utf8_lossy(
            &output.stdout,
        )))
    }
}

pub fn parse_branch_output(output: &str) -> Vec<String> {
    let mut branches = BTreeSet::new();
    for raw in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if raw.contains("HEAD ->") {
            continue;
        }
        let branch = raw.strip_prefix("remotes/").unwrap_or(raw).to_string();
        branches.insert(branch);
    }
    branches.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::parse_branch_output;

    #[test]
    fn parses_local_and_remote_branch_names() {
        let output = "main\nfeature/login\nremotes/origin/main\nremotes/origin/feature/pay\n";

        let branches = parse_branch_output(output);

        assert_eq!(
            branches,
            vec![
                "feature/login".to_string(),
                "main".to_string(),
                "origin/feature/pay".to_string(),
                "origin/main".to_string(),
            ]
        );
    }

    #[test]
    fn removes_empty_duplicate_and_head_rows() {
        let output = "\nmain\nmain\nremotes/origin/HEAD -> origin/main\n";

        let branches = parse_branch_output(output);

        assert_eq!(branches, vec!["main".to_string()]);
    }
}
