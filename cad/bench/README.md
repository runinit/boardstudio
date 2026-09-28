# CAD benchmark

Build the app and WASM artifacts, then run `pnpm run bench:cad -- --output cad/bench/results/latest` followed by `node cad/bench/compare.mjs cad/bench/results/latest`.

The runner is `app/scripts/run-cad-benchmark.mjs` and serves production `app/bench-cad.html`. It measures core preparation, the CAD worker, scene worker, renderer upload, and paint opportunity across five fresh sessions by default. `--sessions 1 --samples 1` is only a smoke run.

Assembly fixtures cover preview, export, cache, edit, cancellation, and queued-edit scenarios. Imported STEP fixtures run only `cold` and `warm-uncached` through production `requestModel` and render on the same canvas. STEP-only export experiments are excluded.

Reports record source/WASM hashes, host details, viewport, kernel, actual browser version, request metrics, RSS, and compressed raw sessions. Browser version is provenance only; it is not pinned or a sole acceptance gate. Busy-host diagnostics are marked ineligible.

`cad/bench/results/baseline` is historical provenance. Use a fresh output directory for new measurements. Frozen budgets are in `budgets.json`; comparison checks raw sample counts, minimum baseline session/sample coverage, fixture hashes, completion/paint/RSS budgets, scenario completeness, and CPU/OS plus core/renderer WASM identity. A smoke run cannot establish budget acceptance.

To compare with a fresh reference on the current renderer/core, pass its directory
as the second argument: `node cad/bench/compare.mjs <candidate> <reference>`.
This preserves the historical reference, frozen budgets and minimum sampling
requirements; a new reference cannot lower them.

The separate `editing-soak` scenario keeps one worker across repeated exact
edits, returns to original geometry, board changes, superseded drafts and STEP
exports. For example, run `--sessions 1 --samples 240 --scenarios editing-soak
--fixtures boss-tray,gasketed-pair --detailed-memory --output <fresh-directory>`.
Interpret its RSS/allocated-WASM trend separately from acceptance timings and
from full-workbench interaction coverage.

Run fixture and CAD correctness checks with `pnpm --dir cad test`.

For unresolved timing tails, `--host-observations` adds one-second CPU/memory
pressure, swap/reclaim counters, reported CPU frequencies, and sampled process
CPU ticks to the raw session reports. It always marks the run ineligible for
acceptance; instrumentation is diagnostic, and does not replace the frozen gate.

`node app/scripts/diagnose-cad-exports.mjs <control-wasm-package> <fresh-output>`
alternates five current/control pairs for the split export scenarios. Each uses
a fresh browser, the unchanged production JavaScript and 20 samples per export
scenario. It rejects mismatched JS bindings and stale production WASM, records
both binary hashes and host observations, and never establishes full acceptance.
The control package must be built separately; the script does not change source
or production assets. This focused diagnostic uses one-second host sampling,
not the full runner's 10 ms RSS sampling, so compare variants within the paired
run rather than substituting its timings for the full-run result.
