use std::{
    collections::{BTreeSet, HashSet},
    env, fs,
    path::{Path, PathBuf, MAIN_SEPARATOR},
};

const MAX_CANDIDATES: usize = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionField {
    Repo,
    Base,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionCandidate {
    pub value: String,
    pub detail: String,
}

impl CompletionCandidate {
    fn new(value: String, detail: impl Into<String>) -> Self {
        Self {
            value,
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionState {
    field: Option<CompletionField>,
    selected: usize,
    candidates: Vec<CompletionCandidate>,
}

impl CompletionState {
    pub fn new(candidates: Vec<CompletionCandidate>) -> Option<Self> {
        Self::with_field(None, candidates)
    }

    pub fn for_field(field: CompletionField, candidates: Vec<CompletionCandidate>) -> Option<Self> {
        Self::with_field(Some(field), candidates)
    }

    pub fn field(&self) -> Option<CompletionField> {
        self.field
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn candidates(&self) -> &[CompletionCandidate] {
        &self.candidates
    }

    pub fn selected_value(&self) -> Option<&str> {
        self.candidates
            .get(self.selected)
            .map(|candidate| candidate.value.as_str())
    }

    pub fn next(&mut self) {
        if !self.candidates.is_empty() {
            self.selected = (self.selected + 1) % self.candidates.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.candidates.is_empty() {
            self.selected = (self.selected + self.candidates.len() - 1) % self.candidates.len();
        }
    }

    fn with_field(
        field: Option<CompletionField>,
        candidates: Vec<CompletionCandidate>,
    ) -> Option<Self> {
        if candidates.is_empty() {
            None
        } else {
            Some(Self {
                field,
                selected: 0,
                candidates,
            })
        }
    }
}

pub fn repo_path_candidates(
    input: &str,
    recent_repo_paths: &[PathBuf],
) -> Vec<CompletionCandidate> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for path in matching_child_directories(input) {
        push_unique_path_candidate(&mut candidates, &mut seen, path, "directory");
    }

    for path in recent_repo_paths {
        if repo_path_matches_input(path, input) {
            push_unique_path_candidate(&mut candidates, &mut seen, path.clone(), "recent repo");
        }
    }

    candidates.truncate(MAX_CANDIDATES);
    candidates
}

pub fn base_ref_candidates(input: &str, branches: &[String]) -> Vec<CompletionCandidate> {
    let mut values = BTreeSet::new();
    values.insert("HEAD".to_string());
    values.extend(branches.iter().cloned());

    values
        .into_iter()
        .filter(|value| input.is_empty() || value.starts_with(input))
        .take(MAX_CANDIDATES)
        .map(|value| CompletionCandidate::new(value, "base ref"))
        .collect()
}

fn matching_child_directories(input: &str) -> Vec<PathBuf> {
    let expanded = expand_home(input);
    let path = PathBuf::from(&expanded);
    let (directory, prefix) = directory_and_prefix(&path, input.ends_with(MAIN_SEPARATOR));

    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut matches = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
        })
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    matches
}

fn directory_and_prefix(path: &Path, ends_with_separator: bool) -> (PathBuf, String) {
    if ends_with_separator {
        return (path.to_path_buf(), String::new());
    }

    let directory = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let prefix = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    (directory, prefix)
}

fn repo_path_matches_input(path: &Path, input: &str) -> bool {
    if input.is_empty() {
        return true;
    }

    let expanded = expand_home(input);
    let path_value = path.to_string_lossy();
    path_value.starts_with(input)
        || path_value.starts_with(&expanded)
        || path
            .file_name()
            .map(|name| name.to_string_lossy().starts_with(input))
            .unwrap_or(false)
}

fn push_unique_path_candidate(
    candidates: &mut Vec<CompletionCandidate>,
    seen: &mut HashSet<String>,
    path: PathBuf,
    detail: &'static str,
) {
    let value = path_candidate_value(&path);
    let key = path_key(&value);
    if seen.insert(key) {
        candidates.push(CompletionCandidate::new(value, detail));
    }
}

fn path_candidate_value(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut value = path.to_string_lossy().to_string();
    if path.is_dir() && !value.ends_with(MAIN_SEPARATOR) {
        value.push(MAIN_SEPARATOR);
    }
    value
}

fn path_key(value: &str) -> String {
    value.trim_end_matches(MAIN_SEPARATOR).to_string()
}

fn expand_home(input: &str) -> String {
    if input == "~" {
        return home_dir()
            .map(|home| home.to_string_lossy().to_string())
            .unwrap_or_else(|| input.to_string());
    }

    if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest).to_string_lossy().to_string();
        }
    }

    input.to_string()
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}
