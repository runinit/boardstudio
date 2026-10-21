# 185: Unify switch-family lookup

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Priority: Highest; approved opportunity 6

## What to build

Switch-family inference and the assigned-versus-inferred lookup each exist once, so profile selection and compatibility diagnostics cannot drift apart. Plate spacing and saved-project behavior do not change.

## Simplification targets

- Family inference from a part definition (MX generator source; Choc v1/v2 flags with a part-level override before the definition parameter) exists in three places: `inferred_switch_family` (mechanical resolution), `definition_family` (settings edits) and `infer_switch_family` (the web profile editor, definition-only). Reduce to one implementation.
- `resolve_profiles` repeats "the profile assigned to this definition, else the inferred family" four times (profile creation, resolved family, incompatible-switch filter, assigned-versus-inferred mismatch) and the `profiles.iter().find(definition_id)` lookup about six times. Replace with one assigned-family lookup and one effective-family lookup.
- The legacy MX stabilizer recognition (profile gap of exactly 5.0 with one of the two reviewed source fixtures and no family) becomes a named, separately tested predicate instead of an inline condition.
- `initial_switch_family` in the settings edits uses its own "is this a switch" predicate (kind, inferable family, or Choc generator source). Decide whether it can share the resolution-side predicate; if the two differ in behavior, keep both and record why.
- Do not add a dependency between crates to share the web copy. Reuse the core implementation only if the web crate already depends on core; otherwise leave it and record that.

## Acceptance criteria

- [ ] One family-inference function remains in core, used by resolution and by settings edits; the definition-only caller passes no part.
- [ ] `resolve_profiles` has one assigned-family helper and one effective-family helper and no other copy of the lookup.
- [ ] Preserved and recorded: definition-owned profile precedence, assigned-versus-inferred mismatch diagnostics, mixed mounting-datum rejection, unknown-family behavior, profile-mutation order, and plate-to-PCB spacing including custom-profile exceptions.
- [ ] Preserved and recorded: the three inference copies differ in two small ways (ASCII versus Unicode lowercasing of the generator source; the web copy ignores part-level parameters). Characterize both, pick no new behavior, and note the outcome.
- [ ] Tests cover explicit and inferred families, a Choc part-level override, legacy stabilizers (matching and non-matching source or gap), incompatible assignments, and unchanged generated plate geometry.
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

2026-10-21, implementation started: characterization tests added in `core/src/mechanical.rs` (inference table with part overrides and both value shapes; legacy stabilizer match and non-match; assigned-versus-inferred, definition-owned precedence, unknown family, mixed datum). All pass on the unchanged code (`.evidence/185-baseline-core.log`).

Blocked before the refactor edits: after the first Gortex write the exact worktree view stayed at `view_building` (reads fell back to the base graph, `exact:false`, line numbers from another commit). The ticket requires exact-view Gortex edits, so no production code was changed. Remaining: the three-copy inference merge, the `resolve_profiles` lookup helpers, the named legacy-stabilizer predicate, the web copy and datum table (`mounting_datum` and `default_plate_thickness` are already public in core, and the web crate already depends on core), plus characterization of the settings-edit and web paths. Resume once the view is exact.
