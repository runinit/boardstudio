# 010: PCB adapter with placement and Routing & nets parity

Status: ready-for-human
Type: build
Blocked by: [002](002-toolbar-accordion-label-fitting.md), [005](005-retain-objects-capabilities.md)
Parent: [map.md](../map.md)
Owner: PCB workspace adapter and PCB-owned content; coordinator alone edits shared routing hunks

## Boundary

Adapt `web/src/presentation/pcb_workspace.rs` Objects/toolbar/PcbInspector to 009. Reuse
applicable Layout Select/Transform/Align/Snap mounts and 005's Add/setup/parts placement
flows; do not invent PCB tools or move electrical state to shared chrome. Keep SVG PcbScene
and mounted-module pick/gesture lifetimes. Selection / Routing & nets content both retain
owners across panel/content switches. Numeric position remains the existing editor.

Retain every [PCB inventory row](../inventory.md#pcb-adapter): controller carousel/selection,
matrix/direct/automatic wiring, terminal/pin locks/manual assignments, conflict replacement,
reviewed remap, module detail and Layers/diagnostics. Preserve save/hand-off protection,
readiness, pending/failure and stale-scope guards; this is not routing/export redesign.
Classify PCB reveals: selection alone never opens Inspect, explicit details/wiring requests do.
M2 phase approval is required. Retain the already keep-mounted Selection/Routing & nets
owners and their wiring/module registered draft signals/guards above panel visibility;
hoist only actual per-owner reset state found during the audit, not a second registry.
Confirm any unregistered retained form's trigger/admission before changing update safety.
Off-screen blockers must identify PCB and their existing editor/remedy.

## Acceptance criteria

- [ ] Tick PCB inventory, all retained Add placement/browse paths and Inspector sections.
- [ ] Draft/error/selection survives close, section switch, A→B→A and workspace round trip.
- [ ] No shell dock mode remains reachable in PCB, and panel opening does not change canvas.

## Proof map

- PCB tools/pick — valid board/component/module → use applicable row tools and SVG/Objects
  selection → same scoped result, ordinary pick leaves Inspect closed, explicit Inspect
  exposes the sole numeric editor · mounted · planned
- Pane/draft — Routing & nets draft/error in owner A → Selection pane, close, choose B and
  return A/workspace → original owner draft/feedback restored, no foreign edit · mounted · planned
- Hidden registered owner — real unresolved wiring/module explicit-submit draft before hide →
  switch workspace and request Update Now → existing PCB/editor remedy defers reload; return
  restores it and existing submit/Cancel resolves only its own blocker · mounted · planned
- Wiring guards — existing pending/handoff-protected/conflicting-net fixture → invoke wiring
  change, replacement/remap through relocated content → existing explicit review/disabled
  reason and accepted result/Undo preserved; stale board response rejected · mounted · planned
- Placement return — new chooser entry → Browse all parts, return to PCB and place → same
  captured board/instance, one accepted placement and Undo; failed placement retains message · mounted · planned

## Checks and mutations

[Verification](../verification.md): `check.py quick`, `check.py lint`, `check.py browser --only pcb`
plus Layout/Parts subsets if shared mounts change; changed page files via `run-wasm-tests.py --files`.
Reuse queued-edit/remap protection evidence; pane-unmount proof-gap mutation targets restored
text/feedback, not generic DOM counts. Coordinator independently reruns stale-board rejection.
