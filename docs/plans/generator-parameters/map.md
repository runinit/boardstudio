# Map: Typed generator parameters

Label: wayfinder:map
Source: research and plan of 2026-10-09, requested while resolving
[typed-core-edits 04](../typed-core-edits/issues/04-refresh-included-keyboards.md)

## Destination

Generator parameters are typed end to end. Core never writes a wrongly typed value, saved
projects that hold the bad values are repaired by a versioned migration, Core rejects
mistyped or undeclared parameters on edit, and the conventions are documented and linted.
Option B of the 2026-10-09 research; renaming the parameter vocabulary or adding integer
and enum kinds (option C) is out of scope.

## Notes

- Decision basis: [ADR-0003](../../adr/0003-rust-footprint-generators-in-core.md) and
  [ADR-0004](../../adr/0004-document-format-version-and-id-rename.md).
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- GitNexus `impact` before each symbol edit and `detect-changes` before each commit; the
  index is stale, so say so. Native Core tests run in a non-nested checkout.
- Never run `check.py release`. Do not merge to `dev` without the user.

## Decisions so far

- Repair saved strings with a v3 to v4 migration, not per-load coercion: ADR-0004 rejects
  an unversioned rewrite on each load, and `migrate_json` does not detect change at the
  current version. Detail in [03](issues/03-migrate-saved-generator-strings.md).
- Unknown parameter names are warnings on Open and errors on edit, so real user projects
  are not refused: [04](issues/04-validate-generator-parameters-in-core.md).

## Frontier

1. [01](issues/01-closure-holes-via-generator-normalization.md) (ready)
2. [02](issues/02-restore-closure-mounts-in-recipes.md) after 01. It must land before
   keyboard 04's reviews so they review final data.
3. [03](issues/03-migrate-saved-generator-strings.md) after 01
4. [04](issues/04-validate-generator-parameters-in-core.md) after 03
5. [05](issues/05-parameter-lint-and-conventions.md) after 04

## Progress

## Not yet specified

- Core building the mounting-hole template itself and removing the `mountingHole` wire
  field (the page still loads and sends it with each mechanical edit).
- Closure holes are de-duplicated only against each other, not against authored mounting
  holes (Sofle's measured holes); closure positions can land on switches. Surfaces in 02.

## Out of scope

- Renaming the parameter vocabulary (`hole_drill`/`hole_size`, `drill`/`outline`,
  `via_drill`/`via_size`, Case `hole_diameter`/`boss_diameter`) and adding integer or
  enum parameter kinds: breaks saved documents and golden schemas, and diverges from
  upstream (ADR-0003).
