# Slash Branch Worktree Design

## Goal

When a user creates a run whose name contains `/`, Agent Manager should preserve that hierarchy in both the Git branch name and the default worktree folder path.

For example, a run name of `feature/login` should create:

- Branch: `feature/login`
- Worktree path: `<repo-parent>/<repo-name>-worktrees/feature/login`

## Scope

This feature changes the backend naming policy used by create-run. It does not change the New Run form, the registry schema, tmux window naming, cleanup behavior, or merge behavior.

## Naming Policy

Run names are interpreted as slash-separated path segments. Each segment is sanitized with the existing slug rules for characters inside that segment:

- ASCII letters and numbers are lowercased and preserved.
- `-`, `_`, `.`, and spaces normalize to `-`.
- Unsupported characters are removed.
- Empty segments are ignored.

The sanitized non-empty segments are joined with `/` for the branch name and as path components under the sibling worktree root.

If no valid segment remains, the fallback remains `agent-run`.

## Component Shape

The pure naming rules stay in `core/src/worktree.rs`.

`create_run_with_registry` should compute a path-aware run slug once, then use it for:

- `branch`
- `default_sibling_worktree_path`

Tmux window names should keep using the existing flattened-safe naming behavior so window identifiers remain simple.

## Data Flow

1. The frontend sends the user-entered run name unchanged through the existing `create_run` command.
2. `create_run_with_registry` converts the run name into a slash-aware run slug.
3. Git receives that slug as the branch name.
4. The worktree path helper joins each slug segment under `<repo-name>-worktrees`.
5. The registry stores the resulting branch and worktree path.

## Error Handling

This feature does not add new create-run errors. Segment sanitization prevents empty path components from accidental repeated slashes or invalid characters. Git remains responsible for rejecting branch names that violate Git's own reference rules beyond this app's normalization.

## Testing

Rust tests should cover:

- `default_branch_name("feature/login")` returns `feature/login`.
- `default_sibling_worktree_path(repo, "feature/login")` returns a nested `feature/login` path under the worktree root.
- `create_run_with_registry` stores branch `feature/login` and passes the nested worktree path to `git worktree add`.

Full verification remains `./init.sh`.
