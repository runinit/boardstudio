# M1 candidate handoff

Branch: `codex/m1-production-20261001`. Runtime decoder source: `5a0972a2`; pointer repairs: `e7d29ce6`; build correction: `fe2ada03`; complete release source: `a49bb798`.
The [to-spec plan](PLAN.md), [specification](spec.md) and six local tickets
define the required scope; acceptance remains open. The [canonical run state](../../docs/migration/m1-production-run.json)
and [acceptance ledger](ACCEPTANCE.md) govern acceptance.

## Editable copied-project trial

The completed release artifact is at `web/target/builds/m1-release-20261002-a49bb798`.
All 18 build commands exited zero, all 925 source hashes verified, and each
prefix contains 47 verified files and one current page WASM. See the
[portable provenance](evidence/integration/release-a49bb798-provenance.json).
The owned editable demo is running at [root](http://127.0.0.1:34285/) and
[/boardstudio/](http://127.0.0.1:34285/boardstudio/).
[Final release QA](evidence/integration/release-a49bb798-qa/README.md) passes
root/subpath online and cached-offline active restoration, preference handling,
startup races, physical case generation and cancellation. The physical selector
uses the existing session navigation boundary; compact keyboard/axe checks pass
on the source-equivalent focused artifact. Broader storage/archive/STEP/renderer
records are reused only where their source inputs are unchanged. The final
[physical STEP check](evidence/integration/release-a49bb798-qa/physical-left-step-acceptance.md)
downloads the configured Sofle left-half case, revokes its URL, and prevents a
held physical export from delivering after navigation to Canonical.

To rebuild, run `python3 scripts/build-m1.py <new-unique-id>` from this checkout.
The maintained builder refuses to overwrite earlier outputs. To serve a
completed build at root and subpath on the same owned local origin:

```sh
node .scratch/m1-production/evidence/browser-storage/serve-release.mjs \
  web/target/builds/m1-release-20261002-a49bb798 0
```

Use the printed origin at `/` and `/boardstudio/`. Open **REVIUNG41 copy** or
**Sofle v2 copy**. Select a part, preview and commit its numeric position or drag
it, then Undo/Redo and reload. Export and reimport an archive. For REVIUNG41,
choose **Add case settings** before **Generate case**; canonical Sofle Left already
has settings. For its physical case, select Left PCB → **left half**, then **Add
case settings** and **Generate case**. Sofle Right PCB currently reports unready
input. Generate, inspect the 3D view and export STEP. Changing a board clears
the previous case. Root and subpath use separate trial databases and caches.
The original `boardstudio-v2` database remains the reference.

## Verified source and limits

Fourteen public session and twelve native web tests pass on the integrated story-completion candidate. Formatting and strict
application/page/CAD-worker Clippy pass. Standards and Spec reviews have no
blocking findings on the unchanged production tree. Both real fixture editors
generated exact cases; high-DPI resize, scope remount, cancellation and retry
passed on the focused artifact. Durable save abort/retry, faithful assets and
archive exchange, case input comparisons, independent STEP readback and wide
revision rejection have separate retained evidence. Current release browser checks pass at both prefixes; all five paired pointer sessions pass the unchanged caps. The
[raw timing summary](evidence/performance/runs/paired-pointer-a49bb798-20261002-retry1/summary.json)
retains 100 measured samples per scenario.

The canvas includes a nominal PCB contour, without populated PCB parity. CAD
revisions above 9,007,199,254,740,991 are rejected before conversion. Context loss
stops rendering; reopening/remounting the preview recovers it. Screen-reader
interaction is blocked on this host because no screen reader is installed.
Axe and contrast checks do not substitute for that gate. Scoped resource and same-input geometry evidence is recorded; exact STEP bytes
differ only at tiny decimal coefficient rounding. Final candidate median p95 values are 18.8/24.4/33.1 ms at 30/100/200 keys
against 33/50/100 ms caps. Ancillary startup visibility observations are incomplete
and do not establish a startup timing gate. The unchanged
[reference run](evidence/performance/runs/reference-gates-20261002T002536Z/assessment.md)
completed with UI/live timing failures and an ineligible frozen CAD comparison
(OS and provider identities differ); no budgets were changed. Actual app-tab close removes Core/CAD browser targets; Rust drop execution,
GPU memory accounting and complete material parity remain unobserved.

## Remaining acceptance work

Ticket 06 remains open. Run the documented screen-reader interactions on a host
with an actual screen reader. Diagnose the retained frozen UI/live timing failures
without changing the budgets, and repeat only affected gates after a repair.
Resolve the frozen CAD environment/provider mismatch before claiming a comparable
baseline result; its current per-row budget pass remains separate evidence.
Complete the required material attribution and direct renderer/GPU/drop accounting,
or obtain an explicit compatibility decision for a narrower acceptance contract.
The full matrix and retained failures are in [ACCEPTANCE.md](ACCEPTANCE.md).

The original checkout, stashes and historical worktrees are retained. Worker
worktrees also retain ignored build/evidence inputs, so cleanup must preserve
those before archiving them. This candidate does not authorize a production
database cutover, push, deployment, main merge or React retirement.
