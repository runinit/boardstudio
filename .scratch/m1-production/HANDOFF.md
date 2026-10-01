# M1 candidate handoff

Branch: `codex/m1-production-20261001`. Runtime source: `2040e23b`; build correction: `fe2ada03`.
The [to-spec plan](PLAN.md), [specification](spec.md) and six local tickets
define the implemented scope. The [canonical run state](../../docs/migration/m1-production-run.json)
and [acceptance ledger](ACCEPTANCE.md) govern acceptance.

## Editable copied-project trial

The complete candidate is being staged at
`web/target/builds/m1-release-20261001-fe2ada03`. Use it only after its
`provenance.json` records all 18 commands exiting zero, both prefix asset maps,
and the final source check. Build outputs and logs are retained locally; the
portable release record will retain their identities.

To rebuild, run `python3 scripts/build-m1.py <new-unique-id>` from this checkout.
The maintained builder refuses to overwrite earlier outputs. To serve a
completed build at root and subpath on the same owned local origin:

```sh
node .scratch/m1-production/evidence/browser-storage/serve-release.mjs \
  web/target/builds/m1-release-20261001-fe2ada03 0
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
revision rejection have separate retained evidence. Current-release checks and
performance results are recorded separately as they finish.

The canvas includes a nominal PCB contour, without populated PCB parity. CAD
revisions above 9,007,199,254,740,991 are rejected before conversion. Context loss
stops rendering; reopening/remounting the preview recovers it. Screen-reader
interaction is blocked on this host because no screen reader is installed.
Axe and contrast checks do not substitute for that gate. Full resource and
eligible performance comparisons remain required where the ledger says open.

The original checkout, stashes and historical worktrees are retained. Worker
worktrees also retain ignored build/evidence inputs, so cleanup must preserve
those before archiving them. This candidate does not authorize a production
database cutover, push, deployment, main merge or React retirement.
