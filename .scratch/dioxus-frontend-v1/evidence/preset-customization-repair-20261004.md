# F4.6 preset customization route repair

The paired F4.6-C01 browser journey found that the Dioxus Parts inspector could
place the selected MX Hotswap RGB preset but did not expose **Customize 3D
assembly**. Ordinary **New assembly** correctly opened an empty draft and remains
unchanged.

The saved-assembly editor now mounts **Customize 3D assembly** when a key
assembly is selected. The action reads the selected preset and switch orientation
at click time, uses library catalogue definitions, and admits only the current
ready project/scope while no save or apply is pending. It constructs an
independent unsaved draft (`base: None`) and leaves the existing Save action to
commit through the established document edit/history path. It does not change
the selected preset, library definitions, accepted document, or placed snapshots.

Preset preview and customization share `assembly_presets::recipe_members`.
Customization uses the reference's default single-sided construction and current
orientation; north orientation rotates every member. The native pure conversion
in `parts_assembly_preset_draft::from_recipe` retains each recipe member ID,
definition ID, parameter map, side, and pose. It marks model selection as
`Defaults` with no per-assembly model overrides, matching the reference preset's
empty `models` arrays while keeping definition-provided default models available.

## Regression and checks

The native regression first ran against a compiling empty conversion stub and
failed at the actual assembly member assertion: `left: []`, while the expected
state contained switch, diode, and LED members with their exact MX Hotswap RGB
parameters, front/back sides, poses, and default-model policy. This establishes
the conversion gap; it does not retroactively detect the absent browser button.

Focused native command and final result:

```text
python3 scripts/migration-deliver.py focused-test -- cargo test --manifest-path web/Cargo.toml --features page --bin boardstudio-web parts_assembly_preset_draft::tests::resolved_mx_hotswap_rgb_recipe_becomes_an_independent_editable_assembly -- --exact
running 1 test
test ...::resolved_mx_hotswap_rgb_recipe_becomes_an_independent_editable_assembly ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 206 filtered out
```

The first package-wide focused invocation omitted `--bin boardstudio-web` and
therefore compiled unrelated existing integration targets. It stopped on their
known missing Dioxus Props fields in `matrix_field_lifecycle`,
`parts_standard_profile_editor`, and `keycaps_fit_lifecycle`; the scoped binary
unit test above compiles and executes independently.

`cargo fmt --manifest-path web/Cargo.toml -- --check` and `git diff --check` on
the five leased Rust files passed. The wasm `page` compiler check and candidate
browser replay remain with the coordinator; no package was built here.
