# 20: Merge the CAD library and wasm workspaces

Status: ready-for-agent
Type: build
Blocked by: [19](19-overlap-steps.md)
Parent: [map](../map.md)

## Evidence

CAD is 47% of real CPU (2112 of 4472 s at `62a274623`). Host library crates appear in up to three trees:
`monstertruck_io` 85, 86 and 87 s, `boardstudio_cad` 83 and 84 s. A library crate compiled in more than one
tree costs at most 553 CPU-s beyond its largest copy. Cargo hashes path crates relative to the workspace root,
so sharing a target dir cannot help; they must share a workspace. `cad/wasm` depends on `cad/monstertruck`
with matching features. `monstertruck-qualification` has different features (`monstertruck_modeling` 31 s
against 18 s) and most of the 18 lockfile version conflicts, so it stays separate here.

## Design

- `cad/monstertruck/Cargo.toml`: `[workspace] members = ["../wasm"]`. Its `[patch]` stays the only one.
- `cad/wasm/Cargo.toml`: drop `[workspace]` and `[patch]`; add `package.workspace = "../monstertruck"`.
  Remove `cad/wasm/Cargo.lock`, and regenerate `cad/monstertruck/Cargo.lock` with the smallest version movement.
- One target dir for both, and one `cargo test --release` selecting both packages.
- Update everything that names the removed lock or target: `cad/scripts/cad_build.py` (lanes,
  `NATIVE_TEST_TARGETS`, the CAD wasm stamp's inputs), `scripts/security-audit.py` and its test, the
  `check.yaml` rust-cache list, `scripts/seed-worktree-target.py`, `scripts/build-time/cold.py`, `scripts/check.py`
  preflight and their tests, and the README's CAD note. Find them all with `git grep`.

Keep the `wasm-pack build wasm` build and its output paths as they are.

## Gain

| Cold wall (estimate) | CPU | Warm |
|---:|---:|---|
| about 0 locally, more on 4 vCPU | -300 to -350 s | same |

## Verification

The lock diff is read in review: list every package whose version moved. `cargo tree` comparison before and
after for wasm32 and host. CAD native tests and CAD browser checks pass with unchanged counts. Compare the CAD
wasm's byte size with the previous build. One full cold run from the Claude shell; `real CPU` from the kit is
the measure, since wall barely moves.

## Stop rule

Revert if real CPU falls under 150 s, or if lock movement changes any non-patch version CAD geometry depends on.
Reopen merging qualification only if a `cargo tree -e features` comparison shows its crates would be shared.
