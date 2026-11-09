# 192: Make constraint replacement visibility explicit

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Priority: Opportunity 7

## What to build

The solver passes the replacement view to matrix-companion classification as a parameter, instead of swapping the document's stored constraints and swapping them back.

## Simplification targets

- `resolve_prepared` removes the document's constraints, installs a filtered copy (without the records being replaced), calls `assembly_placements`, then restores them. The filter is the same one used a few lines earlier to compile the stored constraints; compute it once.
- `assembly_placements` reaches the stored constraints through `companion_mode`, which also has callers in mirrored-half reconciliation and scene companion building. Run impact first and choose the smallest change: an exclusion-aware variant used by the solver, with the existing signature unchanged for the other callers, unless every caller can pass the exclusions at no cost.
- Do not change what counts as a constraint, the claim and target ordering, or the postcondition validation.

## Acceptance criteria

- [ ] No code path mutates and restores the document's constraints during resolution; `resolve_prepared` no longer needs to own the swap.
- [ ] Preserved: companion classification results for records being replaced, mirrored-half preparation, and unchanged stored constraints after both success and error returns.
- [ ] Tests cover a replaced constraint that would change a companion's mode, no replacement, and a constraint error leaving the stored constraints untouched.
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
