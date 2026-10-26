# 004: Prove browser harness fidelity before cutover

Status: ready-for-human
Type: prototype
Blocked by: [Coverage and receipt parity at cutover](003-parity-cutover.md)
Parent: [Chromiumoxide browser checks](../map.md)

## Question

Can a small throwaway chromiumoxide harness satisfy the approved parity policy without
silencing errors or rebuilding Playwright wholesale?

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

## Resolution

Pending prototype evidence and the user's reaction. A successful demonstration informs the
minimal harness boundary and the precise per-consumer cutover gates; it does not authorize
production migration by itself.
