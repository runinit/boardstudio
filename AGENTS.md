# BoardStudio Dioxus migration: agent entry point

Worktree: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`
Branch: `codex/rust-v1-ui-parity-20261001`. Stay on it; do not push; do not use bare `git stash`.

Read in order:
1. `handoff.md`: current state, hold items, traps, next work.
2. `CONSTRAINTS.md`: the operating contract (delivery loop, token discipline, agent routing). Use the Codex route.
3. `.scratch/dioxus-frontend-v1/progress.py`: records tool. `frontier` shows grouped functional work separately from acceptance; `handoff` regenerates the generated half of `handoff.md`.

Rules:
- Stage commits by explicit path only; the tree has many unrelated untracked files.
- Use built-in tools first: shell, `rg`, web search, the built-in browser plugin, native subagents. Use CodeGraph when this checkout has a `.codegraph/` index; otherwise use targeted reads/search. Use `cargo doc` or web search for Dioxus/Rust API facts.
- Presentation code is wasm-only: run the wasm `page` check and `scripts/run-wasm-tests.py`; a plain `#[test]` there does not run.
- Pipe build/test output through `tail`/`grep`. No screenshots or `innerText` dumps unless a visual difference is under test.
- Subagent reports are 150 words or less, with detail in a file.
