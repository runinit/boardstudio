# 189: Separate mechanical validation from construction

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Priority: Opportunity 3

## What to build

Mechanical resolution validates configuration and contours as one stage before construction, so the roughly 520-line `resolve` reads as validate, then build.

## Simplification targets

- `resolve` interleaves validation and diagnostics with assembly construction on shared mutable state. Inventory the checks that depend only on the document, configuration and contours.
- Extract those into one validation stage that returns the diagnostics and a stop signal. Construction receives its result; it does not re-derive anything.
- Reuse the existing diagnostic and finding types. No new error type. Leave the internal gasket `validate` and `resolve_profiles` alone (the latter is ticket 185).

## Acceptance criteria

- [ ] `resolve` shows validation, a single early-return decision, then construction.
- [ ] Preserved: diagnostic IDs, messages and ordering, early-return behavior, and generation-blocking (a blocked configuration still yields no geometry).
- [ ] Tests cover each diagnostic, the blocked-versus-generated outcomes, and ordering when several diagnostics fire together.
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
