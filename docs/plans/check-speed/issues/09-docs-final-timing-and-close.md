# 09: Docs, final timing and closing the map

Status: ready-for-agent
Type: build
Blocked by: 05, 06, 07, 08
Map: [map.md](../map.md)

## Goal

Everything is documented in one place, the destination is measured, and the map is closed.

## Scope

- README and `AGENTS.md` describe the routine loop (`quick`), the full pre-PR check, the opt-in
  `release` step, the test environment for debug info, the parallel cap, and the outcome of the
  `touch` experiment. Remove or correct the stale `CARGO_BUILD_JOBS=2` and `touch` guidance in live
  docs only.
- Run the full check and `quick` on a clean tip; record times against the targets (3 and 6
  minutes) and the [02](02-timing-and-count-baseline.md) baseline, with executed counts.
- If a target is missed, name the dominant remaining cost and either add a ticket for it or record
  why it is accepted.
- Close the map: Progress lines, final Decisions so far, and the *Not yet specified* patches
  graduated, dropped or kept with a reason.

## Verification

`python3 scripts/check.py` (the new full check) passes; documentation links check passes; the
counts match the baseline.

## Outcome

Commits, final timings versus targets, and the disposition of every option from the original
list.
