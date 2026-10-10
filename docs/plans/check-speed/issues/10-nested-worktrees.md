# 10: Nested worktrees run every check

Status: implemented (awaiting verification)
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
- `AGENTS.md`: the nested-worktree paragraph now says nested worktrees run every step, and points
  here. Historical ticket Outcomes that mention `/tmp/claude-1000/boardstudio-tce-verify` are left as
  recorded.

Not changed: `docs/agents/verification-process.md` (user-owned).

## Outcome

Status: implemented (awaiting verification). Run in the nested worktree `.worktrees/tce-matrix`
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

Not verified: `check.py` (full), the `browser` step, and `release` (not run, as instructed). The
`wasm-pack` install error above has no confirmed root cause.
