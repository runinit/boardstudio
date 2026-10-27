# Map: Chromiumoxide browser checks

Type: wayfinder:map

## Destination

An evidence-backed migration plan for replacing the Node/Playwright real-page checks with
a Rust chromiumoxide harness, preserving assertions, trusted pointer interactions,
Undo/reload checks, receipts and CI verdicts.

## Notes

- Planning only: no production migration until the route is settled.
- User confirmed the destination and coverage preservation in this conversation.
- Reuse [Real-browser checks drive Chrome from Rust](../cad-deepening/issues/016-rust-browser-harness.md)
  as the existing proposal, not a second independent implementation contract.
- Prototype primary source: local branch `prototype/chromiumoxide-spike`,
  `scripts/browser-harness-prototype/RESULTS.md`. Its recorded protocol errors need fresh investigation.
- Use grilling and domain-modeling for decisions, research for primary-source investigations,
  and prototype for concrete feasibility demonstrations.
- No pushes without explicit user authorization. Preserve other active plans and prototypes.
- Local Markdown tracker requires an index of all children; dependencies live in their headers.

## Decisions so far

- Destination confirmed by the user: real-page checks migrate; existing mounted Rust/WASM
  tests retain their runner. Coverage and CI verdicts must not weaken.
- [Chrome protocol compatibility and event failures](issues/001-protocol-compatibility.md):
  archived errors concern unsolicited Network events, not observed command failures; exact
  mismatch and required-event safety remain unproven (truncated archive). See the
  [feasibility report](../../research/chromiumoxide-migration-feasibility.md) (§1),
  archived on `research/chromiumoxide-feasibility@74a6230`.
- [Portable isolated browser launch and prerequisite contract](issues/002-launch-contract.md):
  consumer/launch inventory complete; proposed harness contract
  keeps `CHROME_BIN`-first discovery, process-owned profile plus child
  `XDG_CONFIG_HOME`, shared flags/viewport, and leaves the WASM runner untouched.
  See the
  [feasibility report](../../research/chromiumoxide-migration-feasibility.md) (§2).

- [Coverage and receipt parity at cutover](issues/003-parity-cutover.md): pin the CI browser
  and library together; prove fidelity before replacement; cut over drag smoke first, then CAD
  capture and Graphite acceptance, retaining each old runner until its parity gate passes.

- [Prove browser harness fidelity before cutover](issues/004-feasibility-proof.md): reuse the
  existing input spike; keep a consumer-driven minimal harness and make missing fidelity
  proofs per-consumer migration gates, not another broad prototype.

## Children

- [Chrome protocol compatibility and event failures](issues/001-protocol-compatibility.md)
- [Portable isolated browser launch and prerequisite contract](issues/002-launch-contract.md)
- [Coverage and receipt parity at cutover](issues/003-parity-cutover.md)
- [Prove browser harness fidelity before cutover](issues/004-feasibility-proof.md)

## Approved build handoff

The user approved these vertical slices after the Wayfinder decisions. The existing
[Real-browser checks drive Chrome from Rust](../cad-deepening/issues/016-rust-browser-harness.md)
remains the source proposal; the resolved decision tickets above govern its updated proof
strategy. The following build tickets preserve those contracts and do not change this map's
planning-only scope. They are published for implementation, not started by publication.

- [Rust drag smoke end to end](issues/005-rust-drag-smoke.md) — first frontier; minimal harness
  plus the supported launcher and same-build parity.
- [Rust CAD runtime capture](issues/006-rust-cad-runtime-capture.md) — waits on drag smoke;
  includes cache/service-worker and capture-output proof.
- [Rust Graphite acceptance checks](issues/007-rust-graphite-acceptance.md) — waits on drag
  smoke; covers both current acceptance entry points.
- [Retire Playwright browser dependencies](issues/008-retire-playwright-dependencies.md) —
  waits on all three migrations and their parity gates.

Dependency sketch: drag smoke → {CAD capture, Graphite acceptance} → dependency retirement.
Retain old runners until their respective gates pass; final shared dependency removal waits
until all consumers have migrated. No parent specification is closed by this handoff.

## Not yet specified

- No additional design decisions currently identified. The approved build handoff reconciles
  the existing proposal with the resolved decisions; runtime proofs remain acceptance gates.
- Any incompatibility discovered during the bounded acceptance probes returns here as a
  concrete decision; it does not authorize ignoring required events or weakening assertions.

## Out of scope

- Replacing ChromeDriver underneath wasm-bindgen-test or changing mounted WASM tests.
- Removing all Node tooling from the repository.
- Weakening assertions, hiding page errors, accepting incomplete runs, or retries that mask failures.
- Fixing unrelated application bugs such as the part-drag Undo failure.
- Firefox/WebKit support.

## Retrospective implementation, 2026-10-26

On `fix/drag-save-undo` after `b817585c`, the authorized follow-up added scheduler
launch timestamps and startup diagnostics, and clarified delegated implementation
handoffs in the model lookup. The Graphite draft gained pointer-capture and dependent
Undo verdict checks plus optional launch-time local identity; the served build remains
unverified for an arbitrary preview URL. Those Graphite changes remain in the dirty
Ticket 007 draft pending real-browser verification and its existing parity gates. The
TDD skill outside this repository was updated to select existing seams autonomously.
U1 and Snap decisions remain open; this entry is not an acceptance or closure receipt.
