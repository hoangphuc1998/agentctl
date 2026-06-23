# Quick Create Run Prefill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add contextual New Run actions that open the existing modal with editable defaults from a clicked repo or run.

**Architecture:** Keep the backend unchanged. `App` owns create-run defaults, `RepoRunTree` emits source-specific create events, and `CreateRunModal` initializes editable local form state from defaults when opened.

**Tech Stack:** React 18, TypeScript, lucide-react, Vitest, Testing Library, Tauri invoke API.

---

## File Structure

- Modify `src/types.ts` to add the `CreateRunDefaults` type used by `App` and `CreateRunModal`.
- Modify `src/components/CreateRunModal.tsx` so the modal accepts `defaults` and resets editable state on open.
- Modify `src/components/RepoRunTree.tsx` to render contextual plus icon buttons on repo and run rows.
- Modify `src/App.tsx` to set create defaults from repo, run, or current active repo before opening the modal.
- Modify `src/components/CreateRunModal.test.tsx`, `src/components/RepoRunTree.test.tsx`, and `src/App.test.tsx` with red tests before implementation.
- Modify `src/styles.css` only for compact icon placement inside existing row layouts.
- Modify `feature_list.json` and `progress.md` after verification.

### Task 1: Modal Defaults

**Files:**
- Modify: `src/types.ts`
- Modify: `src/components/CreateRunModal.test.tsx`
- Modify: `src/components/CreateRunModal.tsx`

- [ ] **Step 1: Write the failing modal prefill tests**

Add tests asserting that `CreateRunModal` renders defaults and submits edited values:

```tsx
it("prefills editable create fields from defaults", () => {
  render(
    <CreateRunModal
      open
      activeRepoPath={null}
      defaults={{
        repoPath: "/repo/agent-manager",
        baseRef: "main",
        tag: "review",
        agent: "claude"
      }}
      onClose={vi.fn()}
      onCreated={vi.fn()}
      onError={vi.fn()}
    />
  );

  expect(screen.getByLabelText(/repo path/i)).toHaveValue("/repo/agent-manager");
  expect(screen.getByLabelText(/base ref/i)).toHaveValue("main");
  expect(screen.getByLabelText(/tag/i)).toHaveValue("review");
  expect(screen.getByRole("button", { name: /claude/i })).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByLabelText(/run name/i)).toHaveValue("");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/components/CreateRunModal.test.tsx`

Expected: fail because `CreateRunModal` does not accept `defaults`.

- [ ] **Step 3: Add minimal modal default support**

Add this type to `src/types.ts`:

```ts
export interface CreateRunDefaults {
  repoPath?: string;
  baseRef?: string;
  tag?: string;
  agent?: AgentKind;
}
```

Update `CreateRunModalProps` to include `defaults: CreateRunDefaults | null`. Initialize form state from a local `initialValues` object with fallback values:

```ts
const initialValues = {
  repoPath: defaults?.repoPath ?? activeRepoPath ?? "",
  baseRef: defaults?.baseRef ?? "HEAD",
  tag: defaults?.tag ?? "default",
  agent: defaults?.agent ?? "codex"
};
```

Reset local state from those values when `open`, `activeRepoPath`, or `defaults` changes, and keep `runName` blank on open.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- src/components/CreateRunModal.test.tsx`

Expected: pass.

### Task 2: Tree Quick-Create Buttons

**Files:**
- Modify: `src/components/RepoRunTree.test.tsx`
- Modify: `src/components/RepoRunTree.tsx`
- Modify: `src/styles.css`

- [ ] **Step 1: Write failing tree interaction tests**

Add tests asserting repo and run quick-create callbacks:

```tsx
it("requests a new run from a repo row", async () => {
  const onCreateRunFromRepo = vi.fn();
  render(
    <RepoRunTree
      repos={repos}
      selectedRunId={null}
      onSelectRun={vi.fn()}
      onCreateRunFromRepo={onCreateRunFromRepo}
      onCreateRunFromRun={vi.fn()}
    />
  );

  await userEvent.click(screen.getByRole("button", { name: /new run from agent-manager/i }));

  expect(onCreateRunFromRepo).toHaveBeenCalledWith(repos[0]);
});

it("requests a new run from an existing run without selecting it", async () => {
  const onCreateRunFromRun = vi.fn();
  const onSelectRun = vi.fn();
  render(
    <RepoRunTree
      repos={repos}
      selectedRunId={null}
      onSelectRun={onSelectRun}
      onCreateRunFromRepo={vi.fn()}
      onCreateRunFromRun={onCreateRunFromRun}
    />
  );

  await userEvent.click(screen.getByRole("button", { name: /new run from api-cleanup/i }));

  expect(onCreateRunFromRun).toHaveBeenCalledWith(repos[0].runs[1]);
  expect(onSelectRun).not.toHaveBeenCalled();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/components/RepoRunTree.test.tsx`

Expected: fail because quick-create buttons and callbacks do not exist.

- [ ] **Step 3: Add contextual plus buttons**

Add `onCreateRunFromRepo` and `onCreateRunFromRun` props to `RepoRunTree`. Render icon buttons with `Plus` from `lucide-react`. In the run button handler, call `event.stopPropagation()` before invoking `onCreateRunFromRun(run)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- src/components/RepoRunTree.test.tsx`

Expected: pass.

### Task 3: App Wiring

**Files:**
- Modify: `src/App.test.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Write failing App integration tests**

Add tests asserting clicked-position defaults:

```tsx
it("opens New Run prefilled from a repo row", async () => {
  vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
  render(<App />);

  await screen.findByRole("heading", { name: "login-flow" });
  await userEvent.click(screen.getByRole("button", { name: /new run from agent-manager/i }));

  expect(screen.getByLabelText(/repo path/i)).toHaveValue("/repo/agent-manager");
  expect(screen.getByLabelText(/base ref/i)).toHaveValue("HEAD");
  expect(screen.getByLabelText(/tag/i)).toHaveValue("default");
  expect(screen.getByRole("button", { name: /codex/i })).toHaveAttribute("aria-pressed", "true");
});

it("opens New Run prefilled from an existing run and submits edited data", async () => {
  vi.mocked(dashboardState).mockResolvedValue(dashboard("run-1"));
  vi.mocked(createRun).mockResolvedValue({ message: "Created.", run: null });
  render(<App />);

  await screen.findByRole("heading", { name: "login-flow" });
  await userEvent.click(screen.getByRole("button", { name: /new run from api-cleanup/i }));
  await userEvent.type(screen.getByLabelText(/run name/i), "api-followup");
  await userEvent.clear(screen.getByLabelText(/base ref/i));
  await userEvent.type(screen.getByLabelText(/base ref/i), "release");
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

  await waitFor(() =>
    expect(createRun).toHaveBeenCalledWith({
      repoPath: "/repo/agent-manager",
      baseRef: "release",
      tag: "default",
      runName: "api-followup",
      agent: "codex"
    })
  );
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/App.test.tsx`

Expected: fail because `App` does not pass quick-create callbacks or modal defaults.

- [ ] **Step 3: Wire defaults in App**

Add `createDefaults` state, an `openCreateRun(defaults)` callback, and pass contextual callbacks to `RepoRunTree`. Pass `defaults={createDefaults}` to `CreateRunModal`.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- src/App.test.tsx`

Expected: pass.

### Task 4: Full Verification and Artifacts

**Files:**
- Modify: `feature_list.json`
- Modify: `progress.md`

- [ ] **Step 1: Run focused tests**

Run: `npm test -- src/components/CreateRunModal.test.tsx src/components/RepoRunTree.test.tsx src/App.test.tsx`

Expected: pass.

- [ ] **Step 2: Run full frontend tests and build**

Run: `npm test`

Expected: all Vitest tests pass.

Run: `npm run build`

Expected: TypeScript and Vite build pass.

- [ ] **Step 3: Run standard verification**

Run: `./init.sh`

Expected: exits 0.

- [ ] **Step 4: Update artifacts**

Add `feat-022` to `feature_list.json` with verification evidence. Update `progress.md` with this session's changed files, decisions, and command results.

- [ ] **Step 5: Commit**

Run:

```bash
git add docs/superpowers/specs/2026-06-23-quick-create-run-prefill-design.md docs/superpowers/plans/2026-06-23-quick-create-run-prefill.md src/types.ts src/components/CreateRunModal.tsx src/components/CreateRunModal.test.tsx src/components/RepoRunTree.tsx src/components/RepoRunTree.test.tsx src/App.tsx src/App.test.tsx src/styles.css feature_list.json progress.md
git commit -m "Add quick create run prefill actions"
```

Expected: commit succeeds.
