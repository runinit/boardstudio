# 003: Coverage and receipt parity at cutover

Status: resolved
Type: grilling
Blocked by: [Chrome protocol compatibility and event failures](001-protocol-compatibility.md), [Portable isolated browser launch and prerequisite contract](002-launch-contract.md)
Parent: [Chromiumoxide browser checks](../map.md)

## Question

Given the research, which minimal harness boundary, compatibility policy and staged cutover
will preserve the existing drag smoke, CAD runtime capture and Graphite acceptance contracts?

Settle trusted pointer/modifier input, role/name lookup, error/event capture, cache/service-worker
behaviour, receipt/output equivalence, failure diagnostics and dependency retirement gates.
Use the existing migration specification as input. Inventory actual consumers before claiming
parity; compare old/new runners on the same source/build. Treat an existing failing app
assertion as a retained failure, not a migration pass or permission to delete it.

Resolve with the user; do not substitute implementation slices for decisions.

## Answer

The user accepted all three recommendations in the Wayfinder conversation:

- Pin chromiumoxide and CI Chrome together. Check compatibility explicitly before upgrades;
  local Chrome remains selectable through `CHROME_BIN`, and runs log their versions.
- Require a feasibility prototype before replacement. Prove protocol/event handling, trusted
  pointer/modifier input, waits, error capture, storage isolation, and service-worker/cache
  behaviour against the old runner on the same build. Existing app assertion failures remain
  failures, not evidence of successful cutover.
- Stage cutover: drag smoke, then CAD capture and Graphite acceptance. Retire each old runner
  only after its own parity gate passes; retain dependencies while other consumers need them.
  Mounted Rust/WASM tests keep their current runner.

These settle policy and ordering, not feasibility. The truncated protocol-error payloads do
not establish harmlessness. The next decision is whether concrete prototype evidence meets
these conditions; required-event loss and missing coverage are no-go findings.
