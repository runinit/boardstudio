# 02: Mechanical settings patches become a typed Core intent

Status: needs-info
Type: build
Blocked by: 01
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Source: [module deepening](../../module-deepening/map.md), [review candidate 04](../../../investigations/architecture-review-2026-10-07.html#c4)

Specify the operation shape once ticket 01 has settled the pattern; until then this
records what is known.

## What to build

The Case crate's mechanical settings resolver is a web-side domain engine that answers
with `ReplaceDocument` (line numbers at `915d305c0`, orientation only):

- `MechanicalSettingsPatch` (`web/crates/case/src/mechanical_settings.rs:451`) names
  each user change (enable, closure preset/drive/installation/thread/lengths, critical
  fits, battery, processes and so on);
- `prepare_document` (`web/crates/case/src/mechanical_settings_controller.rs:400`) and
  `apply_patch` (`:767`, ~1,060 lines) apply it to a cloned document;
- defaults and normalisation live only in web (`normalize_processes` `:2090`,
  `default_process` `:2148`, `default_plate_thickness` `:2316`,
  `default_plate_foam_thickness` `:2339`, `default_gasket_layout` `:2347`,
  `default_internal_gasket` `:2362`), and two copy Core: `default_material` (`:2237`,
  Core `core/src/mechanical.rs:72`) and `mounting_datum` (`:2331`, Core
  `switch_mounting_datum` `:18`).

Move the patch into Core as a typed intent (for example
`EditOperation::ApplyMechanicalPatch { patch }`, or one operation per patch family if
ticket 01's pattern prefers narrow operations). Core owns the defaults and validation
once; the web resolver translates the patch and retires only for an ineligible target.

## Acceptance criteria (draft)

- [ ] One copy of each mechanical default, in Core; the web copies are deleted.
- [ ] `battery_patch_tests` (`mechanical_settings_controller.rs:~2460-3631`) move to Core
  as tests of the operation, next to the rules they cover.
- [ ] Changed IDs are precise; the operation is classified for `affects_outline` with
  its reasoning in the doc comment.
- [ ] `MechanicalSettingsController`'s shell (ports, `flush_prepared` ordering) is
  unchanged and its native tests pass.
- [ ] The resolver inventory row for the mechanical settings resolver is updated.

## Verification

```sh
cargo test -p boardstudio_core -p boardstudio-application --locked
cargo test -p boardstudio-web-case --locked
python3 scripts/check.py lint typecheck test
wasm-pack test --headless --chrome web/crates/case --locked --lib
```
