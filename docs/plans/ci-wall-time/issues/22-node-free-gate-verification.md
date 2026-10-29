# 22: Node-free gate verification

Status: ready-for-agent
Type: task
Blocked by: [Node-free Ergogen oracle](19-node-free-ergogen-oracle.md), [Node-free tooling tests](21-node-free-tooling-tests.md)
Parent: [CI wall time map](../map.md)
Related: [Web-split design](04-web-split-design.md)

## Question

Does the complete required gate preserve its coverage and failure behavior without Node, including
build subprocesses and the hosted-action runtime boundary?

## Work and completion

- Reconcile the original and replacement command and named-case inventories, failures and
  required-check behavior. Run the complete local gate with isolated targets and recorded results.
- Audit the actual process tree and executable provenance, including tools invoked indirectly.
  A masked `/usr/bin/node` or an executable-basename scan alone cannot exclude renamed or embedded
  runtimes. Describe what evidence can and cannot establish.
- Identify the runtimes used by hosted actions and a viable Node-free path if needed. Hosted
  execution, changes to stack branches, pushes and merges require further authorization.
- Record either a defensible local result with explicit hosted limits or a blocker; never mark an
  unaudited boundary as Node-free.

## Comments

- 2026-10-29: [18](18-face-completeness-replacement.md#answer) is resolved only for the local
  face decision, not integrated gate verification. Claude Opus recommended a complete browser step
  before face closure; it remains required here, along with the complete local gate. The current
  browser step invokes seven other Node provider regressions, and tooling Node tests remain. The
  user prohibits Node anywhere in the full gate: do not bypass, skip, or run those regressions under
  Node as a substitute for the required Node-free proof. Resolve their replacements and retain
  coverage first. Focused face/scheduler evidence is not a complete browser or full-gate run.
  Executable-basename scans cannot rule out renamed or embedded Node, cached package stamps do not
  authenticate arbitrary bytes, and hosted actions remain unaudited; none establishes a green gate.

- 2026-10-29: CAD sequencing is A ([20](20-remaining-cad-provider-regressions.md)), then B and C
  ([20a](20a-step-provider-ports.md), [20b](20b-qualified-analysis-port.md)) in parallel, then E
  ([20c](20c-shared-batch-cutover.md)). Ergogen [19](19-node-free-ergogen-oracle.md) is not a
  technical prerequisite of those CAD tasks, but its unresolved Node-dependent oracle remains a
  separate full-gate blocker. The local complete gate and hosted-action runtime audit remain distinct
  evidence requirements; focused collision results do not satisfy either.

- 2026-10-29 (WIP handoff): The later implementation WIP replaces the seven provider invocations with a
  registered browser batch; its fresh private-package and partial scheduler evidence is recorded in
  [20](20-remaining-cad-provider-regressions.md#seven-suite-cutover-wip-and-fresh-evidence--2026-10-29)
  and awaits independent recheck. The three Node tooling `.cjs` tests and `node --version` remain,
  and hosted-action runtimes are unaudited. No complete browser step, full gate, Node basename or
  runtime independence, hosted action or 10-minute result is established. The package stamp checks
  inputs, not bytes; the shared checkout package remains stale. This ticket remains unresolved.

- 2026-10-29: The reported final independent recheck permits local replacement decisions for
  [20](20-remaining-cad-provider-regressions.md#answer), [20a](20a-step-provider-ports.md#answer),
  [20b](20b-qualified-analysis-port.md#answer) and [20c](20c-shared-batch-cutover.md#answer) at
  `3d816250`; the preceding pending-recheck note is historical. This ticket remains blocked by
  [19](19-node-free-ergogen-oracle.md) and [21](21-node-free-tooling-tests.md). The complete
  browser step and full gate, runtime provenance and hosted-action audit are not established.
  Three tooling `.cjs` tests and `node --version` remain; the shared package is stale and the private
  stamp does not authenticate output. The approximately 10-minute cold full-gate target is not
  established; its performance baseline also remains blocked in [23](23-migrated-gate-performance-baseline.md).

- 2026-10-29: The CAD worker protocol now runs as a browser-gate suite at implementation `9d76fb00`
  (local; see [21](21-node-free-tooling-tests.md#comments)), so earlier "three tooling `.cjs`
  tests" counts become two, plus `node --version`; model-lookup is pending. Three early Node
  STL/oracle checks were removed from the gate with user approval; the manual download checker
  remains Node-based. No full gate, hosted-action audit or timing result follows. This ticket stays
  blocked.
