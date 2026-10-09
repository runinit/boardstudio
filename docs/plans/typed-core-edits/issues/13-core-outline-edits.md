# 13: Typed Core outline edits and atomic version lifecycle

Status: ready-for-agent
Type: build
Blocked by: 12
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Contract: [03 Answer](03-decide-outline-version-intents.md#answer)

## Goal

Add one `ApplyOutlineEdit` operation with a typed target/change vocabulary. Core owns
settings, feature/connection/gap changes, linked-refinement creation, Freeze/manual
perimeter copies, activation, naming, copy and deletion. Move domain planning/validation
behind this interface rather than preserve a web replacement-document planner.
Focus remains presentation. Composite copy/edit/activate actions stay one operation,
one accepted revision and one Undo step; do not add a generic batch interface.

The existing Session resolver asks Core for Changed/Unchanged/Retire through shared
read-only evaluation rules. No prepared replacement document or public cache is
returned. Direct commits evaluate against operation-time state through the same private
implementation. Equal field edits create no history/revision step; do not change the
no-op policy of unrelated operations.

Core allocates identities from accepted state using the captured creation seed. Report
only owners/entities whose durable values or memberships actually changed, including
removals and source cleanup. Geometry/input/activation changes refresh dependent
outlines/findings; metadata-only naming preserves geometry caches. Document the
classification and retain current behavior of existing callers unless the Answer
explicitly changes it.

## Pending targets and errors

- A geometry edit for version A retires when the active outline changed to B; never
  redirect it. Preserve action-specific activation/name/lifecycle eligibility.
- A reviewed generated/source shape edit retires when relevant source geometry changed.
  Use relevant identities/input fingerprints, not whole-document revision equality.
- Vanished/ineligible targets retire. Too few/nonfinite points fail with a Core error
  and no mutation; self-intersection remains a located finding, not blanket admission.
- Freeze/manual perimeter editing creates a fixed edited copy and retains the linked
  refinement. Removing an active named version returns to Generated as today.

## Ownership and seams

One writer for Core operation types, `lib.rs` dispatch/cache/history hooks and the shared
outline implementation. Coordinate model/geometry changes with 11/12 and mechanical.
Public CoreEngine Edit/Preview/Scene/Undo/Redo and read-only evaluation are the seams;
private helpers are not additional interfaces. Use `Result<_, String>` conventions.

## TDD cases

1. Each named settings/feature/connection/gap/lifecycle action runs through the typed
   operation with precise changed owners and no replacement document.
2. Creating a linked refinement and selecting it, and copy/edit/activate, each Undo
   atomically. Freeze retains the original link and produces a fixed selected copy.
3. Direct and resolver no-ops preserve history/revision/redo; malformed input rolls
   back completely; source/target retirement never changes another version.
4. A relevant generated-shape change retires an earlier reviewed edit; unrelated
   metadata and other-board edits do not. Two ordinary field edits still queue in order.
5. New IDs avoid persisted collisions; deleting/recreating a feature cannot redirect a
   pending edit. Copies do not share mutable choices through origin provenance.
6. Geometry-affecting edits refresh accepted contours/findings; renaming does not
   rebuild geometry. Undo/Redo returns both ownership and geometry exactly.

## Verification

Run Core/application tests, format/lint, affected WASM typecheck and graph gates. Keep
wire-fixture edits for the coordinated ticket 15 owner. Source operation presence alone
is not acceptance; record per-behavior red/green evidence and remaining integration work.
