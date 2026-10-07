# BoardStudio

- Architecture (ownership across Rust and browser providers): [docs/architecture.md](docs/architecture.md). Domain terms: [CONTEXT.md](CONTEXT.md). Directory map: Source layout in [README.md](README.md).
- Development and check commands: [README.md](README.md). Browser presentation compiles only for WASM; native tests alone do not validate it.
- Before finishing, run the affected steps of `python3 scripts/check.py` (`--list` shows them: `repo`, `tooling`, `lint`, `build`, `test`, `typecheck`, `browser`; `deny` after dependency changes). Presentation changes need `typecheck` and `browser`. Run the full check before a PR.
- Components, footprints, modules, models: read [component onboarding](docs/hardware/component-onboarding.md) before changing.
- UI or visual work: read [DESIGN.md](DESIGN.md) (large; only for UI). Product scope and users: [PRODUCT.md](PRODUCT.md). Other docs: [docs/README.md](docs/README.md); deferred issues: [docs/backlog.md](docs/backlog.md).
- Preserve unrelated work and user project data. Keep fixes and their verification within the affected behavior.

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
