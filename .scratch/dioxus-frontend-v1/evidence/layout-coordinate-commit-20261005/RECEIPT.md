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

## Published history GREEN

Immutable frontend-layout-coordinate-commit-20261005/source462ec6afa4a85e58a023f8b7a453a877fe798930, at34828, repeated actual exact r17 placed-J1 input with Objects tree selection, rapidX60Enter thenY-40Enter, ProjectSave, twoUndos and ProjectSave. Real downloaded edited copy is r19/60,-40; after exactly twoUndos the saved copy is r21/66.675,-47.625. Full accepted input document restored exactly except expected revision progression; sixassetbytes unchanged. Retained same-action reference aftertwoUndos is also r21/originalcoordinates; only four known1e-14pose serializations differ. public-fixed.json and public-fixed-comparison.json retain actual actions, download names/bytes/hashes and every reference difference. Candidate r17→r19→r21 now matches reference; original phantom history RED remains retained.

All4 exact gate receipts reused at commit (integration-commit.json/log), strict17tests nofailures/exclusions/incomplete. Package proof validates source1409entries/root+subpath190assets each, no mismatches/warnings. Completed F8 output/roundtrip branches at4c retain originalsource/revision attribution; the fix changes only component Inspector submission and its regression test, not exporter/runtime/session owners. No full journey rerun required by sole independent review.

## Required untouched-field correction

The current462 candidate and pinned reference both imported exact r17 fixture SHA0215c9153cf4875bfd1f764136782eb2be15d555b3a625d3112eb12af2c602d6. Nonmutating J1 selection followed by X Shift, Y Shift and Project Save-copy sent no typed input. Actual candidate download Sofle v2(34) rounds accepted66.675,-47.625 to66.67,-47.62 and increasesrevision17→19. Reference Sofle v2(35) also adds two revisions and rounds to66.67,-47.63. Allsixassets unchanged. untouched-blur-public.json and untouched-blur-comparison.json retain exact actions and every raw document difference.

This is a shared reference defect with an additional candidate rounding discrepancy. The deliberate necessary Dioxus behavior correction is to preserve accepted coordinates/history when there is no user edit, including a cancelled draft; copying the reference's accidental edit would violate functional cancellation/data preservation. Same bounded two-file repair and reviewer retained. No new geometry engine/API or full Export replay is needed. Final recovery acceptance remains on hold until mounted regression, combined gates and published changed-branch proof.

## Focus-only blur precision follow-up

The additional mounted regression `mounted_component_position_untouched_blur_and_escape_cancel_submit_no_edit` starts from X=66.675/Y=-47.625, focuses/blurs both fields without editing, then changes X and presses Escape before blur. Its RED log `untouched-blur-focused-red.log` failed as expected with three captured edits: unchanged X rounded to 66.67, unchanged Y rounded to -47.62, and the Escape-restored X rounded to 66.67. This is a command-boundary regression; the retained public candidate/reference archives `candidate-untouched-blur.boardstudio` and `reference-untouched-blur.boardstudio` provide paired persistence evidence.

The minimal guard skips Blur submission when the draft text is still the two-decimal formatted accepted coordinate for that axis. Enter remains explicit, and a draft that differs from the accepted display still uses the existing blur commit path. Existing tests for changed blur against the latest owner and rapid Enter/focus change remain for the coordinator's affected-module gate.

Focused GREEN command was the same command/filter as RED and passed 1/1 with 337 filtered. Raw RED and GREEN runner output are retained at `untouched-blur-focused-red.log` and `untouched-blur-focused-green.log`.

Frozen source SHA-256:

- `web/src/presentation/inspector/layout_component_inspector.rs`: `15ff2d996e91be39b19e6cbdd00d3e51b68bf3cb3f7440cd6c052ce3893b64ec`
- `web/src/presentation/layout_component_inspector_tests.rs`: `bf6bb1565db10f3b24ef8a5953275feedec6cae4b3465b079156b217d260dd14`

No further source edits or focused tests are planned here; the coordinator owns the combined gate.

Coordinator combined untouched correction gates passed: native247/1ignored, wasm page, reachability, strictheadless18 with0failed/incomplete/excluded. Four exact receipt identities in untouched-combined-gates.json. Independent reviewer CLEAR at frozen inspector15ff2d99/testsbf6bb156. Source staged before gates for reuse. Immutable publication and actual downloaded nonmutation replay remain.
