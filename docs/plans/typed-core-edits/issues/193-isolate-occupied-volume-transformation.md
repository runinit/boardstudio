# 193: Isolate occupied-volume transformation

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Priority: Opportunity 8

## What to build

The nominal-fit check builds each part's occupied volumes in one function, leaving overlap detection and reporting as a short loop over them.

## Simplification targets

- `nominal_fit` validates each clearance volume (at least three points, finite coordinates, positive finite height), transforms its points (back side mirrors x before rotation, then rotation and translation) and its z range (surface volumes flip on the back side using the board thickness), then pairs volumes for overlap.
- Extract the validity predicate and the world-space construction. Search for an existing placement transform with identical math (for example the one applied to profile openings in mechanical resolution) and reuse it; if the math or edge handling differs, keep a local one and record the difference.
- While there, tidy only what the extraction exposes: the marker lookup by finding ID (merging contours into an existing finding) and the single-line `Finding` literal.
- Leave the overlap rule, the finding message text and the ID format alone.

## Acceptance criteria

- [ ] `nominal_fit` reads as: build volumes per board, then compare pairs.
- [ ] Preserved: rotation, back-side placement, invalid-volume skipping, the same-part and no-surface-volume exclusions, vertical-overlap test, finding IDs and messages, and marker contours.
- [ ] Tests cover rotated, back-side and invalid volumes, surface versus non-surface pairs, and a pair producing several contours merged into one marker.
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
