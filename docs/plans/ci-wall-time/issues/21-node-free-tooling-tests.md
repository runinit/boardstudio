# 21: Node-free tooling tests

Status: ready-for-agent
Type: task
Blocked by: [Node-free Ergogen oracle](19-node-free-ergogen-oracle.md)
Parent: [CI wall time map](../map.md)
Related: [Web-split design](04-web-split-design.md)

## Question

Can the model-lookup and CAD worker tooling regressions preserve their actual code, assertions and
execution-context behavior without Node?

## Work and completion

- Compare each replacement with its original test. A normal browser page is not automatically
  equivalent to the model-lookup test's bare V8 context; the CAD worker test also exercises its real
  download STL/oracle helper.
- Exercise success and failure paths, retain the original required checks until parity is shown,
  and remove the gate's incidental Node version probe only when it is no longer required.
- Record evidence or a concrete blocker. No push, merge or hosted CI.

## Comments

- 2026-10-29: CAD worker protocol slice, implementation commit `9d76fb00` ("Run CAD worker protocol
  checks in browser", local only; no push or merge). The protocol moved from the Node
  `test-measure-cad-worker.cjs` invocation to a `measure-cad-worker` suite in the trusted browser
  provider gate: the 17 original worker cases plus one source-sha negative, 18 total. The suite
  SHA-checks the real worker source and fixtures. The original used a fake `Worker`, so that is
  unchanged; no real transfer coverage is claimed. A Claude Opus review approved the protocol with
  the documented changes.
- Removal with user approval: the gate no longer runs three early Node STL/oracle checks. The manual
  `check-cad-page-downloads.py`/`.cjs` remains Node-based; it checks real downloads, oracle markers
  and that a truncated STL fails closed. The gate therefore loses its early drift warning for those
  paths. Quick mode needs a browser for the worker suite.
- Evidence (implementation `.evidence/`): `worker-gate-final.log` is a partial `check.run_plan`
  browser-provider run (private current-digest package and stamp, Chrome 156, Node masked, offline,
  shared lock); `worker-mutations.log` shows 4 intentional target mutations killed;
  `worker-final-review-{check,quick,provider,cad-build}.log` pass 40/18/6/16;
  `worker-final-staged-inventory.log` validates the staged checkout without the other lane's untracked
  pilot. The commit excludes the temporary opt-in hunk and all model-lookup and Ergogen WIP. The
  private stamp verifies inputs only and does not authenticate output.
- Not established: this ticket stays unresolved because the model-lookup replacement is pending
  and its bare-V8 context parity is unshown. The two other `.cjs` tooling checks and
  `node --version` still require Node. No full gate, hosted-action audit, runtime provenance or
  ten-minute result exists. [22](22-node-free-gate-verification.md) remains blocked.
