# 14: Outline web adapter and linked/fixed presentation

Status: resolved
Type: build
Blocked by: 13
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Contract: [03 Answer](03-decide-outline-version-intents.md#answer)

## Goal

Replace the outline web domain planner with an adapter submitting `ApplyOutlineEdit`
against accepted state. Keep drafts, selection/focus, lifecycle admission, preparation
ordering and the existing pending-edit ticket protocol. Core owns identities, polygon
validation, version mutation, linked resolution and no-op/retirement decisions.

Make Generated, linked refinements and edited outlines visible as distinct ownership
choices. Accepting an alternative creates a linked refinement. Expose Freeze/manual
perimeter editing as creation of a separate fixed copy, retaining the linked version.
Settings remain editable while linked; show located findings and their affected export
state. Do not introduce a second writable recipe, link-status or findings store.

Capture relevant reviewed target/source identities and fingerprints with the pending
edit. Resolve using the accepted document at execution. Active-target/source changes
retire with an explanation; unrelated edits survive. FocusGap/hover/navigation remains
presentation-only and must not create document history.

## Ownership and seams

`web/crates/layout/src/outline/{planner,hook,inspector,overlays}` and outline settings
adapters, plus bounded presentation call sites. Read DESIGN.md/PRODUCT.md and preserve
unrelated UI work. Do not edit shared Core or transcript files without their owner.
Mounted controls and controller/resolver ports through real Session/Core are the seams.

## TDD cases

1. Reachable version/settings/feature/connection controls submit typed intents, not
   ReplaceDocument. Core returns unchanged/error/retire through existing settlement.
2. Mounted acceptance/Freeze/manual-edit controls show resulting ownership and submit
   one action; one Undo restores both selection and document state.
3. A pending edit on A retires after activation of B. A reviewed source-shape change
   retires; unrelated names/other-board edits and ordinary queued field edits survive.
4. Linked controls remain available after source changes; fixed copies do not move.
   Hover/focus affects neither accepted document nor Undo.
5. Reopen displays persisted linked/unlinked ownership accurately; legacy fixed
   versions are not silently presented as linked.
6. Malformed submission exposes a Core error; geometric findings remain selectable,
   saving/editing is possible, and only affected exports are disabled.

## Verification

Run affected native adapter tests, WASM page typecheck, mounted Layout browser tests
and format/lint. Take the coordinated Chrome lease; enum construction or native checks
alone do not prove presentation delivery. Keep a concise real-product gesture pass for
15; do not remove old rule tests until equivalent public Core coverage exists.

## Outcome

Resolved 2026-10-09 at integrated `9fac9a14c` (lane `typed-core-edits/outline-web-adapter`,
commits `5b746939b`, `d652449ea`, `bf8cb72fa`, the approved `--no-ff` import of Core
`069bc5f0c` at `ba694988a`, `31b9e2960`). The web planner is gone: one
`OutlineAction` maps to exactly one typed `ApplyOutlineEdit` (target, named change,
captured seed, target-specific `reviewed` source shape, adapter-supplied bridge width)
through `outline/adapter.rs` (`outline_edit`, `admission`, `settle`); Core's
`evaluate` decides Submit/Unchanged/Retire/Err, retirement stays explained for Core
`Retire` per ADR-0005's amendment, and no adapter re-derives identity, validation or
retirement rules. Generate creates the Generated envelope through the typed
`CreateGenerated` change — `web/crates/layout/src/outline*` has zero `ReplaceDocument`
consumers. Ownership is presented distinctly (Generated / Linked refinement / Edited
outline from Core's persisted `OutlineOwnership`), with the linked refinement's located
problem (finding-id contract + FindingMarker, never message parsing), affected blocked
exports, saving/editing availability, unselected broken alternatives listed, one-shot
Freeze retaining the link with one Undo restoring document and selection, and reopen
displaying persisted ownership with legacy fixed versions never shown as linked.
Hover/focus create no document history. Checks: native layout 63, mounted layout
browser suite 143, WASM typecheck, fmt/clippy, core 459 isolated, application 30;
GitNexus gates with the `create_generated` CRITICAL reported (Core's symbol, exercised
by its own and this lane's tests).

Fix round after 15's verification (lane commits `d3cfdd7cf`, `6f9b3e2e7`, merged at
`75a761e34`): 15 found two acceptance gaps. (1) Linked refinements had no product entry
point (test-only), so the phase's headline ADR-0002 behavior was unreachable; a
**Refine outline** control now maps to the typed `OutlineChange::CreateLinkedRefinement`
(created and selected, one Undo removes it), with Copy product-context mounted coverage
(mounted layout browser 145, native layout 65, workspace 1137 including boundary parity).
(2) The adapter never reissues a retired outline version identity. 15's "Copy outline
silently produced no version" had no code defect: the cause was the stale core-worker wasm
provider asset served by `scripts/dev-web.py`, which rebuilds providers only when a file is
missing, never when stale; with providers rebuilt from source Copy lands live (live gesture
evidence: reports second pass, screenshots 20-31). Not re-driven live: a Generated-active
Copy click (covered mounted plus one live linked-context click), and the
material-bridge-with-authored-connection gesture (no bundled demo has an authored
connection; Core-level bridge coverage stands).

Accepted residuals: the gap check/uncheck UI is covered natively (no mounted fixture
scene produces repair candidates) and the check-direction alias narrowing is
unobserved by web; operation-kind assertions are native + zero-ReplaceDocument
counts; hover/focus Undo depth uses revision equality as proxy; parallel-workspace
boundary_parity interference reproduces (isolated runs pass — a shared-target
artifact, documented at 15). Reports:
`/tmp/tce-outline-reports/14-outline-web-adapter.md`.
