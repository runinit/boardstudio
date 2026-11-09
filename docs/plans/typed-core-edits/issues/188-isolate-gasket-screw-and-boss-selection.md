# 188: Isolate gasket screw and boss selection

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Priority: Opportunity 2

## What to build

Choosing screw length, insert engagement and boss depth is a pure calculation that can be tested without generating geometry, taking the roughly 900-line `generate` toward a smaller orchestration.

## Simplification targets

- `generate` mixes hardware selection (screw lengths against the length datum, insert engagement, boss height and diameter) with placement and body emission. First inventory exactly which statements compute the selection and what they read and write.
- Extract only the part that takes inputs and returns a result. If the selection is entangled with placement through mutated state, stop at the narrower slice that is separable and record the rest.
- Do not duplicate or move the existing gasket `validate`; the extraction sits beside it.

## Acceptance criteria

- [ ] The selection is a pure function of the closure hardware settings and stack dimensions, with a result type or tuple of at most a few fields.
- [ ] Preserved and recorded: every tolerance and rounding, and the generated geometry (compare before and after on the existing gasket fixtures and the CAD volume baselines).
- [ ] Boundary tests cover the lengths on either side of each selection threshold, engagement at its limits, and a case with no fitting screw.
- [ ] No physical-fit claim is made from software tests.
- [ ] The quality bar below is met and the net production-line change is reported.

## Quality bar

These are refactors: simpler code, identical behavior.

- Characterize first. Add or confirm tests that pass on the unchanged code before the first edit, then keep them green unchanged. A test edited to fit the refactor does not count as preservation.
- Net deletion: production lines must not grow beyond what the removed duplication saves. A new helper needs at least two callers, or removes a branch or parameter at its one caller.
- No new public surface: new items stay private (or `pub(crate)` if a sibling module needs them), with no new dependency, trait, generic, macro or error type unless the ticket names it.
- Pre-existing quirks are preserved and recorded, not fixed in passing. Note each one in the ticket's Comments; a behaviour change is its own ticket.
- Stop condition: if the extraction needs a long parameter list, shared mutable state, or a flag that selects behavior, it is the wrong seam. Take the narrower slice, or record a no-change verdict with the reason.
- Verification: focused suites, `python3 scripts/check.py` repo/test/lint (clippy clean), and `scripts/mutation-check.py` on the touched logic to show the characterization tests fail when the behavior is broken. Logs and mutation results go in the ignored `.evidence/` directory.

## Execution requirements

Use native Gortex MCP, not CLI or shell navigation fallbacks. Refresh the relevant glossary and ADRs, start with task exploration, and resolve every target with search. Before source mutation run change impact; verify each proposed signature change. Use Gortex edit/refactor tools for all writes and the gortex-refactor workflow's single appropriate refactor operation where applicable. After mutation run change detect, then tests, guards and contract against the returned symbol IDs; resolve every violation. If the exact checkout view or required tools are unavailable, stop and report the blocker.

Add behavior-preservation tests for the bounded slice, run the focused suites and the repository/native/lint checks required by current project guidance, and record commands, outcomes and limitations. Put long logs and mutation evidence in the project's ignored evidence directory. Do not claim physical qualification from software tests.

## Comments

Approved by the user on 2026-10-21. Tickets are independently startable; priority is scheduling, not a blocking edge. Coordinate overlapping source edits rather than inventing prerequisites.

Revised 2026-10-21 after a Gortex pass over each target's current source: the simplification targets below name the duplication actually found. Re-verify them against the code before starting; if one no longer holds, record that and shrink the ticket rather than inventing a replacement.
