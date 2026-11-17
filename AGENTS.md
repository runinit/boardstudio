# BoardStudio

- Commands: [README.md](README.md).
- Architecture: [architecture](docs/architecture.md), [decisions](docs/adr/README.md), [terms](GLOSSARY.md).
- UI/product: [DESIGN.md](DESIGN.md), [PRODUCT.md](PRODUCT.md).
- Hardware: [component onboarding](docs/hardware/component-onboarding.md).

## Workflow authority

- Use installed BMad skills as the sole authority for project planning,
  ticketing, implementation orchestration, review and verification.
- Keep live task state in BMad's configured ticket tree and plans.
- Follow the active BMad skill and accepted functional requirements;
  do not add a separate project-wide validation or review protocol.
- Use repository documentation for commands, architecture and domain
  constraints—not retired workflow mandates.
- Keep concise outcomes in BMad plans; do not create duplicate
  validation documents or parallel status records.
- Historical plans and retired agent guides are evidence, not
  operating instructions. Canceled work remains canceled.
- The CAD backend replacement plan remains canceled as of 2026-11-15.

## Editing

- Scripted edits: replace one exact, unique string and assert the match count. Read the diff before running tests.
- Shared worktrees: `git stash` is shared by every worktree and session, so leave it alone.
- Gortex stale writes: delegates read through Gortex and write through its edit tools. When a Gortex write is rejected as stale, `view_building` or read-only, retry once after 30 seconds, then use native Edit/Write for that file and report it. Do not stop for a stale Gortex view; this is the project's "proceed without Gortex" exception for writes.
- Evidence: `.evidence/` holds logs only, except committed transfer records; redirect long check and mutation output there and report only results and relevant failures. Never create Cargo `target` directories or copy crate source trees inside `.evidence/`. Never generate `PREIMAGES.json`, `POSTIMAGES.json`, `before/` or `after/` file copies, or custom SHA-256 manifest files—Git already tracks source state.

## Environment & Build Guardrails

- `$TMPDIR` changes when the sandbox is lifted; write anything that must survive to an absolute path.
- Cargo: lift the sandbox unless `.claude/settings.json` allowlists the directory `readlink -f target` prints (it differs per worktree: lane worktrees point into `~/.cache/boardstudio-target`, session worktrees share the main checkout's `target`). `.worktreeinclude` carries that file into new worktrees.
- Build flags: let `.cargo/config.toml` (`jobs = 12`) and `scripts/run-wasm-tests.py` (`CARGO_PROFILE_DEV_DEBUG=false`) govern concurrency and debuginfo. Never pass `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, or `CARGO_PROFILE_DEV_DEBUG=2`.
- Build lock: a session that shares its `target` directory with other agents wraps each build or check in `bash -c 'exec 9>~/.cache/tmp/boardstudio-build.lock; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9; <command>'`.
- Single build per target: do not manually launch overlapping `cargo` or `wasm-pack` commands against the same `target` directory or busy-poll background commands in a tight loop. The supported `scripts/check.py` scheduler owns its bounded independent launches, governed by `BOARDSTUDIO_CHECK_JOBS`.

Historical workflow documents are not operating instructions.

## Browser Automation

Use `agent-browser` for web automation. Run `agent-browser --help` for all commands.

Core workflow:

1. `agent-browser open <url>` - Navigate to page
2. `agent-browser snapshot -i` - Get interactive elements with refs (@e1, @e2)
3. `agent-browser click @e1` / `fill @e2 "text"` - Interact using refs
4. Re-snapshot after page changes

### Agent documentation and retrieval

For agent drafts, research, transfers or Gortex exclusions, follow [documentation and retrieval](docs/agents/ai-documentation.md).

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/`. See `docs/agents/domain.md`.
