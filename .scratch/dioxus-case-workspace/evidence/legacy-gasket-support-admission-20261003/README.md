# Legacy gasket support admission correction

Source fix: `3e8aac8e57eb8bcef62ef24f902e1fb7a2b7789f`, based on integration `05c0f1373f6a61653ebe55ff7994a1155a98a84a`.

The current accepted Case projection can contain gasket supports from either Core provider. The private `SetGasketSupportDimension` handler rejected legacy configurations solely because `internalGasket` was absent. Pinned React `5a472a9426e6e38993361da402cd4ec730feb369`, `MechanicalAssemblyPanel.tsx` selected-support update, does not impose that predicate. The repair removes only that predicate. Gasket mount, numeric/payload validation, accepted owner/scope/revision guards and the existing canonical/physical-instance persistence route remain intact.

The focused browser-WASM regression invokes the production `apply_patch` with a legacy Gasket configuration and two saved linked anchors. It asserts complete configuration equality except the requested anchor lengths; it does not mount the Editor, execute a Core provider, or claim rendered support changes. Restoring only the former internal-only predicate fails at the operation's expected success assertion with `The current configuration no longer contains gasket supports.` The restored-source run passes 1/1 (exit 0); the old predicate run fails 0/1 (exit 1).

Core `mechanical/gasket.rs` honors saved anchor ID, region ID, outline key, perimeter position and unlink state. Its legacy provider still uses global layout length/width for resolved geometry, unlike the internal provider's per-anchor overrides. The reference UI persists per-support metadata on this route as well. This repair preserves that existing limitation, recorded under RF-006, and introduces no engine geometry semantics.

Command (same filter on old predicate and restored source):

```sh
CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/f73b-layout-layer-controls-20261002/web/target wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- presentation::mechanical_settings_controller::battery_patch_tests::legacy_gasket_support_resize_preserves_saved_anchor_metadata
```

`cargo fmt --manifest-path web/Cargo.toml -- --check` and source diff whitespace checks pass. No broad suite or fresh strict Clippy was run for this one-predicate repair; root qualifies the combined candidate.

Separate browser diagnosis: the Case author's imported original 5b archive after physical-instance Configure→Gasket contains `internalGasket` and `gasketLayout` in the selected left instance and shared construction; top-level legacy mechanical is intentionally retained. Thus that persistence path is sound. The current zero-layer/zero-diagnostic result is separate: CAD preparation can discard blocked mechanical resolutions before they enter the exact CadScene, while the Inspector derives diagnostics from that scene. The current read-only mechanical resolver already returns blocked assemblies. This trace is a diagnostic direction, not a proven corrected production path.

Current/legacy contextual browser edit/history/reopen and full F7.2/F7.4/F7.3/F7.8 acceptance remain open. No parent status changes are included.

Frozen regression source: `e4a89812f0bd7d57c5b44598dbec718f3659240e`. Production fix and regression files were unchanged during the green run. Exact raw output is retained as deterministic gzip; decompressed SHA-256:

- `expected-red.log`: `5894def8b6b53edd4f7975a97687cc6815eed5968e675abff52344e3c155191e`
- `expected-red.log.gz`: `129d77316cd103cfe8af0c441ef3aac9eab4b3f4d62fe1fa5e1b3b320065ae7e`
- `fixed-green.log`: `ae7a3106e8bb0bc24218a906f1679cbfd3ea0fdd64414ea5b12729bba00de3c7`
- `fixed-green.log.gz`: `77c7e0c73e17affc1b3a568d775831cc21ebd407c3c20cfefac8dafb732c9dcb`
