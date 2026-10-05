# F3.7-C02 outline point precision repair

The Outline canvas now ignores a pointer-up sample when the pointer ended at its pointer-down client position and no prior drag sample changed the point. A pointer-up at a displaced client position still samples and commits, including when no `pointermove` was delivered. Escape during an active drag still cancels through the existing path.

The regression is mounted through the production `OutlinePointCanvasOverlay`, with the test Runtime's existing event observer checking whether an edit was submitted. Synthetic PointerEvents require a test-only pointer-capture shim because the browser does not consider them active native pointers. The test Runtime records submitted events without applying them to its accepted document, so this is event-boundary verification. Public archives remain the evidence for persisted geometry.

Before the fix, the focused test failed with a Commit `SetOutline` event despite identical pointer-down/up coordinates; see [focused-red.log](focused-red.log). After the fix, [focused-green.log](focused-green.log) passed 1/1. The affected browser module passed 13/13 with zero exclusions; see [affected-module-results.json](affected-module-results.json) and [affected-module.log](affected-module.log).

The focused command was:

```text
python3 scripts/migration-deliver.py focused-test -- wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- mounted_fixed_outline_point_click_and_escape_preserve_fractional_geometry
```

The affected-module command was:

```text
python3 scripts/run-wasm-tests.py --files web/src/presentation/outline_lifecycle_browser_tests.rs --result-json .scratch/dioxus-frontend-v1/evidence/layout-point-precision-20261005/affected-module-results.json
```

Final source SHA-256:

```text
web/src/presentation/outline_lifecycle.rs          d88b54f350a4966c2a3197ed742e846982458055d77647c8478181dba6c7a6cd
web/src/presentation/outline_lifecycle_browser_tests.rs 2580227086b0e73d61e8bc8d2c5a66287b070eed19b365ddd5123a37686b130c
```
