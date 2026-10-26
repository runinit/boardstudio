# 005: Rust drag smoke end to end

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [Chromiumoxide browser checks](../map.md)
Related: [Real-browser checks drive Chrome from Rust](../../cad-deepening/issues/016-rust-browser-harness.md), [Prove browser harness fidelity before cutover](004-feasibility-proof.md)

## What to build

A developer can run the supported real-page drag smoke through a minimal Rust chromiumoxide
harness and receive the same checks, failure verdicts and verified success receipt as the
existing Playwright runner. Deliver the narrow end-to-end path, not a generic browser framework.
Reuse the existing spike; do not repeat a broad feasibility exercise. Keep the old runner
available for same-build comparisons until the parity gate passes. Keep its dependencies
available to consumers not yet migrated.

## Acceptance

- [ ] The harness has its own manifest/lockfile outside the app workspace. Implement only
  consumer-required helpers: isolated launch, role/name lookup, bounded waits/actionability,
  trusted input, evaluation, fatal error capture and screenshots as required.
- [ ] Portable discovery honours CHROME_BIN first. Each run owns writable config/profile
  storage and cleans up its processes on success/failure; fresh runs do not share IndexedDB.
  Missing prerequisites and startup failures are actionable, without personal absolute pins.
- [ ] Pin the library and CI browser together; log versions and retain the existing launch
  flags/viewport. No machine-global profile or sandbox-policy changes.
- [ ] Preserve every existing drag-smoke assertion, including Part/Column/Stagger/Ctrl-offset,
  trusted pointer capture/live preview, durable saves, complete-document Undo/reload, timing,
  and collision-check observations. Preserve launcher arguments, exit codes, receipt fields,
  provenance validation and failure logs. Never emit a success receipt for partial runs.
- [ ] Same-source/build old/new runs demonstrate matching checks/verdicts and comparable
  receipts. A reproduced app failure remains a failure in both; successful cutover requires
  the full gate to pass, not merely matching failures. Escalate a blocking app defect separately.
- [ ] Negative controls prove page exceptions AND unhandled rejections fail the run; startup,
  command/protocol failure, bounded timeout and assertion failure remain distinguishable.
  Lookup/input tests cover exact/scoped names and absent/ambiguous/non-actionable targets.
- [ ] Capture full redacted invalid-event evidence where it occurs. Identify schema mismatch
  and required-event impact; default dropping/warning is not proof of harmlessness. Required
  event loss or unresolved impact blocks cutover. Do not swallow evaluation errors in waits.
- [ ] The supported check entry point and CI exercise the Rust path after parity, with harness
  formatting/lint coverage and reproducible browser/prerequisite setup. Mounted WASM runner
  stays unchanged; other Playwright consumers keep working during this slice.

## Verification and scope

Use TDD at the existing public launcher/output seams: demonstrate the expected regression
before the fix, then run affected tests and typechecking during development. At closure run
the full supported check and combined Spec + Standards review; record terminal verdicts,
source/build identity and any blocked gates honestly. No weakened assertions, masked page
errors, baseline deletions or retry-until-green. Preserve the archived prototypes and unrelated
work. Builds follow repository concurrency/lock rules; evidence uses supported receipts and
logs, not custom hash manifests. All checks must be runnable by humans and CI without agent tools.
