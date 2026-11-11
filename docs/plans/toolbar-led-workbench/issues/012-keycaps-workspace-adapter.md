# 012: Keycaps adapter with defaults, overrides and fit ownership

Status: ready-for-human
Type: build
Blocked by: [004](004-objects-popover.md), [009](009-shared-toolbar-panel-host.md)
Parent: [map.md](../map.md)
Owner: Keycaps workspace/settings and fit-navigation owners

## Boundary

Use existing Objects tree for navigation through the shared compact host; keep Keycaps'
selection/admission. Subject to the separate M2 decision, move the existing KeycapsBoardSettingsEditor
from below the Objects dock into Inspect; retain matrix-profile disclosure/editor,
selected-key KeyList/settings, resets/inheritance and fit findings/retry/navigation there.
No new appearance model or duplicate board/key editor.

Relocate applicable DesignViewToolbar contextual/2D/3D/Footprints controls into the shared
row. Keep focused-finding, pending-layout-fit and navigation lifetime ownership. Opening
Inspect does not fit; explicit existing finding navigation/Retry can act only for its
captured live board/instance. Audit board/matrix/key reset effects for draft restoration.
M2 gate: classify actual settings field commit modes before retaining them. This crate has
no existing DraftOwnerRegistration; do not assume these are explicit-submit editors. Keep
current editor scopes mounted/inert and hoist only board/matrix/key reset-owned raw state
needed for identity-safe restoration. Record the approved update-safety treatment for any
newly retained unresolved form (including Keycaps/editor/remedy when off screen), using the
existing registry if required. Classification and board-default relocation remain human
phase decisions, not a blocker for Layout M1.

## Acceptance criteria

- [ ] Every [Keycaps inventory row](../inventory.md#keycaps-adapter) retained, especially
      board defaults formerly in Objects and each override/reset level.
- [ ] Ordinary navigation leaves Inspect closed; explicit settings/findings request works.
- [ ] Session drafts survive owner/panel/workspace round trip, no stale fit/navigation.

## Proof map

- Settings parity — board default plus matrix and key override fixture → navigate Objects,
  explicitly Inspect and change/reset each level → same inherited/effective result and
  existing Undo; no duplicate or missing editor · mounted · planned
- Draft/host — unresolved board/matrix/key draft before selection change → close, select
  other owner, switch workspace, return → original text/error/disclosure restored under
  its own identity, camera unchanged from panel opening alone · mounted · planned
- Fit lifetime — finding/Retry queued for A before board/instance switch → select B then
  release A's result/navigation → no B selection/camera change; fresh B request still works · mounted · planned
- View/pick — contextual/View controls and canvas/Objects key picking → operate/pick → same
  preview/selection, no implicit Inspect, no accepted edit from view toggles · mounted · planned

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`, keycaps browser subset
and `python3 scripts/run-wasm-tests.py --files web/src/presentation/keycaps_workspace_tests.rs`.
Use existing settings/fit fixtures; named stale-navigation proof-gap mutation must fail
no-foreign-selection/camera assertions. Coordinator independently reruns the stale path.
