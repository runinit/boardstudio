# 004: Prove browser harness fidelity before cutover

Status: resolved
Type: prototype
Blocked by: [Coverage and receipt parity at cutover](003-parity-cutover.md)
Parent: [Chromiumoxide browser checks](../map.md)

## Question

Does the existing chromiumoxide spike supply enough feasibility evidence to specify the
minimal harness, and which missing proofs must gate each consumer's cutover?

The user stopped duplicate prototyping. Reuse the archived spike; do not commission a new
broad spike. Only bounded probes for specific remaining questions belong here.

Extend or reuse the archived `prototype/chromiumoxide-spike` as a primary source; preserve
that branch and unrelated application work. Use the prototype and diagnosing-bugs skills.
Keep this off production test paths. Human review determines the verdict.

## Evidence to discuss

- Pin and log the chosen chromiumoxide/Chrome pair. Capture full offending event payloads
  (redacted) to identify the old decoding mismatch; distinguish required events from events
  the checks do not consume. Do not treat dropping unknown events as proof of safety.
- Real trusted pointer and modifier input, capture during drag, accessible-name lookup and
  bounded waits/actionability. Show failures as well as successes using small local fixtures.
- Page exceptions and unhandled rejections fail visibly. Browser/session startup failure,
  protocol failure, timeout and application assertion failure remain distinguishable.
- Browser-owned writable profile/config and cleanup; two runs do not share IndexedDB state.
- Cache and service-worker behaviour and response/event metadata match the existing CAD
  capture contract; test bypass versus blocking rather than assuming equivalence.
- Compare old/new observations on the same source/build where applicable. Preserve the
  part-drag Undo failure if it still reproduces; do not delete assertions to obtain parity.

Report executed commands, terminal verdicts, limits, and remaining gaps. Prefer bounded
fixture probes to full builds. Any necessary build uses the project build lock; never run
CPU-saturation probes. Archive the prototype locally without switching the shared checkout
or pushing. Link evidence here; retain prototype code for comparison.

## Existing-spike gap review

Source review against `scripts/test-drag-interactions.cjs` (no runtime rerun):

- Archived `prototype/chromiumoxide-spike:scripts/browser-harness-prototype/RESULTS.md`
  records successful trusted Ctrl/plain drags, pointer capture, and exact/scoped role lookup.
  These are feasibility evidence, not proof of every production assertion.
- The spike's `main.rs:149-153` prints collected page errors and invalid-message counts, then
  returns `Ok(())`. A zero exit code therefore does not prove fatal error detection.
- `wait_js` and `click_role` demonstrate polling; the former discards evaluation errors via
  `.ok()`, while the latter clicks the first matching node. Full production error propagation,
  uniqueness/actionability and timeout semantics are not established.
- Production drag smoke additionally asserts live preview, durable saved document signatures,
  exact Undo, reload, page errors and receipts. Preserve these in the port and compare the
  old/new results on the same build; input feasibility alone cannot establish parity.
- Isolated IndexedDB/profile cleanup remains unproven by the archived results. CAD-specific
  service-worker/cache event parity can gate CAD capture rather than block drag-smoke work.

No new implementation or second spike was launched for this review. A previous duplicate
prototype task was cancelled on the user's instruction.

## Resolution

The user accepted both recommendations after reviewing the existing-spike gaps:

- The existing spike establishes input feasibility; do not repeat a broad spike. Keep the
  harness limited to capabilities used by current consumers: isolated launch, lookup/waits,
  trusted input, fatal error capture, evaluation and screenshots (plus consumer-required
  network/cache/service-worker support for CAD). Do not build a general Playwright replacement.
- Missing proofs become migration acceptance gates, not a prerequisite for writing the port.
  Drag smoke must retain fatal errors, durable save, exact Undo/reload and receipt contracts;
  compare old/new against the same build. Cache/service-worker parity gates CAD capture.
- Retain each old runner until its replacement meets its parity gate. An existing application
  failure must remain a failure and cannot be used to claim a passing cutover.

This resolves feasibility planning, not the unexecuted runtime proofs. The next planning
step is reconciling the existing migration specification and producing bounded build tickets;
no production runner was changed.
