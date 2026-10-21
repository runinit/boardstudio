# 186: Consolidate CAD cache eviction

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Priority: Highest; approved opportunity 9 (second in execution order)

## What to build

The CAD worker's caches evict through one shared routine, so a limit or accounting change is made once. Memory bounds, metrics and topology lifetimes do not change.

## Simplification targets

- The same four-condition eviction loop (entry count, retained solids, retained topology, mesh bytes, then pop the oldest and count an eviction) is written out three times: the region cache in `cached_region` (96 entries, 32 MiB) and, verbatim, the preview cache in both `preview_body` and `export_cached_assembly` (64 entries, 64 MiB). Each iteration also computes `retained_complexity` twice.
- The least-recently-used lookup is also repeated: `cached_preview`, the inline preview take-out in `export_cached_assembly`, and the move-to-back lookup in `cached_region`. Share the position lookup only where the hit semantics match (move to back versus take out then re-insert).
- Check the staged upstream-region cache (`upstream_region` with its own entry, solid and topology limits). Fold it in only if it has the same shape; otherwise leave it and say why.
- Express each cache's limits as named constants next to the shared routine; the metric name (`regionCacheEvictions` versus `bodyCacheEvictions`) stays a parameter.

## Acceptance criteria

- [ ] One eviction routine, parameterized by limits and eviction counter, replaces the three inline loops.
- [ ] Preserved and recorded: eviction order, limits, metric names and values, topology release timing, and the mesh-memory measure, which counts position floats only and not normals. Do not correct that measure here.
- [ ] Computing complexity once per iteration is allowed only because it is a pure function of the same state; show the existing long-edit topology test and the eviction tests still pass unchanged.
- [ ] Tests cover entry-count, solid, topology and mesh-memory eviction for both caches, the metric values after eviction, and preview ownership surviving region eviction.
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
