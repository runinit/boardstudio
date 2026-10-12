# BoardStudio

- Commands: [README.md](README.md).
- Architecture: [architecture](docs/architecture.md), [terms](CONTEXT.md).
- UI/product: [DESIGN.md](DESIGN.md), [PRODUCT.md](PRODUCT.md).
- Hardware: [component onboarding](docs/hardware/component-onboarding.md).

## Editing

- Inventory: to find every use of a field or type you change, remove it and read the `cargo check` errors. `rtk proxy rg` serves exhaustive text inventories; plain `rg` output is condensed.
- Scripted edits: replace one exact, unique string and assert the match count. Read the diff before running tests. Checkpoint with a WIP commit on your branch before a large scripted edit.
- Verification: mutation checks use `scripts/mutation-check.py`; `check.py test` guards native test counts by test name.
- Evidence: logs, hashes and mutation results go in the gitignored `.evidence/`.

## Environment

- `$TMPDIR` changes when the sandbox is lifted; write anything that must survive to an absolute path.
- Cargo builds through the shared `target` symlink (`~/.cache/boardstudio-target`). Run it with the sandbox lifted unless `.claude/settings.json` allowlists that directory; `.worktreeinclude` copies that file into new worktrees.
- GitNexus: refresh with `python3 scripts/gitnexus-refresh.py`, or `analyze --index-only`; any other `analyze` rewrites AGENTS.md and CLAUDE.md.

Historical workflow documents are not operating instructions.
