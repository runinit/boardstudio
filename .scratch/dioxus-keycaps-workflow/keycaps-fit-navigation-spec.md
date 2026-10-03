# F6C.4 specification — Keycaps fit finding navigation

## Outcome

From the Keycaps Inspector, a designer can follow each actionable current fit finding to the same affected object or outline the pinned React workbench selects. The existing Keycaps resolver remains the source of finding identity, targets, messages and severity. The existing accepted workspace/navigation owner remains the only selection, workbench and camera authority.

## User stories

1. As a keyboard designer, I want each fit finding with a resolvable target to offer the same action as React (`Show outline` or `Select affected geometry`), so I can move from the Keycaps warning to the object that needs attention.
2. As a keyboard designer, I want findings whose targets no longer exist to remain readable without navigating to stale geometry.
3. As a keyboard designer, I want navigation from an earlier retained assessment to identify its stale revision and avoid presenting it as a current result.

## Source-backed behavior

- The pinned React `FindingList` uses `findingTarget(finding, document, assembly)`. It shows an action when the target has a label. Outline targets use `Show outline`; other actionable targets use `Select affected geometry`.
- `Workbench.showFinding` is the authority for board routing, Design versus Case, component/matrix/body selection, outline inspection, camera fit, focused-finding state and Inspector focus. Keycaps must call this existing authority using the currently accepted finding/document pair; it must not implement a parallel resolver or selection state.
- `KeycapPanel` supplies Keycaps resolver findings filtered from scene findings. The source-stamped Dioxus `KeycapsFitInspector` already owns presentation of the `ResolveKeycaps` assessment, but `InspectorInput` has no find-target callback. Root `WorkspaceCallbackSlots` supplies tree navigation and `on_show_configured_board`; neither implements React `Workbench.showFinding` for target selection, outline reveal, camera fit and focused Inspector state.
- Existing grouped/deduplicated presentation can combine equivalent Core feature findings. Preserve the presented finding identity and all target IDs when routing.
- Safe suppression of an action after its retained result's target is removed is a correctness guard for Dioxus's explicit stale-result retention; React derives action visibility from the current document and may already have cleared or replaced that result. This guard prevents a stale retained finding from selecting an unrelated object and does not claim literal React stale-state parity.

## Requirements

- [ ] Render the React-equivalent action label only if the finding has a resolvable target in the current accepted document: `Show outline` for an outline target; `Select affected geometry` for other targets.
- [ ] Add the minimal private Keycaps-to-root finding event needed at the existing composition seam. The coordinator-owned root handles accepted project/scope identity, workbench/board routing, target selection, outline reveal, camera fit, focused-finding state and Inspector focus. Cover a part/key, matrix, board outline, case body/feature, and cross-board target where such targets are emitted by `ResolveKeycaps`. Do not create new Core geometry math, public API, persisted state, target resolver, camera, or selection owner.
- [ ] Resolve the action target from the same accepted document snapshot and accepted finding result used for display. Revalidate the root's live project/board/scope identity before applying navigation. A result that has become stale while the UI is open must not select unrelated or deleted geometry; it leaves the finding visible with its existing stale status and safely declines navigation.
- [ ] Keep targetless and unresolved findings readable; do not render a button that cannot navigate. Preserve stable order, severity, target grouping, labels, deduplication and all target IDs from the existing React `FindingList` oracle.
- [ ] Preserve current/pending/error/retry and stale-case messages. A navigation click must not mark an old accepted result current, clear a pending failure, or mutate the project document.
- [ ] Keep the Keycaps Inspector's contextual grouping, labels and keyboard-accessible controls consistent with React. Do not alter the F6C.2 settings operation owner.

## Capability-level start gate

Start after the canonical F6C.2 start prerequisite for F6C.4, with the already implemented accepted `ResolveKeycaps` result and presented finding groups proven against the accepted document snapshot. The root finding handler is part of this ticket's private coordinator integration; it is a known missing capability, not an assumed existing callback. Keep root-owned `presentation.rs` wiring serial with the coordinator and the Keycaps Inspector/callsite feature-owned. This capability gate does not waive F6C.2, F6C.4, INT.2, or any task graph parent/acceptance criteria. If a target kind cannot be represented without changing public interfaces, stop and report that exact boundary for coordinator resolution.

## Acceptance journey

Use the pinned React commit and the source-stamped integrated Dioxus build in isolated named browser profiles. Import the same byte-identical supported fixture and record its SHA-256, embedded project ID, app/build provenance and browser profile. In each app, open Keycaps, locate a finding with a target, activate its action with pointer and keyboard, then verify the resulting workbench, active board, outline/part/matrix/body selection, camera framing, focused finding and Inspector focus. Compare the displayed action label and target label. Repeat with a stale/deleted target and verify safe no-navigation while retaining the stale message. Confirm no document revision/history change from navigation; then exercise the existing fitting edit, Undo/Redo, save/reopen path required by F6C.4.

Browser acceptance remains paired and source-stamped. Component/native tests support it but do not substitute for the journey. Keep F6C.4 and its `INT.2` acceptance join open until the full parent journey passes.

## Not included

This child does not complete F6C.2 settings acceptance, F6C.3 Layout overlap/resize acceptance, F6C.5 3D/STEP integration, F7 viewer acceptance, or F6.6 shared Keymap/Keycaps integration. It does not authorize public API/schema/member visibility changes or broad shared-shell refactoring.

## Focused marker continuation

Issue [07 — Show the focused Keycaps finding marker in Layout](issues/07-keycaps-focused-finding-marker.md) and its [focused-marker specification](keycaps-focused-finding-marker-spec.md) refine the existing focused-finding acceptance without changing this F6C.4 contract. The child may start once issue05's current accepted navigation owner and delayed-focus source/target guards have independent source clearance; it does not wait for the full issue05 paired journey or close F6C.4/INT.2. It consumes current Core scene markers and preserves Core geometry ownership.

## Refactor record

Carry RF-001 and RF-005 from the Keycaps implementation scope. No new RF is proposed: the missing action is a required parity behavior and the remedy is to reuse the existing accepted navigation owner. If implementation reveals that the root callback cannot represent one target kind or has a second selection owner, record the concrete seam evidence before proposing structural cleanup.
