# BoardStudio

## References

- Commands: [README.md](README.md)
- Standards (read during authoring & review): [CODING_STANDARDS.md](CODING_STANDARDS.md)
- Architecture & decisions: [architecture](docs/architecture.md), [ADRs](docs/adr/README.md), [glossary](GLOSSARY.md)
- Product & UI: [PRODUCT.md](PRODUCT.md), [DESIGN.md](DESIGN.md)
- Hardware: [component onboarding](docs/hardware/component-onboarding.md)

## Build & Environment

- **Shared build lock & shallow `TMPDIR`:** Hold `~/.cache/tmp/boardstudio-build.lock` with a blocking `flock` (never fail-fast `flock -n`) and export `BOARDSTUDIO_BUILD_LOCK_FD=9 TMPDIR="$HOME/.cache/tmp"` for Cargo and browser runs. `$TMPDIR` changes when the sandbox is lifted—write persistent files to absolute paths.
- **Chrome Unix socket limit:** Linux caps `SingletonSocket` at 108 bytes. Always launch Chrome via `browser_tools.shallow_browser_tmpdir()` / `browser_tools.browser_env()`, never raw sandbox/worktree `$TMPDIR`.
- **Cargo sandbox & `PATH`:** Lift the sandbox unless `.claude/settings.json` (carried by `.worktreeinclude`) allowlists `readlink -f target`. Non-login shells may omit `~/.cargo/bin` (where `wasm-pack` lives) from `PATH`.
- **Profiles & concurrency:** Let `.cargo/config.toml` (`jobs = 12`) and repo profiles (`dev`, `wasm-check`, `cad-test`) govern builds. Never pass `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, or `CARGO_PROFILE_DEV_DEBUG=2`, and never add a `rustc-wrapper` to repo `.cargo/config.toml`.
- **Single build per target:** Never overlap manual `cargo` or `wasm-pack` runs against the same `target` directory or tight-poll background jobs. `scripts/check.py` owns bounded concurrency: the `browser` batch and individual parallel pools are capped by `BOARDSTUDIO_CHECK_JOBS`, while overlapped `build`/`lint`/`test` coordinate via a shared GNU Make jobserver FIFO. Keep `lint` overlapped in `target/lint`: Cargo hashes `build` and `clippy` dependency modes separately, so `clippy` cannot reuse external `.rlib`s from `build`/`test`.
- **Manifest & dependency hygiene:** Leave zero Cargo manifest warnings (`did not match any packages`, ignored workspace-member `resolver`). Never inspect or print `target/dx/.captured-args/*.json`. Before adopting any new crate, verify its transitive licenses against the allowlist and version-specific exceptions in [deny.toml](deny.toml) and run a `wasm32-unknown-unknown` + `.wasm`-size spike.
- **Worktree & disk hygiene:** Immediately `git worktree remove` merged, clean delegate worktrees (each Rust `target/` burns 1–3 GiB). Periodically check `df -h .` during multi-wave runs and keep >50 GiB free.

## Editing & Verification

- **CLI tools (`rg`, `fd`, `ast-grep`/`sg`, `sd`, `jq`, `yq`, `tokei`, `difft`, `twiggy`, `wasm-tools`, `cargo-nextest`):** Always use `rg` for content search and `fd` (or `rg --files`) for file discovery—never run `grep` or `find` in shell commands. Use `ast-grep` (`sg`) for structural AST search/codemods, `sd` instead of `sed`, `jq`/`yq` for JSON/YAML/TOML queries, `tokei` for LOC counts, `difft` for structural diffs, `twiggy`/`wasm-tools` for WASM size/section inspection, and `cargo nextest` for fast targeted Rust test runs (ensure `~/.cargo/bin` and `~/.local/bin` are on `PATH`). Never run unbounded or unpruned filesystem searches.
- **Inventory & scripted edits:** Find all uses of a changed type/field by removing it and reading `cargo check`. For scripted edits, replace one exact unique string, assert match count, read the diff before testing, and WIP-commit before large rewrites. Never use `git stash`.
- **Checks & baselines:** Use `python3 scripts/check.py quick` during iteration and `check.py` for full gates. Parallel worktree delegates run only fast, targeted crate checks (`cargo test -p ...`, `cargo check`)—reserve full `check.py` (browser/release) for batched merges on the main branch so parallel lanes do not starve `~/.cache/tmp/boardstudio-build.lock`. After merging parallel worktrees, run `python3 scripts/check.py repo tooling` before continuing (cross-script tests live outside individual script test files). Mutation checks use `scripts/mutation-check.py`.
- **Simplification, refactoring & deduplication first (structural gates, advisory LOC):** Every wave must prioritize real simplification, clean refactoring, and deduplication—delete dead paths, collapse redundant state layers/prop-bags, and reuse or consolidate shared helpers rather than adding wrapper layers or feature flags. Never satisfy a deletion list by pasting code verbatim into a sibling file or keeping obsolete async-worker settlement ceremony (`PendingSettlement`, generation/request-ID guards, `ProjectionIdentity` mirrors) on synchronous `Runtime::commit` paths; never delete invariant comments to pad LOC or squeeze under file limits (`check_reduction.py` counts non-comment code lines). Enforce structural hard gates (`--must-delete`, `--forbid-symbol`, `--max-file-lines`, `--max-pattern`, `--max-relocation-ratio`, automatic `LAYER-DIRECTION`) plus a `<= 0` anti-bloat floor on refactor tracks; numeric LOC targets are advisory/observational only (no hard negative LOC quotas, and never line-golf or chase artificial Prod LOC offsets on correctness/safety work).
- **Permanent safety, listener & theme invariants:** Keep `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]` green across `core/crates/*` and `web/crates/runtime`; use `ui_shared::DomEventListener` for UI DOM events (zero new UI `Closure::wrap`/`.forget()` leaks); keep `web/tailwind.css` passing `scripts/check-theme-contrast.py` and in sync with `FIRST_PAINT_THEME_HEAD` in `scripts/build-web.py`; keep shared 3D viewport primitives downward in `ui-shared::viewport_3d` (no sideways `-> case` deps or upward `-> workbench::editor_session` imports).
- **Evidence:** `.evidence/` holds logs only (plus committed handoff records). Redirect long check/mutation logs there and report concise results. Never create `target` dirs, crate copies, `before/`/`after/` trees, or custom SHA-256 manifests inside `.evidence/`.

## Workflow & Conventions

- Browser automation: use `agent-browser` (`agent-browser --help`: `open` -> `snapshot -i` -> `click`/`fill` by `@ref` -> re-snapshot).
- Tickets & triage: [issue tracker](docs/agents/issue-tracker.md), [triage labels](docs/agents/triage-labels.md), [domain docs](docs/agents/domain.md).
- Orchestration: [model routing](docs/agents/model-routing.md), [model lookup](docs/agents/model-lookup.md).
- Historical workflow documents are not operating instructions.
