# 20c: Shared CAD batch integration and cutover

Status: resolved
Type: task
Blocked by: [CAD collision and shared foundation](20-remaining-cad-provider-regressions.md), [Five STEP provider ports](20a-step-provider-ports.md), [Qualified analysis port](20b-qualified-analysis-port.md)
Parent: [CI wall time map](../map.md)
Related: [Node-free gate verification](22-node-free-gate-verification.md)

## Work and completion

- Integrate the collision and six remaining provider replacements through one shared batched
  browser runner, not the superseded direct single-suite wrapper. Reconcile suite/case inventory,
  execution and meaningful failure detection, including adapted or removed nonessential behavior.
  Keep both vendor STEP assets at their existing paths; do not edit them.
- Validate actual package freshness, fixture and transport failures, assertions and exception
  controls, async completion, CDP error handling, missing/invalid/duplicate results and nonzero
  propagation. Exercise the real scheduler path, lock inheritance and receipt classification, and
  demonstrate a regression mutation fails. Obtain applicable independent review and address its
  findings before removing the original gate invocations.
- Record exact local evidence and remaining boundaries; do not claim the complete browser step,
  Node-free full gate, hosted-action runtime audit or performance from focused batch evidence.
  Those are separate follow-ups in [22](22-node-free-gate-verification.md) and
  [23](23-migrated-gate-performance-baseline.md). No push, merge or hosted CI.

## Answer

Resolved only for the local replacement decision at `3d816250`: the registered shared batch
replaces seven provider invocations. The reported independent final recheck and focused scheduler,
control and targeted mutation evidence are recorded in [20](20-remaining-cad-provider-regressions.md#answer).
This is not a complete browser step or full gate; those remain in
[22](22-node-free-gate-verification.md), with performance in
[23](23-migrated-gate-performance-baseline.md).

## Comments

- 2026-10-29: E follows A and both parallel branches B and C; Ergogen [19](19-node-free-ergogen-oracle.md)
  is not a technical prerequisite for these CAD ports. Coordinate shared file ownership.

- 2026-10-29: The implementation checkout now has an uncommitted registered seven-suite batch
  cutover, including trusted driver and controls. Independent Opus review found no proven code
  defect but requested fresh evidence. The subsequent private-package hermetic, scheduler and
  three-mutation evidence is recorded in [20](20-remaining-cad-provider-regressions.md#seven-suite-cutover-wip-and-fresh-evidence--2026-10-29).
  At that WIP handoff recheck was pending; see the later [answer](#answer). The partial scheduler
  run is not the complete browser step; [22](22-node-free-gate-verification.md) retains full-gate
  verification.
