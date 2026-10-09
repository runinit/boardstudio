# 02: Timing and test-count baseline

Status: ready-for-agent
Type: build
Blocked by: 01
Map: [map.md](../map.md)

## Goal

Make every later ticket measurable. `scripts/check.py` prints per-command wall time and a final
per-step summary, and the executed-test counts of today's full check are recorded as the baseline
that coverage parity is judged against.

## Scope

- Print the wall time of each command and a per-step total at the end of `check.py` (plain text,
  no new dependency). Keep the existing output and exit codes; `scripts/test-check.py` must keep
  passing and gains a case for the timing summary.
- Record, in this ticket's Outcome, the baseline for the **current** full check on the pre-change
  tip: per-step and per-command times, and the executed counts.

Known before this ticket (2026-10-09, tip `77b9548b8`; confirm, do not re-derive):

| Step | Time |
| --- | --- |
| `lint` + `typecheck` | 25 s |
| `build` + `browser` | 979 s |
| `test` (verify worktree) | 123 s |

Counts: browser crate suites 8/29/5/26+1/30/16/40/12/81/65/49/146+1, 60 page tests, 15 CAD
browser checks; native core 271 + 463, CAD 51 passed / 4 ignored.

## Verification

`python3 scripts/check.py repo tooling` passes, the timing summary prints, and one full baseline
run is recorded. The `build` and `browser` steps need the sandbox lifted.

## Outcome

Record commits, the baseline table with counts, and anything the timing exposed.
