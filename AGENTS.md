# BoardStudio

- Architecture (ownership across Rust and browser providers): [docs/architecture.md](docs/architecture.md). Domain terms: [CONTEXT.md](CONTEXT.md). Directory map: Source layout in [README.md](README.md).
- Development and check commands: [README.md](README.md). Browser presentation compiles only for WASM; native tests alone do not validate it.
- Before finishing, run `python3 scripts/check.py quick` (checks only what your changes touch; never `build`). `--list` shows every step: `repo`, `tooling`, `lint`, `build`, `test`, `typecheck`, `browser`, and the opt-in `release`; `deny` after dependency changes. Presentation changes need `typecheck` and `browser`, which `quick` includes. Run the full check (`python3 scripts/check.py`) before a PR, and `release` before shipping.
- Components, footprints, modules, models: read [component onboarding](docs/hardware/component-onboarding.md) before changing.
- UI or visual work: read [DESIGN.md](DESIGN.md) (large; only for UI). Product scope and users: [PRODUCT.md](PRODUCT.md). Other docs: [docs/README.md](docs/README.md); deferred issues: [docs/backlog.md](docs/backlog.md).
- Preserve unrelated work and user project data. Keep fixes and their verification within the affected behavior.

## Checks and builds

- Daily loop: `python3 scripts/check.py quick`. Pre-PR: `python3 scripts/check.py`. `release` (optimized `wasm-opt` and page build) is opt-in, for shipping or CI; do not run it unless asked. Details: [README.md](README.md#checks).
- Cargo uses 12 jobs from `.cargo/config.toml`; never pass `CARGO_BUILD_JOBS=2` (old plan docs do). Do not `touch core/src/lib.rs` before runs: cargo already rebuilds on real edits. Use `CARGO_TARGET_DIR=<main checkout>/target` in worktrees, shared by all of them.
- A new worktree needs `ln -s <main checkout>/target target` (the tooling tests look for `target/` in the checkout) and, for `build`, a real copy of `cad/.cache` and `cad/wasm/pkg` (untracked; a symlink dangles inside the CAD container). Nested worktrees cannot run the native `test` step (the separate CAD workspace breaks); run it from a non-nested checkout such as `/tmp/boardstudio-tce-verify`.
- `wasm-pack`, headless Chrome and the CAD container need the agent sandbox lifted. There is no outbound network.
- `/tmp` is a 16 GB tmpfs. Put throwaway target directories on disk (`~/.cache/...`), never under `$TMPDIR`, and delete them: a full tmpfs loses command output.
- Test wasm is built without debug info for speed (`CARGO_PROFILE_DEV_DEBUG=false` in the check scripts). Run `wasm-pack test` by hand, without it, to debug in DevTools.
- `scripts/dev-web.py` rebuilds provider assets only when one is missing, not when stale; run `python3 scripts/build-web.py --providers-only` before judging UI behavior.
- Measure before optimizing: `check.py` prints each command's wall time. Executed test counts must not drop (see the baseline in [docs/plans/check-speed/issues/02-timing-and-count-baseline.md](docs/plans/check-speed/issues/02-timing-and-count-baseline.md)).

## Git safety

- Stage explicit paths only; never `git add -A` or `git add .`.
- Never `git stash`: the working tree may hold uncommitted work that a stash would capture.
- To commit part of a file that also holds unrelated edits, build the staged blob from `git show HEAD:<file>` plus your edit (`git hash-object -w`, `git update-index --cacheinfo`) so the working tree keeps both.

## Rust

- Toolchain and edition: `rust-toolchain.toml` (1.98, edition 2024). Workspace lints live in the root `Cargo.toml`; the `lint` step denies warnings, so new code must be rustfmt- and Clippy-clean.
- Suppress a lint on the narrowest item only, with a trailing comment saying why; a workspace-wide allow needs its justification in `Cargo.toml`.
- Errors: follow the surrounding module's error shape (many Core and Session APIs return `Result<_, String>`). Do not add `thiserror`, `anyhow` or a new error hierarchy unless the ticket asks for it.
- Browser crates are single-threaded WASM on Dioxus: `Rc`, `RefCell` and signals are the norm; do not add `Arc`, `Mutex` or `Send` bounds there.
- The `rust-skills` and `rust-best-practices` skills, when installed, apply to writing and reviewing Rust. Where they disagree with this file, the workspace lints or the surrounding crate's conventions, the repo wins.

## Docs and tracking

- Issue tracker: specs, wayfinder maps and tickets are Markdown under `docs/plans/<effort>/`; see `docs/agents/issue-tracker.md`.
- Domain docs: single-context, `CONTEXT.md` at the root and ADRs in `docs/adr/`.

## Tooling

- Code questions, call paths and blast radius: GitNexus (indexed as `boardstudio`). Use `query` for concepts and flows, `context` for a named symbol, and `impact` (upstream) before changing a shared symbol's signature or behaviour; the `gitnexus-*` skills cover each workflow. A `risk: UNKNOWN` or empty caller set is unanswered, not safe: confirm with a text search before changing or deleting. Rename symbols with GitNexus `rename`, not find-and-replace.
- Text search: `rg`, which honours `.gitignore` and so skips agent worktrees (`.claude/worktrees/`, `.worktrees/`, `.pi/worktrees/`) and tool state; with `grep -r`, pass `--exclude-dir={.claude,.worktrees,.pi,target,node_modules}`.
- Library, framework, or API docs (Rust crates, wasm-bindgen, web-sys, browser APIs): Context7 (`resolve-library-id`, then `query-docs`) rather than memory or web search.
- Past decisions, "why did we do X", prior debugging: `memsearch:memory-recall` before re-deriving. Skip it for questions about current code state. Where memory disagrees with `CONTEXT.md`, ADRs, or `docs/plans/`, the docs win.
