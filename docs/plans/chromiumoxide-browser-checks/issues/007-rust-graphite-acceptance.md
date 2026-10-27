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

## 2026-10-26: entry-point inventory

`test-graphite-usability.cjs` directly requires `test-graphite-feedback.cjs`;
both are manual exclusions in `scripts/test-check.py`. The feedback runner uses
full `playwright`, `CHROME_BIN` (default `/usr/bin/chromium`),
`BOARDSTUDIO_PREVIEW_URL` (default `http://127.0.0.1:8182/`) and optional
`BOARDSTUDIO_SCREENSHOTS`. It starts at 1513x1043 in dark mode, then checks each
of six workspaces at 1513x1043, 1024x1043 and 390x844. With screenshots set,
it creates the directory and writes one `<workspace>-<width>.png` per iteration.

A completed run prints 75 labeled checks: 27 before the workspace loop, six
Case/Parts checks, six workspace reference checks and 36 viewport checks. Waits
and interactions are also required: some labels print `true` only after a
successful wait. Failed labels and page errors accumulate; interaction or wait
errors abort the sequence, and either route exits nonzero. There is no structured
source/build receipt in this runner. A same-site comparison must record identity
outside it and must not treat unprinted labels as passing.

The shared Rust driver supplies isolated launch, role lookup, trusted clicks and
drags, bounded waits and fatal page-error collection. The port still needs the
consumer-specific setup, dark emulation, viewport changes, screenshots and all
checks, followed by negative controls and same-build comparison. No Rust Graphite
acceptance implementation or parity result is claimed. The public launcher RED
at this HEAD is `.evidence/ticket007-launch-red.log`: `cargo run --offline
--manifest-path tools/browser-harness/Cargo.toml --bin graphite-feedback` exited
101 because that binary does not exist. The current worktree also contains
uncommitted 006 drafts, so this is not a clean-source acceptance receipt.
