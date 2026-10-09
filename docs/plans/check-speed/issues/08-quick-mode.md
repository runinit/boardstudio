# 08: `check.py quick`, the routine loop

Status: ready-for-agent
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

Commits, the mapping table, the three timings, and fallbacks that fired.
