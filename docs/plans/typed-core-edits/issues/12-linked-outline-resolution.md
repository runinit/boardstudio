# 12: Linked outline resolution, source cleanup and scoped findings

Status: resolved
Type: build
Blocked by: 11
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Contract: [03 Answer](03-decide-outline-version-intents.md#answer)

## Goal

Resolve every linked refinement from its board's current included components and its
own settings/choices. Follow placement, rotation, geometry, inclusion, additions and
removals. Do not turn source changes into an implicit Freeze or unlink. Fixed edited
outlines retain their geometry and receive clearance findings instead.

Source removal cleans only authored connections, protected gaps and cutouts requiring
that source. Independent choices survive. Recalculate bridges from the remaining
components and that version's settings; do not silently retarget a chosen endpoint.
Cleanup belongs to the same accepted edit/Undo as source removal, not a second edit or
a mutation performed only by a view adapter.

Use this Core resolution for accepted scenes, previews, checks and exports. Recompute
inactive linked alternatives too, but their findings cannot block output from a valid
active outline or an unrelated board. Keep the link active when resolution fails;
show located findings and a clearly identified last-valid display if a current shape
cannot be rendered. Do not manufacture the fallback as a valid current result, block
saving/editing, or stop retrying after later source changes.

## Ownership and seams

Core outline/geometry resolution, scene/readiness and source-change settlement. Shared
`lib.rs`/geometry hooks require a lease coordinated with mechanical and ticket 13.
Tests observe `CoreEngine` accepted documents/scenes and selected output gates, not
private resolver helpers. Adapters must not reproduce this rule engine.

## TDD cases

1. Asymmetric source movement, rotation and definition geometry changes update linked
   boundaries/bridges while an edited alternative remains fixed.
2. Adding/removing/inclusion-changing components uses the live included set. Normal
   removal with valid remaining inputs recalculates instead of displaying stale shape.
3. Removing a named endpoint removes only dependent connections/gaps/cutouts; other
   choices remain. One Undo restores the source, choices and resolved geometry together.
4. Changing one refinement's margin/corner/connection settings updates that refinement
   only and preserves its link and other versions' choices.
5. A geometric problem stays editable with a precise board/version/choice/location
   finding; only dependent exports are blocked. Saving and a later repair remain valid.
6. Inactive/unrelated findings do not block valid selected output. A failed linked
   resolution followed by a repair resumes current geometry without relinking.
7. Existing edge-clearance, required pad/drill support and permitted-overhang rules
   retain ADR-0001's blocker/advisory meanings. Protecting a gap cannot waive support.

## Acceptance / verification

Use independently worked asymmetric expectations, not expectations computed by the
resolver being tested. Run affected Core/application/CAD checks and WASM typecheck.
Record changed source hooks, cache dependencies, output scopes and red/green evidence.
Do not close on a standalone projection-helper test without accepted-state/export proof.

## Outcome

Resolved 2026-10-09 at integrated `08bbc3975` (lane `typed-core-edits/outline-resolution`,
commits `ff2c99420`, `979e579c3`, `8bba73280`). Core owns one linked-resolution rule: linked
refinements rebuild from the board's current included components with their own settings,
authored connections, protected gaps, cutouts and margin (placement, rotation, geometry,
inclusion, additions and removals all follow; fixed edited outlines keep persisted geometry and
receive findings). `RemoveParts` prunes only choices naming a removed source inside the same
accepted edit/Undo. A failed resolution keeps the link, emits one located
`board:{board}:outline:linked:{version}` finding (Error, Scope Pcb) with a FindingMarker and
composes the last persisted projection as the clearly identified last-valid display; saving,
editing and later repair continue. Inactive alternatives are recomputed at source level and
their findings cannot block the active outline or another board. Linked active versions derive
the expected region count from the current structure like the Generated version (a refinement
accepted before a split stays valid after it). ADR-0001's blocker/advisory meanings and the
protected-gap support rule are unchanged. Checks: core 268+443 with hand-worked asymmetric
expectations, application 30, fmt/clippy, WASM typecheck; GitNexus gates low per commit with the
`board_composition` CRITICAL impact pre-reported and contained to Core's composition path.

Accepted deviations: the last-valid display after a failure is the last persisted projection
(not a cached previous successful resolution); inactive alternatives' compose-level failures
surface only when selected; linked alternatives add one feature-geometry pass per version per
`outlines` call (unmeasured); the pre-existing duplicate-findings pattern is unchanged. The
`ApplyOutlineEdit` handoff for ticket 13 (creation seeding, version-local settings authority,
identity namespacing, retirement, freeze, finding-id contract) is recorded above and in the
tests in `core/tests/outline_linked_resolution.rs` (the scratch report was not retained).

Amended at the outline phase's close: cleanup originally ran only in `RemoveParts`. The
Spec review found matrix key deletion detached linked choices to world points and other
removal paths left dangling anchors; Core now prunes linked choices on every part removal
inside the same accepted edit (`prune_removed_components`, and in matrix deletion's
`remap_parts`), pinned by matrix-key-delete and remove-matrix tests. The "last valid"
display is the version's persisted projection from creation or the last prune, not a
re-persisted last successful resolve; a failed resolution can show that earlier shape.
