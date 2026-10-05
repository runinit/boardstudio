# Generated keycap pick repair

Required functional repair only. Actual paired browser defect and same-pixel falsifier are in `RECEIPT.md`: candidate generated caps in Keymap/Keycaps intercepted the pick but produced no selected key; hiding that cap made the same pixel select the correct underlying PCB part. Reference generated-cap picks selected the key.

## Owner and executed regression

`LayoutCanonicalViewer` handled module IDs and PCB model references only. The renderer emits `keycap:<spec.id>` and `keycap-legend:<spec.id>` for generated bodies, neither of which is a PCB model reference.

The single added native regression obtains actual selected-board and foreign-board KeycapSpecs through CoreEngine::ResolveKeycaps on an accepted document. It passes the represented generated body IDs to the same native-owned pick resolver. For RED, the resolver’s existing policy was unchanged: an optional metadata input/signature was added without using it, and existing callers supplied None. The PCB reference still resolved part-1; the first generated cap ID failed with None versus Some(part-1). `keycap-pick-native-red.log`: **0 passed / 1 failed**, expected assertion at the generated-cap mapping, not compilation/setup; wrapper duration6.42s. Exact RED source hashes are in `keycap-pick-red-source.json`.

The repair extends this resolver only for cap/legend IDs. It requires an exact single rendered body ID and matching unique accepted spec, maps spec.id to a unique accepted part on the captured board, and rejects retired Layout leases or mismatched accepted scope/token/revision/document/scene/source generation. Keycap scope/token/revision and current-vs-rendered nonzero generation must also match. The consumer captures its supplied mesh generation, reads the current preview at event time, and passes its actual body IDs/specs to this resolver before the unchanged selection submission. PCB references still use the existing mapping; mounted-module picks retain their existing branch. No Runtime, shared viewer, renderer or generator changes were made for this repair.

The regression covers cap and legend success, absent body, absent spec, old mesh generation, wrong preview token/revision, a real Core spec from another board, wrong Layout generation/instance, and retired lease. It proves selection mapping/admission from represented mesh IDs and actual Core specs, not native CAD execution or a physical browser click.

## Final checks and integration

`python3 scripts/migration-deliver.py focused-test -- cargo test --manifest-path web/Cargo.toml --locked --no-default-features --features page --bin boardstudio-web layout_viewer_source::tests:: -- --nocapture` passed **12/12**, including existing authored/imported board, module, async source and model-delivery ownership tests. Raw log `keycap-pick-native-green.log`; wrapper duration2.40s. Scoped rustfmt and git diff --check passed. Native reachability is the existing main.rs path declaration; no new harness was introduced.

Both files are frozen and the compiler is released. Final hashes: 

- `web/src/presentation/layout_viewer_source.rs`: `fcaba6be9b7c6e13d1b7af9709fd927855614afe8356772e5c2afc5bd42b7bff`
- `web/src/presentation/layout_viewer.rs`: `90f8d0f9f28eea0b15416264dd085eb21de7a8d66b2e3ce12e4f2e487f11c0bf`

The mandatory combined gate should run the full existing `presentation::layout_viewer::mounted_keycaps_preview_tests::` module: `finding_focus_maps_outline_and_preserves_part_ids_for_current_board` and `mounted_preview_failure_retries_and_ignores_a_late_old_owner_reply`. The native source module has no WASM tests and runs in the existing mandatory native page target. The coordinator retains wasm-page/reachability/gate/config ownership. This author did not duplicate that pending WASM gate.

Same independent reviewer has the exact source packet. Final packaged Keymap and Keycaps cap/legend selection remains required; no browser or parent-acceptance pass is inferred from native GREEN.
