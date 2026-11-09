# 11: Core outline ownership, identity and persistence

Status: resolved
Type: build
Blocked by: 03
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Contract: [03 Answer](03-decide-outline-version-intents.md#answer)

## Goal

Represent generated outlines, linked refinements and fixed edited outlines without
using a copied snapshot or origin provenance as implicit link authority. Persist
ownership, version-local settings/choices and the component references those choices
require. A refinement follows its board's current included components; its resolved
shape is a projection, not another independently writable recipe.

Core allocates stable version/feature/connection identities against accepted state,
with a captured creation seed when needed. Prevent collisions across persisted
namespaces and redirection of pending edits after removal/recreation. Copies retain
origin information without creating a live dependency between versions.

Existing projects retain geometry, active selection, IDs and effective behavior.
Preserve explicit linked/unlinked ownership on reopen; absence of new link state must
not opt an existing fixed snapshot into linkage. Keep Generated available. If a format
change is necessary, append the next ordered migration under ADR-0004 after the actual
integrated mechanical schema version; do not reserve conflicting version numbers.

## Ownership and seams

- One coordinated owner for `core/src/model.rs`, `core/src/migrate.rs` and schema-literal
  adaptations. Coordinate with mechanical 02/08 before editing shared files.
- Public `CoreEngine` Open/Scene and archive save/load, including browser storage
  load/writeback. Verify accepted ownership/geometry, not private serialization helpers.
- Do not widen private outline/cache helpers or introduce an unrelated error hierarchy.

## TDD cases

Use one behavioral red -> green slice at a time; the seams are the confirmed Core,
archive and browser persistence interfaces.

1. Reopen a legacy project with Generated and multiple fixed versions: geometry,
   selection, identities and absence of inferred links are unchanged.
2. Save/reopen explicit linked and edited ownership, independent settings, connections,
   gaps and source references. Editing one version does not alter another's choices.
3. A created entity cannot collide with another persisted namespace; a removed ID cannot
   redirect an earlier pending edit to a later entity.
4. Origin provenance does not make an edited copy follow its source version/components.
5. Native archive migration and browser storage writeback produce the same ownership and
   geometry. A new project writes the integrated current format version.

## Acceptance / verification

Record schema changes, migration/version order and red/green evidence. Run Core and
application tests, affected persistence browser tests, repo links and WASM typecheck.
Keep the ticket open until these behaviors pass; source type additions alone are not
acceptance. Linked resolution and typed creation actions follow in 12/13.

## Outcome

Resolved 2026-10-09 at integrated `4ab266e64` (lane `typed-core-edits/outline-ownership`,
commits `9a7d29a1d`, `6b6543cdd`, `562ede9db`). `OutlineOwnership { Fixed (default), Linked }`
persists per outline version with absence-equals-fixed serde defaults (byte-identical legacy
documents, `CURRENT_VERSION` stays 3, no migration added); origin provenance
(`source.version_id`) never grants link authority and copies are always constructed Fixed.
Version-local settings/features/protected-gap choices persist and survive reopen
independently. `BoardOutline.retired_version_ids` reserves removed version identities;
`CopyOutline` refuses retired identities and any collision across persisted outline
namespaces; Core-authored derived identities are version-namespaced so one reservation
covers features/connections/gaps. Native archive pack/unpack and the browser-persistence
migrate/writeback boundary agree on ownership, reservation, provenance, geometry and
identities; legacy reopen changes nothing and invents no links. Checks: core 268+432,
workspace, footprints, fmt/clippy, WASM typecheck, host (16) and layout (138) browser
suites; GitNexus gates low per commit (`OutlineVersion` HIGH and `state_mut` CRITICAL
impacts were pre-reported and contained to Core's outline-version module). Ticket 13's
allocation entry point is documented as a handoff (`identity_namespace` as the single
collision source of truth, retired identities never reissued, freeze/create paths set
ownership explicitly); linked resolution stays ticket 12's.
