# 20b: Qualified analysis provider port

Status: resolved
Type: task
Blocked by: [CAD collision and shared foundation](20-remaining-cad-provider-regressions.md)
Parent: [CI wall time map](../map.md)
Related: [Shared batch integration](20c-shared-batch-cutover.md)

## Work and completion

- Port `cad/wasm/tests/qualified-analysis.cjs` to the shared batched browser contract after
  foundation A is ready, independently of [20a](20a-step-provider-ports.md). Coordinate shared
  runner ownership; do not concurrently edit its integration files.
- Compare the original cases and failure behavior, including shared and detached buffers, transfer
  copies, sealed ownership, handles, query limits and real vendor asset. Assess Node process-memory
  diagnostics separately from regression assertions; document material changes. Nonessential
  behavior may be removed or updated, but meaningful failure detection and safety must remain.
- Keep `ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step` and
  `ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step` at their existing paths;
  do not edit either fixture. Exercise the actual package and relevant negative controls; record
  evidence and obtain the applicable independent review before cutover. Leave the original required
  check in place until validated replacement and integration.

## Answer

Resolved only for the local replacement decision at `3d816250` after the reported independent
final recheck; see [20](20-remaining-cad-provider-regressions.md#answer) for evidence and limits.
The registered suite runs both triangle and exponent cases without the former filter. Browser
WASM/heap diagnostics replace Node process-memory diagnostics, `retainedStorage` assertions remain,
and isolation/shared-buffer checks are strengthened. Complete browser and full-gate verification
remain in [22](22-node-free-gate-verification.md).

## Comments

- 2026-10-29: C follows A, can run in parallel with B, and precedes E. No gate or timing result is
  implied by this ticket. See [20](20-remaining-cad-provider-regressions.md#shared-batch-direction-and-exception-control--2026-10-29).

- 2026-10-29: The qualified `.mjs` suite is present in the cutover WIP and its `.cjs` original is
  removed there. The approved port removes the triangle/exponent filter so both run, replaces Node
  process-memory diagnostics with browser WASM/heap diagnostics, retains `retainedStorage`
  assertions, and strengthens isolation/shared-buffer and pinned vendor-path checks. Earlier
  qualified mutation evidence in the implementation checkout's
  `.evidence/t20-qualified-lane-CZEKgINX/mutation-run.log`
  records four killed mutations: no COOP, no shared buffer, no detachment and retained equality.
  See [fresh cutover evidence](20-remaining-cad-provider-regressions.md#seven-suite-cutover-wip-and-fresh-evidence--2026-10-29).
  At that WIP handoff recheck was pending; see the later [answer](#answer). Diagnostics are not
  equivalent process-memory measurements and focused evidence is not a full gate.
