# F6C.1 Keycaps projection and 2D selection reconciliation — 2026-10-03

This bounded reconciliation uses the pinned React implementation, current accepted Rust source, existing projection/settings/empty-state receipts, and one paired public keyboard-selection journey. It preserves F6C.1's complete acceptance language and does not change task status.

## Paired journey

- React reference: `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34770/boardstudio/`, source `47623521afaf96fb54690e2eaf873c1f0e7d2db5`, provenance SHA-256 `a4677af663c0e8286ae5e923bf1884b0230bcc6dc852a3030cc35a68d453365c` (root-reported).
- Both imported `.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`, at 1280×577 with Left PCB selected.
- In each app, opened Keycaps, pointer-clicked `left-keys-SW1`, focused `left-keys-SW2`, and pressed Enter. The selected-key control resolved to `left-keys-SW2 · Unassigned` in both apps. This exercises the real canvas pointer and keyboard event paths; no page errors were reported.
- Captures: [React](react-keyboard-selection.png), [Dioxus entry](dioxus-entry-34770.png), and [Dioxus after selection](dioxus-keyboard-selection-34770.png). The entry capture also preserves the desktop layout on this candidate; no desktop scrim appeared during the route.

## Criterion mapping

| F6C.1 clause | Evidence and disposition |
| --- | --- |
| Same board-scoped supported keys and matrix list as React | Pinned React `createKeymapWorkspace.tsx` filters supported switches and top-level matrix members against the active board. Rust `keycaps_scene::project` applies the same board membership, switch-kind, and matrix-prefix filters. Its existing `accepted_board_projection_preserves_membership_pose_and_display_fallbacks` fixture covers a standalone switch alongside valid and malformed matrix members, and excludes off-board/unsupported parts. The paired archive exposes the same 24 left `keys` switches and 5 left `thumbs` switches. No source-backed list mismatch remains. |
| Stable selected-key identity and keyboard/pointer selection | Both canvases emit the stable part ID and use the shared selection owner. React `KeymapLayout` and Rust `KeycapsCanvas` both provide button semantics and Enter/Space selection. The paired click-SW1/focus-SW2/Enter route resolves SW2 in each app. |
| Effective dimensions, color, inherited or explicit legend | Both projections use per-key settings over board color and use key-specific units over part/definition dimensions, with the same 18.2 mm fallback. Legend resolves explicit saved text (including blank) before the base binding label. Existing F6C.2 color/profile/legend receipts and F6C.3 linked-size receipt exercise the public values and visible Keycaps state; the 34757 assembly receipt shows generated cap/legend output from the accepted specs. No F6C.1 projection defect is identified. |
| Physical Keycaps view stays distinct from Keymap's active-layer labels | React exposes separate `canvas` and `keycapsCanvas` projections; the latter renders physical legend/color/dimensions. Rust composes separate Keymap and Keycaps canvas leaves, and `keycaps_scene::KeycapsCanvas` renders the physical view. This follows source equivalence and the retained physical Keycaps 2D/3D receipts. |
| Empty input, standalone switch, mixed matrix, and per-key overrides without materializing defaults | Empty project guidance is qualified by the retained 34751 empty-state receipt. The Rust mixed standalone-switch/matrix projection test covers membership and filtering, and `keycaps_projection_supplies_editable_defaults_without_materializing_config` confirms defaults do not create persisted keycap configuration. Existing settings/legend receipts exercise explicit per-key overrides. A new single-purpose fixture/browser permutation is not warranted by the current source/evidence. |

## Joins and limits

`INT.1` is accepted in the local task record. Root reports F3.1 accepted in the current batch; the local `tasks.json` still has its historical `implementing` status and retains F6C.1's exact `acceptance_after: ["F3.1"]`. No task status or canonical parent criterion was changed here. The current paired route covers one real pointer-plus-keyboard selection sequence, not all keys, boards, focus routes, or assistive-technology behavior. No additional acceptance join is inferred beyond the canonical F3.1 edge.

The owner-local module names have moved since the issue map was written: the current Rust read projection and physical canvas are in `web/src/presentation/keycaps_scene.rs`, while workspace selection composition is in `web/src/presentation/keycaps_workspace.rs`. Existing issue map lines are corrected to those current owners. No production change was justified by the source comparison and paired route.

RF handoff: no new refactoring takeaway observed in this F6C.1 comparison; retain the existing RF-001 architectural hotspot record and all other RF entries unchanged.
