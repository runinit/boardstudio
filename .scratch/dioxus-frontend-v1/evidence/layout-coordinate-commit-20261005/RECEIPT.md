# Consecutive coordinate edits create an extra Undo entry — public RED

Current immutable candidate frontend-layout-point-precision-20261005/source4c0a2fc4 at34827, TypeScript5a472a9 at5175. Both imported the exact saved probe-01-placement.boardstudio (revision17,J1 X66.675/Y-47.625). Candidate selected J1 from the Objects tree, reference selected the actual SVG rolebutton with Enter; neither selection moved the part. Both filled X60/Enter then immediately Y-40/Enter using the actual controls.

Candidate actual saved probe-04-consecutive-xy.boardstudio is revision20 at60,-40; reference settled DOM revision19 at60,-40. With saved checkpoints between edits, candidate placement17→X18→Y19 agrees with reference; this isolates the extra commit to consecutive field editing before accepted projection settlement, not placement or Case configuration.

After one Undo both return Y to -47.625. After two Undo the candidate actual saved probe-05-two-undos.boardstudio is revision22 and stillX60/Y-47.625; the reference actual saved reference-two-undos.boardstudio is revision21 andX66.675/Y-47.625. Candidate third Undo finally restores X (DOM). Thus the issue is an extra history entry, not merely differing revision counters. coordinate-undo-probe.json retains DOM observations and archive paths. Earlier six-workbench outputs remain attributable to their original revisions24/23 and final geometry, with this history contradiction explicitly open.

Author layout_escape_repair owns only inspector/layout_component_inspector.rs and layout_component_inspector_tests.rs plus its regression evidence here; meaningful owning RED precedes source fix. No broader pending-operation matrix or optional tooling is required. Independent reviewer preset_batch_review retains the same consolidated batch review. F3.7-C02/parent are corrected to missing/implementing with preserved decision history; F8 waits the required repair while completed output/roundtrip branches remain retained.

## Owning mounted regression and repair

The regression `mounted_rapid_xy_enter_and_focus_change_submit_one_edit_per_axis` uses the mounted production `LayoutComponentInspector`, its actual X/Y inputs, Enter keydowns, and the X-to-Y focus change. The Runtime test observer records submitted application edits but does not apply them to an accepted document, so this test asserts command count, axes, and values; public saved archives above remain the persistence and Undo proof.

Before the fix, the mounted RED failed because three commands were observed instead of two: X=60 was submitted twice (Enter then blur), followed by Y=-40. The retained raw output is `focused-red.log`. After the fix, the exact focused command below passed 1/1 with 336 tests filtered:

```sh
python3 scripts/migration-deliver.py focused-test -- wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- mounted_rapid_xy_enter_and_focus_change_submit_one_edit_per_axis
```

The repair suppresses only the blur paired with an identical Enter for the same selected Inspector owner and coordinate axis/value. A new input clears that pairing, explicit Enter still submits again (including retry after rejection), and a blur with another owner/value follows the existing commit path. Existing transaction rejection and context-reset behavior were not changed. GREEN output was observed in tool session 1026; its terminal summary was `test presentation::layout_component_inspector_tests::mounted_rapid_xy_enter_and_focus_change_submit_one_edit_per_axis ... ok`, `test result: ok. 1 passed; 0 failed; 0 ignored; 336 filtered out`, wrapper `focused-test status=passed`.

Frozen source SHA-256:

- `web/src/presentation/inspector/layout_component_inspector.rs`: `6792aeb06e6f2feb8bcf7ed1b7e00b1a02997847140828a5995c36cf98147c76`
- `web/src/presentation/layout_component_inspector_tests.rs`: `12871c23ebebadf92ed7084a34c3b89cece04e9433b179be3311303a0010f0a4`

The coordinator's combined native/page/reachability/affected-headless gate remains responsible for full affected-module coverage; no duplicate module run was started here.

Coordinator combined gate terminal0: native247pass/1ignored, wasm page, reachability and strictheadless17pass with0failed/incomplete/excluded. Exact receipts retained in combined-gates.json. Source staged before gate; no source bytes changed after independent review. Public saved-history GREEN remains pending on the next immutable package.
