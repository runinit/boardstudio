# Matrix and Keycaps root integration RF handoff

Integration tip: `a896cd493ab31a9894c740ca131379409fc49735` on `codex/rust-v1-ui-parity-20261001`.
Feature mount commits: Matrix `1ad8c478d05305d6c7b6f1a2a74fd8c52963ff87`; Keycaps `052351be6e7498af85a5246bfd4f4e23150fb537`.
Also merged the independently cleared Keymap expected-red fix `deba7087e6fc2c6761b18e37f0c6884fa3fff764` and the docs-only six-stream reconciliation `c79163f121941e9905f2a55f70911d145227ba1c`. Review pointers remain in `/tmp/frontend-parity-reset-20261002/layout/source-spec-review.md`, `/tmp/frontend-parity-reset-20261002/layout/source-standards-review.md`, and Keycaps `implementation-handoff.md`, `source-spec-review.md`, `source-standards-review.md`.

## RF-001 — shared composition and Runtime ownership

Matrix source is registered privately in `objects`; its hook is called unconditionally at the shared Editor lifetime with the current Runtime, version signal, selected tree context, workspace signal and SelectionAdapter generation. Only the current Layout matrix projection mounts the real selected-matrix Inspector. Its existing adapter/controller owns fresh scope/context admission and SetMatrix/SetLayout operations, history, and exact feedback. No new domain or selection owner was introduced. Matrix source is Standards and Spec clear; root compilation and accepted-result/Undo/Redo/public gates remain open.

The Keycaps selection controls now live in the actual shared Inspector frame, and the key select uses the existing guarded root `keycaps_select` callback. This is a bounded routing/composition mitigation under RF-001; the earlier mount review was insufficient when it checked valid props without confirming the visible control placement. No new state authority or public API was introduced.

## RF-005 — physical projection and canonical selection

The Keycaps scene is registered and memoized from one token-checked AcceptedSnapshot, exact current Scope and active canonical board. The new root callback validates live workspace, Scope, board/instance and SelectionAdapter generation, reprojects against the current accepted snapshot, resolves the existing object context and uses `selection::submit_canvas_selection` for both canvas/list selection. Empty list selection clears canonical selection. The selected summary reads the same projection; the feature module remains read-only and has no Runtime access. Keycaps adds no new authority and no new RF ID; existing RF-005 is the correct owner.

## RF-009 — bounded visible parity and evidence limits

Keycaps mounts the reviewed physical canvas in the center and places the searchable key select, read-only matrix list and selected summary together in the right Inspector, matching the React `KeycapPanel.tsx` ordering/placement and the saved baseline screenshot `composition/baseline/react-keycaps.png`. The key-list search signal now lives in the stable Inspector subtree without a selection-dependent component key; the paired rerender behavior still needs browser confirmation. Root callback routing stays canonical and guarded. It uses accepted board contours with the exact board-entry rule (an explicit empty entry stays empty; the legacy contour fallback applies only when the entry is absent and the document has one board), and camera bounds union physical key rectangles with contour vertices. CSS selection/focus rules are scoped beneath `.m1-keycaps-layout`; list/summary styles are scoped beneath Keycaps classes, and obsolete center controls-grid rules are removed. The fresh shared archive browser journey, selected-key remount green after root rebuild, keyboard/AT checks, and F3.1/F6C/parent acceptance joins are still open; this source mount is not acceptance evidence.

The first integrated browser capture also showed that MatrixInspector's connected name/dimension/pitch controls were unstyled and crowded together, despite the edit successfully persisting. The mount review checked the input wiring but did not require a rendered Inspector style check. Commit `a896cd49` adds scoped MatrixInspector layout, spacing, heading, field, focus, feedback and millimeter-unit rules using existing theme tokens. Root's rebuild and visual recheck remain pending. RF-009 should retain this integration lesson: verify the actual rendered public control surface and its stylesheet, in addition to source props/callback connectivity.

## Checks and state

`git diff --check` passed before the placement commit `4f7b8aa9` and Matrix styling commit `a896cd49`. No Cargo/build/browser commands were run by the merger; root owns compilation and the next green run. All existing untracked closure/viewport files remain intact. Matrix and Keycaps slice authors/reviewers observed no new refactoring takeaway; existing RF IDs/status stay owned by the central ledger. Parts and Case source branches were not merged in this wave.
