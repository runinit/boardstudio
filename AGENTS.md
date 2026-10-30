# BoardStudio

- Commands: [README.md](README.md).
- Architecture: [architecture](docs/architecture.md), [decisions](docs/adr/README.md), [terms](GLOSSARY.md).
- UI/product: [DESIGN.md](DESIGN.md), [PRODUCT.md](PRODUCT.md).
- Hardware: [component onboarding](docs/hardware/component-onboarding.md).

## Editing

- Inventory: to find every use of a field or type you change, remove it and read the `cargo check` errors.
- Scripted edits: replace one exact, unique string and assert the match count. Read the diff before running tests. Checkpoint with a WIP commit on your branch before a large scripted edit.
- CAD migration review: follow the [map decisions](docs/plans/cad-backend-replacement/map.md#closure-and-workflow-decisions-2026-10-18-user). Run one combined spec+standards review at ticket closure or when a change touches public API, geometry, allowances or a cross-crate contract; private storage and test-only slices get RED/GREEN evidence and a diff read instead. An explicit `code-review` request and 012's final two-axis review still apply.
- Verification: mutation checks use `scripts/mutation-check.py`; `check.py test` guards native test counts by test name.
- Evidence: redirect long check output and mutation logs to the gitignored `.evidence/` directory and report only the result and relevant failures. Never create Cargo `target` directories or copy crate source trees inside `.evidence/`. Never generate `PREIMAGES.json`, `POSTIMAGES.json`, `before/` or `after/` file copies, or custom SHA-256 manifest files—Git already tracks source state.

## Environment & Build Guardrails

- `$TMPDIR` changes when the sandbox is lifted; write anything that must survive to an absolute path.
- Cargo: lift the sandbox unless `.claude/settings.json` allowlists the directory `readlink -f target` prints (it differs per worktree: lane worktrees point into `~/.cache/boardstudio-target`, session worktrees share the main checkout's `target`). `.worktreeinclude` carries that file into new worktrees.
- Build flags: let `.cargo/config.toml` (`jobs = 12`) and `scripts/run-wasm-tests.py` (`CARGO_PROFILE_DEV_DEBUG=false`) govern concurrency and debuginfo. Never pass `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, or `CARGO_PROFILE_DEV_DEBUG=2`.
- Single build per target: do not manually launch overlapping `cargo` or `wasm-pack` commands against the same `target` directory or busy-poll background commands in a tight loop. The supported `scripts/check.py` scheduler owns its bounded independent launches, governed by `BOARDSTUDIO_CHECK_JOBS`.

Historical workflow documents are not operating instructions.

## Browser Automation

Use `agent-browser` for web automation. Run `agent-browser --help` for all commands.

Core workflow:

1. `agent-browser open <url>` - Navigate to page
2. `agent-browser snapshot -i` - Get interactive elements with refs (@e1, @e2)
3. `agent-browser click @e1` / `fill @e2 "text"` - Interact using refs
4. Re-snapshot after page changes

### Issue tracker

Committed Markdown tickets under `docs/plans/<plan>/issues/`. See `docs/agents/issue-tracker.md`.

### Triage labels

The five default roles, recorded as a ticket's `Status:`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/`. See `docs/agents/domain.md`.

### Orchestration

- Routing delegated planning, implementation, research, checks, and review: [model routing](docs/agents/model-routing.md).
- Looking up one selected provider/model for delegation: [model lookup](docs/agents/model-lookup.md).

<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->

<!-- gortex:communities:start -->
## Community Skills

| Area | Description | Explore |
|------|-------------|---------|
| Src Objects 55 Dirs | 4117 symbols | `analyze(operation:"communities", id:"community-557")` |
| Tests 49 Dirs | 2055 symbols | `analyze(operation:"communities", id:"community-529")` |
| Core Src 28 Dirs | 1441 symbols | `analyze(operation:"communities", id:"community-175")` |
| Runtime Src 12 Dirs | 1365 symbols | `analyze(operation:"communities", id:"community-527")` |
| Scripts 2 Dirs Append | 1023 symbols | `analyze(operation:"communities", id:"community-295")` |
| Tests 19 Dirs | 987 symbols | `analyze(operation:"communities", id:"community-139")` |
| Runtime Src 24 Dirs | 965 symbols | `analyze(operation:"communities", id:"community-528")` |
| Monstertruck Src 44 Dirs | 662 symbols | `analyze(operation:"communities", id:"community-222")` |
| Runtime Src 11 Dirs | 633 symbols | `analyze(operation:"communities", id:"community-1")` |
| Case Src 14 Dirs | 594 symbols | `analyze(operation:"communities", id:"community-433")` |
| Presentation 23 Dirs | 593 symbols | `analyze(operation:"communities", id:"community-526")` |
| Artifact 7 Dirs | 578 symbols | `analyze(operation:"communities", id:"community-123")` |
| Mechanical 10 Dirs | 481 symbols | `analyze(operation:"communities", id:"community-209")` |
| Core Src 38 Dirs | 447 symbols | `analyze(operation:"communities", id:"community-74")` |
| Core Src 2 Dirs Collect | 443 symbols | `analyze(operation:"communities", id:"community-192")` |
| Host Src 1 Dirs Run Request | 415 symbols | `analyze(operation:"communities", id:"community-352")` |
| Src Step 14 Dirs | 406 symbols | `analyze(operation:"communities", id:"community-50")` |
| Src Objects 16 Dirs | 378 symbols | `analyze(operation:"communities", id:"community-395")` |
| Mechanical 6 Dirs | 372 symbols | `analyze(operation:"communities", id:"community-153")` |
| Core Src 3 Dirs Apply Change | 348 symbols | `analyze(operation:"communities", id:"community-149")` |

<!-- gortex:communities:end -->
