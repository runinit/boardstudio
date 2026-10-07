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

## Proposed spec for human approval

This proposal preserves current Case behavior, including generated PCB clearance
parts; it is not approved for implementation yet.

- Add one `ApplyMechanicalPatch` Core operation with an explicit board ID, optional
  physical-instance ID, the typed patch, and a normalized mounting-hole catalogue
  definition supplied by the existing adapter. Core validates that template's source
  identity before using it. The template is input data, not a prepared document.
- Move the patch vocabulary, dependent field enums, defaults, normalization and
  profile-target validation into Core. Include the relevant effective-configuration
  rules currently in host Case settings and CAD input projection: shared construction,
  instance settings, board thickness, wireless battery defaults and flipped instances.
  Preserve existing constants, family inference, explicit empty mount collections,
  errors and installation behavior.
- At execution, Core uses its accepted document and outline cache to initialize absent
  closure mounts through the existing mechanical resolver. Mirror physical-instance
  contours consistently with the current host projection. Core also owns the existing
  closure-clearance projection, including generated definitions, parts, references and
  board/layout memberships. The whole patch and projection form one Undo step.
- Return the board owner and only durable entity IDs whose values or memberships
  actually changed: affected physical instances, generated parts and definitions,
  and relevant membership owners. Preserve unrelated settings and parts. Pure settings
  changes do not rebuild board outlines; projection changes are classified according
  to their effect on outline inputs, with regression coverage and a code comment.
- Equal results remain Unchanged with no history/revision step. Reuse Core evaluation
  for resolver no-op detection rather than duplicating patch rules. Vanished or
  ineligible targets retire; invalid submitted values become Core edit errors.
- Preserve controller ports and `flush_prepared` ordering. Move domain-rule tests to
  Core, retain controller lifecycle/queue coverage, and add atomic projection, flipped
  instance, unrelated-edit queueing, no-op and Undo regression tests.

The scope therefore also includes the relevant host Case defaults/projection helpers
and `case/closure_clearance.rs`, beyond the original controller-only file list.
Implementation waits for approval and for module-deepening 09 to merge.

The alternative is a smaller settings-only operation with a separate projection
follow-up. That would leave the current whole-document clearance mutation in web and
requires an explicit scope exception to this ticket's Core-ownership goal.
