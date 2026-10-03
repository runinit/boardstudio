# Case11 live-preview implementation review

Reviewer: Sol 6.1 High, `/root/sol_case_battery_review`; 2026-10-02. Requested model/effort retained; no separate Fast/provider setting changed.

Frozen source `590e1eac56430b0ad6dea42edc11df9d142a4bc3`, isolated worktree `/home/chris/.local/share/boardstudio/worktrees/case11-live-preview-implementation-20261002`; clean at inspection. Reviewed `git diff b6871c7..590e1eac -- web/src web/assets/m1.css` plus implementation handoff. Approved contract `b71dc8fce37add45117365d8dd84fbf0785c8480`, final planning review SHA-256 `cdc39ac4b459aa31cd022273dbc03bbdaf6a4d5ce945cd7c5acff912441d1a72`. Handoff SHA-256 independently matches `56fd1f8da219e0124d7b538adb01f3aad398d308e24c4358b9ab1fe806f87fc9`. No source or integration checkout edits by reviewer.

## Standards

**CHANGES REQUIRED; 1 P2 finding.** `web/src/runtime.rs:1480–1486` relaxes token/revision ownership for any stored scene with a matching fingerprint, without requiring completed exact output. `generate_current` publishes a preview scene (`exact: false`) before its Exact request finishes. Rebinding that preview to a newer accepted snapshot crosses the boundary that CONSTRAINTS requires preserving for captured-source/currentness and that the accepted plan limits to completed output. Require completed owned output before fingerprint relaxation; keep unfinished/preview data tied to its captured token/revision. Add a regression through the production rebind seam that rejects a non-exact scene and accepts a completed same-owner equivalent-input scene.

The rest of the private ownership direction is sound: the app-root switch persists across Case remount, manual/automatic requests go through the existing Session event, scope/token are rechecked before automatic dispatch, and no public API/schema/storage/history owner or worker is introduced. Fingerprint field selection matches the pinned React physical-input dependencies, with effective document/scene reflection applied by existing Rust adapters. No tooling-enforced formatting/lint smell is counted as a review finding. No new distinct RF takeaway; retain RF-003's TS/Rust adapter duplication and RF-001 operation/currentness ownership.

## Spec

**CHANGES REQUIRED; 3 P2 findings.**

1. **Completed-only reuse is not enforced** at `runtime.rs:1480–1486`. The plan states “Pending jobs always require exact captured token/revision/currentness.” Both Preview and Exact publications receive fingerprints at `runtime.rs:2958`, but the getter can rebind either. Restrict and test reuse as described above.

2. **Previous generated rows/Inspector remain missing after physical invalidation.** The plan requires retaining same-owner generated-layer rows/selection as visibly stale during a blocked edit. `case_workspace.rs:152–156` admits Objects' mechanical assembly only when its token is current. `mechanical_settings_mount.rs:296–301` applies the same filter before `project_scene_rows`, and `381–383` clears projected selection when those rows disappear. Changed physical inputs correctly prevent fingerprint rebinding, leaving an old token, so the previous rows/selected Inspector still vanish exactly as the retained Case12 defect describes. Add a separate same-owner previous display projection with truthful stale status. Preserve strictly current token guards for diagnostics, closure initialization and mutation dispatch; do not globally remove token checks.

3. **Temporary ineligibility clears terminal-attempt protection.** `case_generation_lifecycle.rs:217–220` clears `attempted` whenever `eligible` becomes false. The same accepted owner can become temporarily ineligible during a settings controller request, draft/gesture or other admission pause, then become eligible again without a token/revision change. That causes another automatic attempt after a failure/blocked attempt, contrary to the plan's terminality until manual retry, live re-enable or materially changed accepted context. Retain the attempted owner across temporary pauses; explicit re-enable/new owner should permit retry. Add a state regression for true → false → true eligibility under the exact same owner.

## Evidence and gates

Author handoff reports 3 native lifecycle/fingerprint tests, 1 mounted Chrome test, page/WASM tests compilation, formatting and diff checks passed. Packet contains the handoff rather than raw result logs; these results were read as author evidence, not independently rerun. No heavy test job was started because the parent reported host resource pressure. Fresh exact HEAD/clean-status, handoff hash and frozen-range diff checks pass.

The native tests cover one-shot eligibility, pause and board-thickness invalidation; they do not cover temporary eligibility loss after a terminal attempt, Runtime scene rebinding or stale generated rows. The mounted test uses the Runtime event seam with manually supplied readiness/generation state; it supports the switch/dispatch/remount behavior but cannot establish production lower-level admission or the omitted projections. Required negative regressions for the source findings belong to the repair before source clearance.

Paired public reopen, accepted equivalent-input reuse, physical invalidation, stale blocked Plate edit/Undo, scope races and exact request counts remain OPEN. Source presence or a mocked mounted test does not close these gates or Issue11/F7.6/parent joins. Existing manual status/Configure control parity and remaining full workflow gates are also retained; this report does not claim them executed.

Author was notified of the exact source seams and is preparing an isolated repair. No changes to the canonical graph, public API/schema, six streams, 62 parents or 14 RF records are requested.

Summary: Standards 1 P2 finding; Spec 3 P2 findings. Source requires bounded repair and rereview; no parent-wide start lock or new approval is introduced.
