# BoardStudio

- Architecture (ownership across Rust and browser providers): [docs/architecture.md](docs/architecture.md). Domain terms: [CONTEXT.md](CONTEXT.md). Directory map: Source layout in [README.md](README.md).
- Development and check commands: [README.md](README.md). Browser presentation compiles only for WASM; native tests alone do not validate it.
- Before finishing, run the affected steps of `python3 scripts/check.py` (`--list` shows them: `repo`, `tooling`, `build`, `test`, `typecheck`, `browser`). Presentation changes need `typecheck` and `browser`. Run the full check before a PR.
- Components, footprints, modules, models: read [component onboarding](docs/hardware/component-onboarding.md) before changing.
- UI or visual work: read [DESIGN.md](DESIGN.md) (large; only for UI). Product scope and users: [PRODUCT.md](PRODUCT.md). Other docs: [docs/README.md](docs/README.md); deferred issues: [docs/backlog.md](docs/backlog.md).
- Preserve unrelated work and user project data. Keep fixes and their verification within the affected behavior.

## Git safety

- Stage explicit paths only; never `git add -A` or `git add .`.
- Never `git stash`: the working tree may hold uncommitted work that a stash would capture.
- To commit part of a file that also holds unrelated edits, build the staged blob from `git show HEAD:<file>` plus your edit (`git hash-object -w`, `git update-index --cacheinfo`) so the working tree keeps both.

## Docs and tracking

- Issue tracker: specs, wayfinder maps and tickets are Markdown under `docs/plans/<effort>/`; see `docs/agents/issue-tracker.md`.
- Domain docs: single-context, `CONTEXT.md` at the root and ADRs in `docs/adr/`.

## Tooling

- Code questions and before edits: `codegraph_explore` before grep or reading files; name symbols or files in the query.
- Library, framework, or API docs (Rust crates, wasm-bindgen, web-sys, browser APIs): Context7 (`resolve-library-id`, then `query-docs`) rather than memory or web search.
- Past decisions, "why did we do X", prior debugging: `memsearch:memory-recall` before re-deriving. Skip it for questions about current code state. Where memory disagrees with `CONTEXT.md`, ADRs, or `docs/plans/`, the docs win.
