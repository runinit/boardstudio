# 014: Full check is green locally

Status: ready-for-agent
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

- [ ] The rustfmt cause is recorded under `## Comments` (local and pinned versions, CI's behaviour).
- [ ] `cargo fmt --all --check` passes, or the ticket records why it should not be changed here.
- [ ] The unrecorded test is named and either recorded in the baseline or explained.

## Comments
