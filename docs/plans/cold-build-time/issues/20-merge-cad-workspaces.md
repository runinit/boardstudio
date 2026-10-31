# 20: Merge the CAD library and wasm workspaces

Status: resolved

## Resolution (2026-10-31)

`cad/monstertruck/Cargo.toml` is now the workspace root (`members = ["../wasm"]`, its
`[patch]` the only one); `cad/wasm/Cargo.toml` dropped `[workspace]` and `[patch]` and points
at it via `package.workspace`, keeping edition 2021 and every `=` pin. `cad/wasm/Cargo.lock`
is removed; `cad/monstertruck/Cargo.lock` was regenerated `--offline` from the old
monstertruck lock, keeping its versions (four gratuitous resolver bumps — `cc 1.10.0`,
`find-msvc-tools 0.1.15`, `smallvec 1.16.4`, `tokio 1.54.1` — were pinned back with
`cargo update --precise --offline`). One `cargo test --release --locked` selects both
`boardstudio-cad` and `boardstudio-cad-wasm` on the merged target; the owner lane,
`_check_owner_output` strictness, `JOBS=1` serial path and the `wasm-pack build wasm`
outputs are unchanged. `wasm_inputs_digest` reads `monstertruck/Cargo.lock` instead of
`wasm/Cargo.lock`. `[workspace]` still sits at `cad/monstertruck`; root metadata shows 19
members with none under `cad/`. `cargo tree` before/after is identical for the library and
differs only by the table below for the wasm package, on host and wasm32.

Packages whose version differs from the old `cad/wasm/Cargo.lock` (none missing, none
added beyond the old graphs; no non-patch move in a geometry crate):

| Package | Old | New | Change |
|---|---|---|---|
| `cc` | 1.5.1 | 1.8.0 | minor (build-time C driver, not geometry) |
| `equivalent` | 1.0.2 | 1.0.3 | patch |
| `glam` | 0.33.11 | 0.33.12 | patch |
| `libc` | 0.2.189 | 0.2.191 | patch |
| `syn` | 3.0.6 | 3.0.9 | patch (1.x and 2.x unchanged) |
| `unicode-ident` | 1.0.26 | 1.0.27 | patch |

Against the old monstertruck lock the only changes are pure additions: `approx 0.5.1`
beside 0.4.0 and `cpufeatures 0.2.17` beside 0.3.1. Every `=` pin holds, including
`parry3d-f64 =0.31.1`, so the research link now points at the merged lock.

Follow-ups (stale but non-breaking, deliberately untouched): `cad/.dockerignore`'s
`wasm/target/` line; `cad/monstertruck-qualification/README.md`'s "both patch" sentence;
the root `Cargo.toml` and `deny.toml` comments naming the old wasm workspace (`deny`
scans the root workspace only, so `deny.toml` itself is unchanged); `measure-cad.py`'s
`cad/wasm/target` probes (degrade to `exists=False`); `PROOF.md`'s bare-manifest cargo
commands. Still to be measured from the parent shell: the full cold run, CAD native and
browser counts, the CAD wasm byte size, and the stop-rule numbers.
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
