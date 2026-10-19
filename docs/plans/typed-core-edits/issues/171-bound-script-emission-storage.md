# 171: Reject oversized geometry-script output before copying it

Status: ready-for-agent
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Geometry scripts stop as soon as their shared object-count or combined emitted
string-byte budget would be exceeded. Opening or applying an oversized script
must fail cleanly instead of collecting large amounts of output that will later
be discarded. Normal scripts retain their current output and stable identities.

The audit's 4,149-byte script repeatedly emitted four 4,096-byte string fields.
Packaged WASM linear memory grew from 1,835,008 to 185,532,416 bytes before the
operation limit rejected evaluation. This measures linear memory growth, not
allocator usage or process peak memory.

## Acceptance criteria

- [ ] Reproduce the delayed emission-count/byte rejection before the fix. Cover
  both part and rectangle emitters through real script evaluation.
- [ ] Before copying emitted strings or appending an object, check one shared
  emission-count budget and one finite, documented aggregate UTF-8 byte budget.
  Count every copied string field and use checked arithmetic for the accounting.
- [ ] Part and rectangle emissions share the budgets, including mixed calls.
  Exact-boundary output succeeds and the first over-budget emission is refused;
  a large first emission cannot allocate its copied output before preflight.
- [ ] A script cannot catch the budget error and publish a successful truncated
  result or resume copying output. Budget exhaustion stops evaluation and remains
  a failed script operation, with no partial document changes accepted.
- [ ] Existing source, operation, string, array, map and recursion limits stay in
  force. Normal emitted IDs, geometry, poses and validation remain compatible.
- [ ] Native regression checks cover count, aggregate bytes, UTF-8 accounting,
  mixed emitters, attempted error recovery and unchanged accepted document after
  failed application. Fresh packaged Core WASM evidence exercises the public
  open/apply boundary and shows bounded refusal before the prior growth pattern.
  Record actual measurements without claiming allocator/RSS proof.

## Comments

Created from the user's audit findings after primary-source research. Independent
of the other four active tickets. Bound the existing emission collector; do not add a
replacement scripting service or change unrelated script language behavior.
