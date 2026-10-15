# BoardStudio

- Commands: [README.md](README.md).
- Architecture: [architecture](docs/architecture.md), [decisions](docs/adr/README.md), [terms](CONTEXT.md).
- UI/product: [DESIGN.md](DESIGN.md), [PRODUCT.md](PRODUCT.md).
- Hardware: [component onboarding](docs/hardware/component-onboarding.md).

## Editing

- Inventory: to find every use of a field or type you change, remove it and read the `cargo check` errors.
- Scripted edits: replace one exact, unique string and assert the match count. Read the diff before running tests. Checkpoint with a WIP commit on your branch before a large scripted edit.
- Verification: mutation checks use `scripts/mutation-check.py`; `check.py test` guards native test counts by test name.
- Evidence: logs, hashes and mutation results go in the gitignored `.evidence/`; redirect long check output there and report only its result and relevant failures.

## Environment

- `$TMPDIR` changes when the sandbox is lifted; write anything that must survive to an absolute path.
- Cargo: lift the sandbox unless `.claude/settings.json` allowlists the directory `readlink -f target` prints (it differs per worktree: lane worktrees point into `~/.cache/boardstudio-target`, session worktrees share the main checkout's `target`). `.worktreeinclude` carries that file into new worktrees.

Historical workflow documents are not operating instructions.

## Agent skills

### Issue tracker

Committed Markdown tickets under `docs/plans/<plan>/issues/`. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default roles, recorded as a ticket's `Status:`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/`. See `docs/agents/domain.md`.
