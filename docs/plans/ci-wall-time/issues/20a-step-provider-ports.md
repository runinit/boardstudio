# 20a: Five STEP provider ports

Status: resolved
Type: task
Blocked by: [CAD collision and shared foundation](20-remaining-cad-provider-regressions.md)
Parent: [CI wall time map](../map.md)
Related: [Shared batch integration](20c-shared-batch-cutover.md)

## Work and completion

- Port the packaged-provider repair accounting, zero repair, projected line, planar seam and complete
  surface preview regressions to the shared batched browser contract after foundation A is ready.
  Work independently of [20b](20b-qualified-analysis-port.md); do not modify shared runner files
  concurrently without coordinating ownership.
- Compare original cases, meaningful assertions and failure behavior, including zero-repair offset
  views and input immutability, fixture integrity and handle release. Record substantive adaptation
  or nonessential removal and retain meaningful failure detection. Old node-removal ports are
  reference only; do not copy their broad handler-error drain.
- Keep `ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step` and
  `ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step` at their existing paths;
  do not edit either fixture. Exercise the real package and relevant negative controls; record
  evidence and obtain the applicable independent review before cutover. Leave the original required
  checks in place until validated replacement and integration.

## Answer

Resolved only for the local replacement decision at `3d816250` after the reported independent
final recheck; see [20](20-remaining-cad-provider-regressions.md#answer) for evidence and limits.
The five STEP suites are in the registered browser batch. The optional planar legacy comparison and
redundant Panasonic disk reread were removed; zero-repair offset sentinels and strict refusal checks
were strengthened. Complete browser and full-gate verification remain in
[22](22-node-free-gate-verification.md).

## Comments

- 2026-10-29: B follows A, can run in parallel with C, and precedes E. No gate or timing result is
  implied by this ticket. See [20](20-remaining-cad-provider-regressions.md#shared-batch-direction-and-exception-control--2026-10-29).

- 2026-10-29: Five STEP `.mjs` suites are present in the later cutover WIP, with their `.cjs`
  originals removed in that checkout. The approved adaptations remove the optional planar legacy
  comparison and redundant Panasonic disk reread; the zero-repair offset sentinel and strict refusal
  checks and pinned vendor path are strengthened. Earlier targeted mutation evidence in the
  implementation checkout's `.evidence/`: seven STEP mutations in
  `t20-step-mutations-IBOhj5/mutation-run.log`, four offset sentinels in
  `t20-step-sentinels-fj0tr2/mutation-run.log`, and strict refusal in `t20-zero-refusal-mutation.log`.
  See [fresh cutover evidence](20-remaining-cad-provider-regressions.md#seven-suite-cutover-wip-and-fresh-evidence--2026-10-29).
  At that WIP handoff recheck was pending; see the later [answer](#answer). This is not full-gate evidence.
