# 005: Rust drag smoke end to end

Status: claimed
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

## Comments

### 2026-10-26: vertical slice implemented; parity gate BLOCKED by two app defects (ticket stays claimed)

Built the minimal excluded crate `tools/browser-harness/` (own `[workspace]`,
`Cargo.toml` pins chromiumoxide `=0.9.1`, own `Cargo.lock`; lib + `drag-smoke` +
`negative_probe` binaries). Launcher opt-in: `BOARDSTUDIO_DRAG_DRIVER=rust`
(default stays `node`); `check.py` `LINT` appends harness fmt/clippy (gate `lint`
step passes, 4.9 s). All Cargo/wasm work under the build lock; isolated per-run
profile/config dirs with whole-run-dir cleanup; CHROME_BIN-first discovery.

Contract port: every old-runner assertion is preserved, including Control-first
gesture for the Control-swapped offset handle (the toolbar swaps the stagger
handle label only after Control is down), pointer-capture/pointerdown-trust/
movement checks, in-page exact verdicts (all document/state comparisons run
page-side so no float repr crosses the bridge), durable-save wait, exact
page-signature Undo equality with `diff_paths` diagnostics, exact scene restore,
receipt byte-compat (launcher checksum verifies), timing bounds, quaternion
collision observations/provenance validators. `force: 0.5` on held buttons
matches Playwright's reference input exactly (page-observable pressure 0.5).
Negative probe: 12/12 controls green (page exception + unhandled rejection fail
the run; command/timeout/startup/assertion failures distinct; exact/scoped/
absent/ambiguous/zero-area/hidden lookups; IndexedDB isolation between contexts).
Lib unit tests 8/8; seam tests `test-browser-tools.py` 6/6, `test-check.py` 31/31.

RED->GREEN record: stale-node races (-> resolve+describe retried as a unit, plus
`no match` vs detached distinction), Control-after-locate ordering (-> Control
down first + wait for the swapped handle), inlined-fixture paren imbalance
(-> node --check on rendered output), zero-area fixture with UA border/padding
(-> border:0;padding:0), deliberate-rejection allowlist in final error scan,
hit-test flake on a re-rendered toolbar (-> re-resolve every 100 ms poll;
hit-test now names the covering element).

PARITY (same source/build, `web/target/site` 04:36Z): Rust passes all 15 checks
when the app cooperates (full PASS receipts verified by the launcher). The old
runner deterministically FAILS Part-drag Undo (`undone.signature !=
before.signature`). Root cause (byte-exact, escalation-ready): the Undo save
(rev N+1 = restored content, proven by waitDocument matching) is then clobbered
by a STALE save carrying rev N / dragged content (final doc byte-equals the
after-drag doc; revision regresses N+1 -> N). The same stale-save Undo race was
also observed for Matrix move on healthy state. Separately, drag sequences in
BOTH runners trigger a WASM `unreachable` panic at timing-variable points
(after Matrix move in Rust flow; around Ctrl-offset/matrix-select in old flow;
matrix-move-alone on healthy state is clean; pageerror: `unreachable`), which
kills the app and leaves the `#__dx-toast` panic toast (max z-index,
still hit-testable) covering the toolbar — the harness now fails fast with
`the app panicked: ...` instead of burning the 60 s click bound. Both defects
are app-side (harness implements the contract exactly; the panic also fires
under the reference Playwright driver) and are escalated separately — NO app
code was changed here, and the ticket is NOT resolved by matching failures.

Gate status: `check.py lint` passes (incl. harness entries). Full gate blocks at
the drag smoke on the app defects above (old default fails Part Undo; Rust fails
fast on the panic toast when it fires). Parity gate (matching old/new verdicts)
fails: old fails Part Undo deterministically while Rust passes it — opposite
verdicts on the same build, explained by the race interleaving, not by port
infidelity (identical centers/coords/params verified: Part center 606.50,583.64
both runners; down/move records byte-identical modulo capture retargeting).

Open: app-bug escalation (stale-save Undo race + `unreachable` panic);
parity re-run after app fix; CI wiring of the Rust path only after parity truly
passes; two independent reviews (Standards + Spec vs d800bff12) still to run.
