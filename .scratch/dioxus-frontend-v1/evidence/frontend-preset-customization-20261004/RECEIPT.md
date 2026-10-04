# Preset customization desktop continuation

## Source and gates

Reviewed source commit `4e716156` adds the previously absent Parts preset Customize 3D assembly entrypoint. Independent exact-file source review: ../preset-customization-review-20261004.md. Native conversion regression ran RED then GREEN; required guarded commit passed native web tests, wasm page check, wasm test reachability and affected headless-Chrome modules (commit-gate.log). Source success does not establish browser acceptance.

Provider-reuse preflight rejected an existing inline test module in included renderer_host_page_base.rs, before reserving an output directory. The normal full build is the fallback; no reuse guard or production source was changed to bypass it.

## Reference North oracle

On the retained reference root5175 project, Parts -> MX Hotswap RGB -> Switch orientation North-facing LED -> Customize 3D assembly. The draft opens MX HOTSWAP RGB with switch/diode/led. Public member poses: switch(0,0),180 degrees; diode(-7.4,1.5),270 degrees; led(0,4.75),0 degrees. Each uses component model defaults. The switch exposes checked Hotswap socket and Switch side Front. Toggle Hotswap socket off, Switch side Back, name `North preset regression20261004`, Save assembly. Reference reports `Assembly saved. Existing placements are unchanged.` and `Saved locally`, and lists the saved recipe. This is reference evidence only until candidate replay below.

## Candidate replay

Published full build frontend-preset-customization-20261004 at root34822 and /boardstudio/, source4e716156a0692972b1309798cdc883739c7ff73b. Extra-candidate reason was recorded; detached listener PID2239133. Publication verifies both routes and served assets against provenance.

The first same-origin navigation used the old cached offline shell: DOM script URL ended `dxh96997dca3fce4488.js`, matching the prior build. After background service-worker update, a second navigation used `dxh7a84b355b3d74c.js`, matching this build. Only the latter is candidate replay evidence; no source regression was inferred from the old shell.

Parts -> MX Hotswap RGB now exposes Customize 3D assembly. Set North-facing LED, click Customize: independent draft contains switch/diode/led with the same numeric North poses as the reference above. Zero renders as -0 on the candidate; this is a cosmetic difference, not a changed position. Each member selects Use component defaults. Switch starts Hotswap checked, Switch side Front. Turn Hotswap off, select Back, name North preset regression20261004, Save: saved locally and Assembly saved/Existing placements unchanged feedback appear.

Close editor, Undo: the new recipe disappears. Redo: it returns. Full reload and reopen preserve all three poses, model-default selections, switch Hotswap off and Back. Reference full reload/reopen retains the same parameter values and numeric poses. Ordinary New assembly still opens New assembly with zero member groups and Add component; close without saving.

Reuse earlier paired import, saved-recipe placement/history and identity proofs for unchanged paths. F4.6-C01 is qualified by this originating-action replay plus retained CRUD/custom-model evidence; C02 uses its separate import/validation receipts. No claim that native conversion alone proves mounting or every scope-race branch. Mobile checks remain deferred. Parent F4.6 stays open for prerequisite acceptance.
