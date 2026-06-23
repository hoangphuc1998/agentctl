# Quick Create Run Prefill Design

## Goal

Let users create a new run from the repo or run row they are looking at by opening the existing New Run modal with editable prefilled values.

## Scope

This feature changes only the desktop React UI. It does not add a backend command because the existing `create_run` command already accepts `repoPath`, `baseRef`, `tag`, `runName`, and `agent`.

## Interaction Design

The left workspace tree gains contextual icon buttons:

- Repo row plus button: opens the New Run modal with `repoPath` from the clicked repo, `baseRef` set to `HEAD`, `tag` set to `default`, `agent` set to `codex`, and `runName` blank.
- Run row plus button: opens the New Run modal with `repoPath`, `baseRef`, `tag`, and `agent` copied from the clicked run, and `runName` blank.
- Top-bar and empty-state New Run actions continue to open the modal with the active repo path fallback.

The modal remains fully editable. Prefilled values are initial values for speed, not locked values. Clicking a run-row quick-create button must not select that run.

## Component Shape

`App` owns a small `CreateRunDefaults` object. Opening New Run from different positions sets those defaults before rendering `CreateRunModal`.

`RepoRunTree` receives two callbacks:

- `onCreateRunFromRepo(repo)`
- `onCreateRunFromRun(run)`

`CreateRunModal` receives `defaults` and resets its local form state from those defaults each time it opens.

## Data Flow

1. User clicks a contextual plus button.
2. `RepoRunTree` calls the matching callback with the repo or run data.
3. `App` converts that source into `CreateRunDefaults` and opens the modal.
4. `CreateRunModal` initializes editable fields from the defaults.
5. On submit, the existing `createRun` API sends the current edited form values.

## Error Handling

Existing required-field validation and backend error display stay unchanged. The new contextual buttons stop event propagation so row selection does not fire accidentally.

## Testing

Vitest and Testing Library cover:

- Repo quick-create opens the modal prefilled with that repo path and default create values.
- Run quick-create opens the modal prefilled from that run while leaving run name blank.
- Editing prefilled values changes the final `createRun` payload.
- The run-row quick-create button does not select the run.

Full verification remains `./init.sh`, which runs npm install/checks when dependencies are present and cargo tests.
