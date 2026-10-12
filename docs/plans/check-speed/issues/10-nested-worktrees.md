# 10: Nested worktrees run every check

Status: resolved
Type: build
Blocked by: none
Map: [map.md](../map.md)

## Goal

`check.py test`, `quick` and `build` run from a worktree nested under the main checkout (for
example `.worktrees/<name>`), with no second non-nested checkout. Before this, native `test` failed
there and agents had to build in `/tmp/claude-1000/boardstudio-tce-verify`, which duplicated builds
and invalidated the shared `target`.

## Root cause

- `check.py test` runs `cad/scripts/test-cadrum.py`, which runs
  `cargo build/test --manifest-path step-oracle/Cargo.toml` and
  `--manifest-path wasm/Cargo.toml` from `cad/`. Both CAD crates are listed in the root
  `Cargo.toml` `exclude`, but neither declares a workspace.
- Cargo finds a manifest's workspace by walking up. In a nested worktree the walk passes the
  worktree's own root (which excludes the CAD crate) and continues into the enclosing checkout's
  root manifest, which does not list the crate. Cargo then stops with
  `current package believes it's in a workspace when it's not`. In the main checkout nothing
  encloses the repo, so the same commands succeed.
- Reproduced in a scratch tree: the error appears when an outer workspace exists above the nested
  root, even with the CAD crate in the nested root's `exclude`. Adding an empty `[workspace]` to
  the crate's own manifest stops the walk; removing the outer workspace also fixes it.
- `build` failed in the second checkout for a different reason: `cad/.cache` was a symlink into
  the main checkout. `build_wasm` mounts `cad/` alone into the podman container, so the symlink
  dangles there (the OCCT path, the CARGO target and the `wasm-pack` output all go through it).

## Change

- `cad/step-oracle/Cargo.toml` and `cad/wasm/Cargo.toml` declare an empty `[workspace]` table, so
  each is its own root. Root `exclude` and the container's temporary workspace are unchanged.
  Lockfiles are unchanged; `cargo metadata --locked` passes for both.
- `cad/scripts/cadrum_build.py` `build_wasm` raises a clear error, after the up-to-date check and
  before any download or container call, when `cad/.cache`, `cad/wasm/pkg` or `cad/wasm/target` is
  a symlink. Mounting the symlink's target was rejected: a bind mount cannot be placed under a
  dangling symlink in the container, and it would have changed the mount and OCCT paths.
- Tests in `cad/scripts/test-cadrum-build.py` (run by `check.py tooling`):
  - `CadWorkspaceTests.test_cad_crates_declare_their_own_workspace`
  - `CadWorkspaceTests.test_nested_checkout_resolves_cad_crates_without_the_parent_workspace`:
    a temporary outer workspace, a nested root with the repo's real `exclude` list, and the real
    CAD manifests, then `cargo metadata --no-deps --locked`. Red before the fix with the error
    above (2 failures).
  - `CadrumBuildTests.test_symlinked_cache_or_output_is_rejected_before_the_container` (red before
    the guard: 3 failures).
  - `CadrumBuildTests.test_current_provider_skips_before_the_symlink_check` (added in verification):
    an up-to-date provider still skips with a symlinked `cad/.cache`, so the guard stays after the
    up-to-date check.
- `AGENTS.md`: the nested-worktree paragraph now says nested worktrees run every step, and points
  here. Historical ticket Outcomes that mention `/tmp/claude-1000/boardstudio-tce-verify` are left as
  recorded.

- Verification also: the guard message now says how to fix it (`rm cad/<name> && cp -a <owner>/cad/<name>
  cad/<name>`); `docs/agents/issue-tracker.md` lists `implemented (awaiting verification)`;
  `docs/agents/verification-process.md` says nested worktrees run every step and records the
  `wasm-pack` overlap error; `README.md` notes that the CAD crates are their own roots.

## Outcome

### Implementation (Haiku 5.5; unverified until the section below)

Run in the nested worktree `.worktrees/tce-matrix`
(branch `typed-core-edits/mechanical`), with the sandbox lifted only for `build`, which uses podman.

- `python3 scripts/check.py tooling`: exit 0 (3.4 s). `cad/scripts/test-cadrum-build.py` ran 20
  tests, all passing, including the three new ones.
- `python3 scripts/check.py test`: exit 0 (93.3 s). `cad/scripts/test-cadrum.py` built the
  step-oracle and ran the `cad/wasm` tests; the log shows 53 `test result: ok` blocks and no failure.
  No second checkout was used.
- `python3 scripts/check.py build`: first attempts failed at the first `wasm-pack` step
  (`Installing wasm-bindgen ... invalid type: sequence, expected a string`), 12 s in, before any CAD
  step. The first of these overlapped with another background launch of the same command (two
  builds shared the target and the wasm-pack cache), and its output was lost. An unmodified control
  in the main checkout passed the same step, and the same command passed when run alone. The cause
  of the first error is not identified. The run that passed, on its own and with the CAD stamp
  cleared so the container runs: `build` exit 0 (251.1 s). `build-cadrum-wasm.py` pulled the
  published `containerfile-` image from ghcr, `wasm-pack` built the provider inside the container
  with the new manifests (3m15s), and `cad/.cache/cadrum/wasm-build.sha256` was rewritten.
- `python3 scripts/check.py quick --files cad/scripts/cadrum_build.py cad/scripts/test-cadrum-build.py`:
  exit 0 (`repo`, `tooling`). A manifest edit was not run through `quick`: a root-level or unplaced
  path falls back to every Rust check, which includes the browser tests, and that was not run.
- GitNexus `detect_changes`, scope `all`: risk `low`, 0 affected processes, not partial or
  truncated. The index reports 16693 symbols against the 16965 in AGENTS.md, so the index is probably
  stale; the changed symbols match the edited files.

Not verified by the implementer: `check.py` (full), the `browser` step, and `release` (not run, as
instructed). The `wasm-pack` install error above had no confirmed root cause (see below).

### Verification (Sonnet 5.5, high effort)

Everything below was run or read by the verifier in `.worktrees/tce-matrix` (nested), the sandbox
lifted only for the full check. The implementer's report was not relied on.

- **Root cause confirmed, fix minimal.** In a scratch nested layout (outer workspace, nested root,
  a stub crate under `cad/`), `cargo metadata` fails with `current package believes it's in a
  workspace when it's not` unless the crate has its own `[workspace]`, whether or not the nested
  root lists it in `exclude` (4 combinations tried). So `exclude` no longer matters to cargo's
  discovery; it is kept because the container's temporary root (pinned by
  `test_container_supplies_isolated_workspace_for_contract_lints`) and the root `Cargo.toml` comment
  still name it, and removing it would touch the shared root manifest for no gain. Removal is an
  optional later cleanup, not a defect.
- **Resolution unchanged.** Unpacked `HEAD~1` and `HEAD` into scratch trees and compared
  `cargo tree -e features,normal,build,dev --locked --offline` for `cad/wasm` (targets
  `wasm32-unknown-unknown` and `x86_64-unknown-linux-gnu`) and `cad/step-oracle`: identical output
  for all three. `cargo metadata --no-deps` reports the same `workspace_root` (the crate itself)
  and `target_directory` (`<crate>/target`) before and after, so the container's
  `CARGO_TARGET_DIR` and the `step-oracle`/`wasm` target paths are unaffected. Neither `Cargo.lock`
  changed in `04b9bb081`.
- **Other invokers.** Every `--manifest-path`, `cad/wasm` and `cad/step-oracle` use was checked:
  `cadrum_build.py` (`cd cad`, manifests `step-oracle/` and `wasm/`), `.github/workflows/check.yaml`
  (rust-cache `workspaces` list and cache keys, which hash `cadrum_build.py` and `cad/wasm/Cargo.lock`),
  `cad-image.yaml` (tag from the Containerfile only), `scripts/security-audit.py` (reads
  `cad/wasm/Cargo.lock`), `deny.toml`, the Containerfile. None depend on the crates being
  discovered through the root workspace. `wasm_inputs_digest` hashes `cad/wasm/Cargo.toml`, so the
  manifest edit correctly invalidated the stamp and the full check re-ran the container build.
- **Symlink guard.** It runs after the up-to-date check and before `prepare`, the runtime lookup
  and the image step; `Path.is_symlink()` is false for a real directory or a missing path, so
  there is no false positive for those. One imprecision remains: a relative symlink that stays
  inside `cad/` would be rejected although the container could resolve it; judged acceptable
  because none of these paths is a symlink in a correct checkout.
- **Mutation checks** (revert one fix, run `python3 cad/scripts/test-cadrum-build.py`, restore):
  no `[workspace]` in `cad/step-oracle`: 2 failures (declaration test and nested-resolution test);
  none in `cad/wasm`: 2 failures; guard disabled: 3 failures (one per path); guard moved before the
  up-to-date check: the added skip test errors. Restored each time; 21 tests pass.
- **The `wasm-pack` error is an overlap artifact, not flakiness.** Two `wasm-pack build` runs of the
  `core-worker` provider started 0-0.9 s apart with the same `--out-dir` failed with
  `invalid type: sequence, expected a string at line 6 column 11` (the `files` array of a
  `package.json`) in 4 of 4 trials: the loser exited 1, the other 0. Run one after the other into the
  same directory, or concurrently into different directories, every run passed, and a single run
  over an existing `package.json` passed. The wasm-pack cache (`~/.cache/.wasm-pack`) holds intact
  binaries and no JSON, so it is not cache corruption. The implementer's two overlapping `build`s
  share `web/target/providers/*`, which is exactly this case. Nothing in `check.py` runs two
  `wasm-pack build`s at once (the parallel launches are `wasm-pack test`, which has no `--out-dir`).
  Not traced into wasm-pack's source; the exact read of the other process's `package.json` is
  inferred from the line and column, which match the generated manifest.
- **Full check, sequential and in the foreground** (`python3 scripts/check.py`, exit 0, 480.1 s):
  repo 0.1 s, tooling 2.7 s, lint 5.4 s, build 236.4 s, test 76.1 s, browser 159.5 s. `build` ran
  the CAD container (`wasm-pack` 3m16s inside podman) because this change edits the stamp inputs.
  `test` ran `cad/scripts/test-cadrum.py` natively (51 passed, 4 ignored in `boardstudio-cadrum-wasm`).
  The native `cargo test` blocks sum to 1865 passed and 0 failed over 67 result blocks; the
  browser runner reported `executed 70, failed 0`. No second checkout was used.
- **Not fixed, outside this ticket.** The wasm test builds print compiler warnings that native
  `lint` does not see (unused `pair` in `web/crates/catalogue/src/physical_setup.rs:8`; unused
  `Severity` in `web/crates/case/src/case_generation_admission.rs:8`; unused `construction_source`
  at `mechanical_settings.rs:1272`; `NumberRule` private-in-public at `mechanical_settings.rs:285`
  and `:346`; unused `Result`s at `:6102`, `:6121`, `:6138`; unused `EditCommand`/`EditPhase` in
  `cad_presentation.rs:273`; unused `SnapshotToken` in `mechanical_settings_controller.rs:804`;
  needless `mut` in `case_viewer.rs:1917`, `:2478`, `parts/preview.rs:1121`, `mechanical_settings.rs:5310`).
  They do not fail the check; they belong to the Core mechanical-edits work on this branch.
- GitNexus: `impact build_wasm` (upstream) risk MEDIUM, 9 callers, all tests and the build script;
  the verification edit only changed an error string. `detect-changes` is recorded in the commit
  step; the index is stale relative to this branch.
