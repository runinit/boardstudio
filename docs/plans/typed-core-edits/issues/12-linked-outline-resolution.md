# 12: Linked outline resolution, source cleanup and scoped findings

Status: ready-for-agent
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
