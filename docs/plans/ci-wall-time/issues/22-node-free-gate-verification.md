# 22: Node-free gate verification

Status: ready-for-agent
Type: task
Blocked by: [Remaining CAD provider regressions](20-remaining-cad-provider-regressions.md), [Node-free tooling tests](21-node-free-tooling-tests.md)
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
