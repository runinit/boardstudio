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
- Research 001 resolved: spike's 19 errors are surfaced unsolicited-event loss in an
  unsubscribed experimental Network domain, not command failures; exact field unknown
  (truncated archive). See [001](issues/001-protocol-compatibility.md) and the
  [feasibility report](../../research/chromiumoxide-migration-feasibility.md) (§1),
  archived on `research/chromiumoxide-feasibility@74a6230`.
- Research 002 resolved: consumer/launch inventory complete; proposed harness contract
  keeps `CHROME_BIN`-first discovery, process-owned profile plus child
  `XDG_CONFIG_HOME`, shared flags/viewport, and leaves the WASM runner untouched.
  See [002](issues/002-launch-contract.md) and the
  [feasibility report](../../research/chromiumoxide-migration-feasibility.md) (§2).

## Children

- [Chrome protocol compatibility and event failures](issues/001-protocol-compatibility.md)
- [Portable isolated browser launch and prerequisite contract](issues/002-launch-contract.md)
- [Coverage and receipt parity at cutover](issues/003-parity-cutover.md)

## Not yet specified

- Version support and upgrade policy, contingent on protocol evidence.
- Minimal harness boundary, accessible-name lookup fidelity, and shared launch ownership.
- Migration sequencing across drag smoke, CAD capture and Graphite acceptance.
- Evidence needed to distinguish migration regressions from existing application failures.
- Exact cutover gates and retirement of old dependencies, after the parity inventory.

## Out of scope

- Replacing ChromeDriver underneath wasm-bindgen-test or changing mounted WASM tests.
- Removing all Node tooling from the repository.
- Weakening assertions, hiding page errors, accepting incomplete runs, or retries that mask failures.
- Fixing unrelated application bugs such as the part-drag Undo failure.
- Firefox/WebKit support.
