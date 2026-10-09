# 05: Generator parameter lint test and conventions

Status: ready-for-agent
Type: build
Blocked by: 04
Map: [map.md](../map.md)

## Request

Parameter vocabulary is inconsistent (`hole_drill`/`hole_size`, `drill`/`outline`,
`via_drill`/`via_size`, Case `hole_diameter`/`boss_diameter`) and nothing documents how to
declare one. Renaming and new integer/enum kinds are out of scope (see the map).

## What to do

- Add a `footprints/tests/` lint over every registered generator: unique names, snake_case,
  each default accepted by its kind, `side` declared with default `F`, and no new numeric
  parameter declared as `string` unless it is on an allow-list (the allow-list documents
  existing names, one comment per entry).
- Add a Core test: every Core writer of generator parameters (closure projection,
  `electrical_*`, `mechanical/inputs.rs` mirror `side`) produces values the 04 check
  accepts.
- Docs: a "Generator parameters" section in `docs/hardware/component-onboarding.md`
  (declare explicit kinds, millimetre numbers, no numeric strings, a migration for any
  type change, existing vocabulary listed as legacy). In `CONTEXT.md`, extend
  **Generator parameters**: values are typed by the declaration; Core rejects wrong types
  and undeclared names on edit.

## Acceptance

- [ ] The lint fails on a deliberately mistyped default (shown in the Outcome).
- [ ] The docs pass `scripts/check-doc-links.py`.

## Verification

`quick`, `repo`. No ADR. Risk: the allow-list can grow quietly.
