# Unchanged reference gate assessment

Run record: `record.json`  
Production build: `release-b9748745.json` (`b9748745ec444416136a97d4befd075670ef49b7`)  
Run source checkout: `41b21c22ba65f73a5fa7d05a0396b3bce8706c54`

The UI/live command exited 1. Its complete raw output is in `reference-ui-live.log`, with machine-readable observations in `ui-live.json`. The workbench reference baseline failed its frozen worker thresholds for 100-single, 100-row, and 200-single; 200-row passed. The live-preview checks passed pointer submission and mount observations, but failed `mainWorkP95` and numeric, undo, and gasket p95 gates. These are unchanged-reference gate results, not candidate pointer comparisons.

The CAD benchmark completed five sessions successfully (`reference-cad.log`, `cad/session-1.json.gz` through `session-5.json.gz`). The frozen comparator exited 1 (`frozen-cad-compare.log`). Every scenario row passed its existing completion-p95, painted-p95, and RSS limits. The comparison is ineligible because `comparableEnvironment` is false: CPU matches, while the frozen baseline metadata has OS `linux 7.2.7-1-cachyos` and core/renderer WASM hashes `5b75a57506629078fc80e76171481388b54ae0524741e9d92520a81b556bdfdb` / `9890f03cf31e886074778a5ffa30f1d3d3e10b8ba9553fd00891c909514a5f51`; the current measured environment has OS `linux 7.2.8-1-cachyos` and hashes `bc3a091122da2ebcb4eb55d3f0967d2b1943611105068f287c2fc6f0e17aa186` / `f406725700e66afc3859e706d15726dd53bfcb10b1c50a15dff3f41212da17a8`. The run's current hashes match the b9748745 release. Scenario completeness, sample sufficiency, fixtures, and browser-version checks all matched.

All nine pinned input files are recorded in `record.json` and remained byte-identical at completion. Neither the failed UI/live gate nor the environment-ineligible CAD comparator establishes an M1 acceptance result.
