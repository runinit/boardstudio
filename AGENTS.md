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
- **Single build per target:** Never overlap manual `cargo` or `wasm-pack` runs against the same `target` directory or tight-poll background jobs. `scripts/check.py` owns bounded concurrency, strictly capped by `BOARDSTUDIO_CHECK_JOBS`. Keep `lint` overlapped in `target/lint`: Cargo hashes `build` and `clippy` dependency modes separately, so `clippy` cannot reuse external `.rlib`s from `build`/`test`.
- **Manifest hygiene:** Leave zero Cargo warnings (`did not match any packages`, ignored workspace-member `resolver`). Never inspect or print `target/dx/.captured-args/*.json`.

## Editing & Verification

- **Inventory & scripted edits:** Find all uses of a changed type/field by removing it and reading `cargo check`. For scripted edits, replace one exact unique string, assert match count, read the diff before testing, and WIP-commit before large rewrites. Never use `git stash`.
- **Checks & baselines:** Use `python3 scripts/check.py quick` during iteration and `check.py` for full gates. After merging parallel worktrees, run `python3 scripts/check.py repo tooling` before continuing (cross-script tests live outside individual script test files). Update native test baselines only via `check.py test --update-native-baseline` and verify zero test names were dropped. Mutation checks use `scripts/mutation-check.py`.
- **Evidence:** `.evidence/` holds logs only (plus committed handoff records). Redirect long check/mutation logs there and report concise results. Never create `target` dirs, crate copies, `before/`/`after/` trees, or custom SHA-256 manifests inside `.evidence/`.

## Workflow & Conventions

- Browser automation: use `agent-browser` (`agent-browser --help`: `open` -> `snapshot -i` -> `click`/`fill` by `@ref` -> re-snapshot).
- Tickets & triage: [issue tracker](docs/agents/issue-tracker.md), [triage labels](docs/agents/triage-labels.md), [domain docs](docs/agents/domain.md).
- Orchestration: [model routing](docs/agents/model-routing.md), [model lookup](docs/agents/model-lookup.md).
- Historical workflow documents are not operating instructions.
