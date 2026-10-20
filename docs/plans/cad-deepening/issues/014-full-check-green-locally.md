# 014: Full check is green locally

Status: resolved
Type: task
Blocked by: none
Parent: [CAD deepening](../map.md)

## What to build

Two failures in `scripts/check.py` predate this plan and hide real ones:

- `cargo fmt --all --check` reports 13 diffs under `footprints/`, from `dev`.
- The `test` step reports one executed native test missing from the baseline.

For formatting, find out first whether the local rustfmt differs from the pinned toolchain and
CI. Reformat only if CI's rustfmt agrees; otherwise record the cause and stop. For the
baseline, name the test and record it if it should run.

## Acceptance criteria

- [x] The rustfmt cause is recorded under `## Comments` (local and pinned versions, CI's behaviour).
- [x] `cargo fmt --all --check` passes, or the ticket records why it should not be changed here.
- [x] The unrecorded test is named and either recorded in the baseline or explained.

## Comments

- rustfmt: local `rustfmt 1.9.0-stable (88d9e12ae1 2026-08-18)` and `cargo 1.98.0`, which is
  the toolchain `rust-toolchain.toml` pins (channel 1.98.0, with rustfmt). There is no
  `rustfmt.toml`. CI (`.github/workflows/check.yaml`, job `native`) runs `check.py repo tooling
  lint test` under that same pin, and the lint step runs `cargo fmt --all --check`. So CI's
  rustfmt would report the same 13 diffs in 11 files, all under `footprints/`. The cause is
  unformatted code from `dev` (import order and line wrapping in recent footprint generator
  commits), not a toolchain mismatch. Outcome: ran `cargo fmt -p boardstudio-footprints`; the
  commit touches only `footprints/`, and `cargo fmt --all --check` now passes.
- Baseline: the unrecorded test was `polygon::tests::vertices_follow_the_same_rule`
  (`boardstudio_contracts`). It is a legitimate test. `check.py test --update-native-baseline`
  added only that name (2096 to 2097 executed). Log: `.evidence/014-test.log`.
