# 007: Rust Graphite acceptance checks

Status: ready-for-agent
Type: build
Blocked by: [Rust drag smoke end to end](005-rust-drag-smoke.md)
Parent: [Chromiumoxide browser checks](../map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md)

## What to build

A designer can invoke the existing Graphite acceptance workflows through the Rust harness
with unchanged checks and readable PASS/FAIL output, without Playwright. Inventory both the
feedback and usability entry points; do not silently port only the one named in older prose.
Keep these manual against-server checks manual unless the current check already runs them.

## Acceptance

- [ ] Preserve every existing assertion, environment/input contract, viewport, dark-scheme
  emulation, optional screenshot behaviour, output labels and exit status for each consumer.
- [ ] Run old/new against the same source/build/server state and compare the assertion inventory,
  verdicts and screenshot conditions. Existing app failures stay failures; unrun checks do not pass.
- [ ] Reuse the minimal harness's isolated launch, trusted input, bounded waits and fatal page
  error handling; add only helpers exercised by these consumers. No visual redesign in this slice.
- [ ] Negative controls demonstrate failed assertions/page errors produce failure output and
  nonzero exit. Preserve the supported check's explicit manual-exclusion list and entry-point docs.
- [ ] Keep old entry points available until parity passes; verify browser consumer dependencies
  remain installed for other unmigrated workflows.

## Verification and scope

Use TDD at the existing public launcher/output seams: demonstrate the expected regression
before the fix, then run affected tests and typechecking during development. At closure run
the full supported check and combined Spec + Standards review; record terminal verdicts,
source/build identity and any blocked gates honestly. No weakened assertions, masked page
errors, baseline deletions or retry-until-green. Preserve the archived prototypes and unrelated
work. Builds follow repository concurrency/lock rules; evidence uses supported receipts and
logs, not custom hash manifests. All checks must be runnable by humans and CI without agent tools.
