# F3.5-C05 attachment replacement choices

The retained paired Layout receipt reproduces the gap on the saved PCB fixture:
the candidate offers 59 replacement options, the TypeScript reference offers 52,
and all seven extra candidate values are unrelated assembly snapshot definitions.
Both retain the attachment's currently assigned diode snapshot. See
`layout-context-followup-20261004/RECEIPT.md` and
`layout-criterion-reconciliation-20261004.md` for the exact browser journey and
the remaining criterion clauses.

`MatrixTransformFields::Key` now carries the deduplicated document/catalogue
definitions until each attachment row renders. The native-tested
`attachment_component_choices` policy is then applied per row: it preserves that
row's current assembly snapshot, excludes unrelated snapshots and the retired
`nice_nano_pretty` generator, keeps source-backed KiCad definitions, and retains
the input order and existing Rust labels. The existing Key Assembly filtering
policy was not changed. Its snapshot parser is WASM-only and narrower than the
reference policy, so this attachment-specific native seam uses a private copy of
the reference's case-insensitive ID rule plus the KiCad-source exception.

## Regression and checks

Native RED command before the policy implementation:

```text
python3 scripts/migration-deliver.py focused-test -- cargo test --manifest-path web/Cargo.toml --features page --lib matrix_transform_operation::tests::attached_component_choices_keep_only_this_members_snapshot_and_preserve_order -- --exact
test ...::attached_component_choices_keep_only_this_members_snapshot_and_preserve_order ... FAILED
assertion `left == right` failed
left: 59 options, including assembly-placement-0..6/definition/switch
right: 52 options, retaining the current diode snapshot
```

The same command after the fix passed: **1 passed, 0 failed, 38 filtered out**.
The fixture also asserts source-backed assembly definitions remain selectable,
retired generators remain excluded, and option order is unchanged.

`rustfmt --edition 2024` and `git diff --check` passed for the three owned source
files. The wasm page check and browser replay are left to the coordinator; this
packet did not build a package or repeat successful edit/history journeys.
