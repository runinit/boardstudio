# M1 candidate handoff

Branch: `codex/m1-production-20261001`. Runtime decoder source: `5a0972a2`; pointer repairs: `e7d29ce6`; build correction: `fe2ada03`; complete release source: `b9748745`.
The [to-spec plan](PLAN.md), [specification](spec.md) and six local tickets
define the required scope; acceptance remains open. The [canonical run state](../../docs/migration/m1-production-run.json)
and [acceptance ledger](ACCEPTANCE.md) govern acceptance.

## Editable copied-project trial

The completed candidate is at `web/target/builds/m1-release-20261001-b9748745`.
All 18 build commands exited zero, all 925 source hashes verified, and each
prefix contains 47 verified files and one current page WASM. See the
[portable provenance](evidence/integration/release-b9748745-provenance.json).
The owned editable demo is running at [root](http://127.0.0.1:46890/) and
[/boardstudio/](http://127.0.0.1:46890/boardstudio/). The [b974 browser smoke](evidence/integration/final-browser-smoke-b9748745.md)
passed root and subpath offline library reload, copied-project editing, Sofle STEP
download, and disposal on switching to REVIUNG41. Earlier broader
[8f browser checks](evidence/browser-storage/release-8f509433-final-qa.md)
remain evidence only for their recorded source and actions. Saved-project startup
restoration and a physical-instance selector are being completed after a final
story audit; this release does not yet provide those two flows.

To rebuild, run `python3 scripts/build-m1.py <new-unique-id>` from this checkout.
The maintained builder refuses to overwrite earlier outputs. To serve a
completed build at root and subpath on the same owned local origin:

```sh
node .scratch/m1-production/evidence/browser-storage/serve-release.mjs \
  web/target/builds/m1-release-20261001-b9748745 0
```

Use the printed origin at `/` and `/boardstudio/`. Open **REVIUNG41 copy** or
**Sofle v2 copy**. Select a part, preview and commit its numeric position or drag
it, then Undo/Redo and reload. Export and reimport an archive. For REVIUNG41,
choose **Add case settings** before **Generate case**; Sofle Left already has
settings. Generate, inspect the 3D view and export STEP. Changing a board clears
the previous case. Root and subpath use separate trial databases and caches.
The original `boardstudio-v2` database remains the reference.

## Verified source and limits

Thirteen public session and twelve native web tests pass. Formatting and strict
application/page/CAD-worker Clippy pass. Standards and Spec reviews have no
blocking findings on the unchanged production tree. Both real fixture editors
generated exact cases; high-DPI resize, scope remount, cancellation and retry
passed on the focused artifact. Durable save abort/retry, faithful assets and
archive exchange, case input comparisons, independent STEP readback and wide
revision rejection have separate retained evidence. Final-release browser checks pass at both prefixes. Performance results and
remaining resource/semantic checks are recorded separately as they finish.

The canvas includes a nominal PCB contour, without populated PCB parity. CAD
revisions above 9,007,199,254,740,991 are rejected before conversion. Context loss
stops rendering; reopening/remounting the preview recovers it. Screen-reader
interaction is blocked on this host because no screen reader is installed.
Axe and contrast checks do not substitute for that gate. Scoped resource and same-input geometry evidence is recorded; exact STEP bytes
differ only at tiny decimal coefficient rounding. One focused pointer session
passes all caps; final rebuilt-release five-session timing remains required. The unchanged
[reference run](evidence/performance/runs/reference-gates-20261002T002536Z/assessment.md)
completed with UI/live timing failures and an ineligible frozen CAD comparison
(OS and provider identities differ); no budgets were changed. Actual root/tab
close and complete material parity still lack direct browser evidence.

The original checkout, stashes and historical worktrees are retained. Worker
worktrees also retain ignored build/evidence inputs, so cleanup must preserve
those before archiving them. This candidate does not authorize a production
database cutover, push, deployment, main merge or React retirement.
