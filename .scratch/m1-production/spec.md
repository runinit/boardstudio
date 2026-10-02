# Remaining M1 production workflow

**Status:** candidate implementation under final story completion; acceptance open. **Category:** behavior-preserving vertical migration, with the accepted save-failure recovery behavior from ADR 0003.
**Triage:** ready-for-agent

**Authority:** user request of 2026-10-01 to plan the remaining M1 documentation with to-spec and then implement. Existing six module specifications, capability map, constraints and accepted P1/P2/P3 evidence remain authoritative. This specification bounds M1 only.

## Problem Statement

Editors still depend on the React/TypeScript application coordinator to open a saved keyboard, edit it safely, recover a failed save and generate/export its case. Accepted worker, renderer and storage probes establish feasibility but do not deliver this complete workflow. The continuation also predates newer encoder/VIK providers, so adopting it unchanged could lose supported data and capabilities.

## Solution

Deliver an isolated static Dioxus editor with Rust session and browser policy. Reconcile the exact newer provider reference first. Open faithful copies of REVIUNG41 and Sofle, edit with preview and one-step history, durably save and recover, archive/reload, change case settings, generate exact case geometry in workers, inspect it with the existing renderer and export committed STEP. Retain the working React reference and its project database until later cutover approval.

## User Stories

1. As an editor, I want to open a copied REVIUNG41 project, so that I can continue its saved layout.
2. As an editor, I want to open a copied Sofle project, so that I can work with split-board cases.
3. As an editor, I want existing component, encoder, VIK, extension and asset data preserved, so that saving does not remove unrelated capabilities.
4. As an editor, I want project discovery and active-project restoration, so that I can resume saved work.
5. As an editor, I want part selection and board/instance navigation, so that I edit the intended target.
6. As an editor, I want pointer previews without storage writes, so that I can inspect a move before committing.
7. As an editor, I want the final pointer sample committed once, so that one Undo reverses one gesture.
8. As an editor, I want Escape and pointer cancellation to restore accepted geometry, so that abandoned edits do not enter history.
9. As an editor, I want modifiers, snapping and Alt behavior preserved, so that movement remains familiar.
10. As an editor, I want numeric preview and commit, so that I can place components accurately.
11. As an editor, I want Undo and Redo durably saved, so that reload reflects accepted history actions.
12. As an editor, I want a save failure clearly reported, so that I know which snapshot is durable.
13. As an editor, I want failed-save retry to retain the committed edit and history, so that recovery does not duplicate changes.
14. As an editor, I want dependent mutations and exports blocked during recovery, so that unsaved state is never presented as accepted.
15. As an editor, I want close and project switching to resolve pending saves, so that work is not silently discarded.
16. As an editor, I want archive export and import with original asset bytes, so that I can exchange files with React.
17. As an editor, I want invalid imports to preserve the active project, so that a bad file cannot destroy my session.
18. As an editor, I want reload and cached offline reopen, so that saved work remains usable without a network.
19. As an editor, I want explicit unavailable-asset errors, so that incomplete offline caches do not pretend to work.
20. As an editor, I want case settings committed through normal history, so that generated cases match my saved choices.
21. As an editor, I want exact generation off the page thread, so that editing stays responsive.
22. As an editor, I want stale and cancelled generation separated from current exact geometry, so that I cannot export a misleading preview.
23. As an editor, I want worker failures to settle operations, so that I can recover from visible errors.
24. As an editor, I want 3D inspection with existing camera and materials, so that I can evaluate the generated case.
25. As an editor, I want STEP built from the accepted committed snapshot with an independent export cache, so that exports are manufacturing geometry.
26. As an editor, I want project/board changes to prevent stale downloads, so that I never receive an artifact from another scope.
27. As a keyboard user, I want named controls, visible focus and equivalent navigation, so that the editor remains accessible.
28. As an editor on a compact display, I want required controls reachable, so that viewport size does not block the workflow.
29. As an editor, I want root and subpath hosting to work identically, so that static deployment paths do not change behavior.
30. As a maintainer, I want reproducible provider assets and exact-candidate reviews, so that passing results can be traced to the integrated source.
31. As a maintainer, I want worker, canvas, URL and storage resources released on teardown, so that remounts and cancelled imports remain bounded.
32. As a maintainer, I want wide integer revisions preserved or explicitly blocked at an incompatible provider boundary, so that no operation silently targets a rounded revision.

## Implementation Decisions

- Reconcile the six committed provider changes from the exact main-checkout dev revision; preserve unrelated scratch work and both stashes. Resolve overlapping Rust lint/enum-boxing edits by retaining accepted contracts and new provider behavior.
- Add the proposed headless application crate and a separate web application package. The application emits typed effects and consumes identified completions; web hosts execute effects. Neither Dioxus signals nor transport snapshots become a second writable document authority.
- Keep existing public CoreEngine, artifact/archive, CAD and renderer interfaces. New consumer-owned interfaces belong to the application package. Existing API visibility, durable schema and wire-format changes require explicit approval.
- Use session/executor epochs, operation IDs, save-attempt IDs and immutable accepted-snapshot tokens outside persisted provider payloads. Match expected reply variants and IDs; settle every caller once.
- Publish mutation/open/history results after transaction completion. Retain failed commits for save-only retry, block queued dependent intents and never replay an uncertain engine mutation.
- Reuse the accepted probe decisions through reviewed adoption into maintained modules; production must not import scratch packages. Document every browser/generated/upstream boundary and its ownership, cancellation, copies and retirement condition.
- Preserve Dioxus/CLI 0.7.10 and exact binding pins, existing styles, static hosting, isolated copied storage and one writer. Keep expensive work in real workers and preview/export caches separate.
- Maintain exact kernel, readiness, geometry, cache-budget and artifact semantics. Record the CAD wide-revision limitation; no silent float rounding or unsupported full-range compatibility claim.
- Restore supported data faithfully without evaluating saved generator bodies. Broad generator authoring and workspace parity remain later capabilities.

## Testing Decisions

- Seams inherited from the approved six module specifications: public Rust session events/effects/read models with a real CoreEngine; actual Dioxus/browser interactions on REVIUNG41 and Sofle paired with the React reference. Use existing provider archive/STEP/geometry interfaces as oracles. Optional user feedback may add acceptance boundaries; it does not reopen the already approved public session and real-browser seams.
- Characterize observable reference behavior first. Red-green tests cover each delivered session behavior rather than private functions or mirrored implementation calculations.
- Native session cases cover durable ordering, abort/retry, one-step history, final sample/cancellation, reopen at equal ID/revision, stale completions, executor failures and close.
- Real Chromium checks cover workers, IndexedDB abort/retry, byte hashes and Uint8Array storage, reference archive exchange, root/subpath release assets, cached/cold offline, invalid import and service-worker update separation.
- Startup restoration is verified through the saved-project library, scoped active preference and actual reload at each prefix, including absent/stale preferences, slow worker startup and a newer explicit open superseding a delayed saved read. Physical navigation uses canonical and valid board-filtered instances; public session effects and browser case disposal verify cancellation and stale-scope suppression.
- Case/STEP comparisons use independent cached/uncached provider lifetimes, prepared-input/readiness parity and reopened geometry/material/bounds oracles. Pixels alone cannot prove STEP correctness.
- Run affected fmt, strict native/WASM Clippy, locked tests/builds, contracts, repository/boundary checks and reference suites. Preserve all existing assertions and frozen budgets. Record failed/unperformed gates honestly.
- Final acceptance includes paired desktop/compact visual interaction evidence, keyboard/focus and relevant screen-reader checks, raw axe/contrast, resource lifecycle and existing applicable performance checks. Measurements without an approved comparator are observations.
- Fresh independent Standards and Spec reviews apply to exact integrated candidates; changed candidates receive affected verification and renewed review.

## Out of Scope

Production database handoff, push, deployment, main merge, React retirement, full application parity, native/server hosting, wider browser support, new hardware qualification, firmware/PCB workflow migration, dynamic generator authoring, kernel substitution and unrelated performance work.

## Further Notes

P1/P2/P3 acceptance is retained as scoped feasibility evidence. M1 is complete only when the entire copied-project workflow and applicable integration gates pass. Existing historical run entries stay intact; a new bounded run ledger tracks implementation and blockers. Missing approval for an existing public API change or new gate/budget blocks only dependent work.
