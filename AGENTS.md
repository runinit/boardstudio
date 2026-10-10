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
- A new worktree needs `ln -s <main checkout>/target target` (the tooling tests look for `target/` in the checkout) and, for `build`, a real copy of `cad/.cache` and `cad/wasm/pkg` (untracked; a symlink dangles inside the CAD container). Nested worktrees (under `.worktrees/`) run every step, including native `test`: each CAD crate declares its own `[workspace]`, so no second checkout is needed. `build` refuses a symlinked `cad/.cache`, `cad/wasm/pkg` or `cad/wasm/target` (the container cannot resolve it); copy it as a real directory instead. See [docs/plans/check-speed/issues/10-nested-worktrees.md](docs/plans/check-speed/issues/10-nested-worktrees.md).
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

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **boardstudio** (16965 symbols, 49914 relationships, 565 execution flows).

> Index stale? Run `node .gitnexus/run.cjs analyze --index-only` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? Bootstrap with `npx`, `bunx`, or `pnpm dlx` — e.g. `bunx gitnexus@latest analyze` (npm 11 npx crash; #1939).

## Always Do

- **MUST run impact before editing.** Use `impact({target: "symbolName", direction: "upstream"})` or `node .gitnexus/run.cjs impact "symbolName" --direction upstream --repo .`; report callers, processes, and risk. Never substitute grep for graph analysis.
- **MUST analyze graph changes before committing.** Use `detect_changes({scope: "all"})` (MCP) or `node .gitnexus/run.cjs detect-changes --scope all --repo .` (CLI fallback). `partial: true` or `truncated: true` is not a clean check — a zero means unseen, not unaffected; re-run it. For regression review: `detect_changes({scope: "compare", base_ref: "main"})` or `node .gitnexus/run.cjs detect-changes --scope compare --base-ref "main" --repo .`.
- MUST warn on HIGH/CRITICAL `risk` pre-edit; never use `riskSharedAxes` to waive a HIGH/CRITICAL `risk` warning. Compare File/symbol: MCP File omits axes; Graph-RAG expands File.
- **MUST treat `risk: UNKNOWN` as unresolved, not as low.** An empty caller set is not evidence the symbol is unused — it can also mean the callers are not resolvable by the index (plain-object property access, dynamic dispatch, cross-language calls). `impact` pairs `UNKNOWN` with a `riskNote` saying so. Confirm with a text search before treating the symbol as safe to change or delete; do not proceed on the strength of a zero.
- **MUST use `query({search_query: "concept"})` for concepts/flows, `context({name: "symbolName"})` for a named symbol, or `impact` for blast radius, on read-only callers, dependencies, imports, or execution flow.** Graph first; text search only for empty/`UNKNOWN`/literals.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method before MCP/CLI impact analysis.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis, and never read `UNKNOWN` as an all-clear — it means the walk could not answer, which is the one verdict that requires confirming by other means.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit before MCP/CLI graph change analysis.

## Resources

| Resource | Use for |
| --- | --- |
| `gitnexus://repo/boardstudio/context` | Codebase overview, check index freshness |
| `gitnexus://repo/boardstudio/clusters` | All functional areas |
| `gitnexus://repo/boardstudio/processes` | All execution flows |
| `gitnexus://repo/boardstudio/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
| --- | --- |
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
