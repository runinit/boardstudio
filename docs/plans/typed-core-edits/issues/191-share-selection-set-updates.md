# 191: Share selection-set updates

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Priority: Opportunity 5

## What to build

Ordinary and matrix selection apply replace, add and toggle through one routine, while range and anchor semantics stay in their own handlers.

## Simplification targets

- `SelectParts` filters the incoming IDs against the document's parts, then applies replace, add (append if absent) and toggle (remove if present, else append) inline; the matrix-cell selection repeats the add and toggle logic. Locate the matrix copy first and confirm it matches before sharing.
- Extract one function taking the selection list, the incoming IDs and the mode. Range expansion, anchor updates and scope validation remain separate per selection kind.
- Do not widen the shared function to cover range handling.

## Acceptance criteria

- [ ] One selection-update routine serves both selection kinds; each handler is shorter.
- [ ] Preserved and recorded: ordering of the selection list, behavior for duplicate IDs inside one toggle request, filtering of unknown IDs, anchor handling, and operation settlement.
- [ ] Tests cover replace, add, toggle (including a repeated ID), unknown IDs, and range plus anchor behavior for both kinds.
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
