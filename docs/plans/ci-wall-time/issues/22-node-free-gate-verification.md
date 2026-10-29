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
