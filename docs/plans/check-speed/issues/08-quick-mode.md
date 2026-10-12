# 08: `check.py quick`, the routine loop

Status: resolved
Type: build
Blocked by: 03
Map: [map.md](../map.md)

## Goal

One everyday command that checks what changed and nothing else (decision 4 in
[01](01-destination-and-scope.md)). Target: 3 minutes or less for a typical presentation or core
change.

## Scope

- `python3 scripts/check.py quick` takes the changed files (against the merge base with `main`
  plus the working tree, overridable with `--files`) and runs: `repo`, format, lint, typecheck, the
  affected native tests, and the affected browser tests through `run-wasm-tests.py --files`. It
  never runs `build`.
- Mapping rule, written down and tested: a path under a crate maps to that crate and its
  dependents; a shared change (`Cargo.lock`, `Cargo.toml`, `.cargo/config.toml`, `scripts/`)
  falls back to the full affected steps; a path the mapping cannot place also falls back, and is
  named in the output. Edge cases that need judgement go back to the map's *Not yet specified*.
- It prints which steps it chose and why, and the per-step times from
  [02](02-timing-and-count-baseline.md).
- Script tests for the mapping; README documents `quick` as the everyday command and the full
  check as the pre-PR command.

## Verification

Three representative edits (a `core` change, a `web/crates/layout` change, a docs-only change)
each run `quick` within the target and run the tests that cover them; the executed browser tests
for the layout edit are a superset of the layout module's tests.


## Outcome

Resolved 2026-10-09 (commit `12f37299b`). `check.py quick [--files F...]` selects checks from the
changed files (git diff against the merge base with `main`, plus untracked files) through
`scripts/check_quick.py`: docs-only changes run `repo`; scripts add `tooling`; a path under a
workspace crate selects that crate plus its dependents (from `cargo metadata`) for `lint`,
`typecheck`, native `test -p ...` and the matching browser launches; `web/src` and `web/tests`
add the page runner by file; `cad/` adds the CAD native test; `Cargo.toml`, `Cargo.lock`, `.cargo/`
or an unplaceable path fall back to every Rust check and are named. Data such as `web/assets/`
prints a note that `build` covers it. Never runs `build`. Pinned by `scripts/test-check-quick.py`.

Timings (warm): docs-only 0.1 s; a layout edit (`web/crates/layout/src/outline/hook.rs`) 85.7 s;
a core edit (`core/src/outline_edit.rs`) 213.8 s (native test 106.8 s, browser 103.4 s), 34 s
over the 3-minute target because nearly every crate depends on core. No fallback fired in these.

