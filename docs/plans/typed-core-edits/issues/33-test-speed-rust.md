# 33: Make the native tests faster (audit phase 2, part 1)

Status: claimed
Type: build
Blocked by: [24](24-test-audit.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Audit: [test-audit.md](../audit/test-audit.md)

## Request

User, 2026-10-10: implement all the audit's recommendations ([section 7 speed options and section 6 items 1, 3, 4](../audit/test-audit.md)).
The native `test` step is 74.5 s because the shipped-keyboard recipes are built twice in debug.

## What to do (measure each; keep only what is a net win)

- Share one build: `demo_projects::native_recipes_preserve_all_examples_and_embedded_model_bytes` and
  `included_keyboards::projects()` build the same recipes twice (55 s and about 50 s). Build once per test binary
  (one `OnceLock` shared through `core/tests/support`) and make `included_keyboards` read from it; build the
  reused-PCB project itself only where needed.
- Make `prepare()` cheaper: try per-fixture parallelism and a dev-profile `opt-level` for the core crate; keep a change only
  if `check.py test` and `build` both get faster and `lint` does not get slower. Record before/after numbers.
- `generator_export::every_bundled_generator_exports_a_board_kicad_parses` (15 s): one board containing all generators and one
  `kicad-cli` run that still reports which generator failed.
- `included_keyboards::closure_proposals_on_a_paired_half_...`: 9 projects down to 2 (the synthetic closure tests in
  `mechanical_integration` cover the rule).
- `kicad_parses` passes silently when `kicad-cli` is missing: fail with a clear message unless
  `BOARDSTUDIO_ALLOW_NO_KICAD=1` is set.
- Merge `source_right_splits_reproduce_the_shipped_left_half` into `every_shipped_split_mirror_follows_the_switch_and_companion_rules` and trim
  the Corne reproduction test only where an identical assertion survives (audit D1, D2).
- Do NOT delete the recipe-reflection tests (audit R1): they retire with ticket 23.

## Acceptance

- [ ] `check.py test` wall time before/after recorded (target: roughly 25-35 s, measured not assumed).
- [ ] Every removed or merged assertion names the surviving test; mutation-check that coverage of the behaviour survives.

## Verification

`repo lint test`; full before/after timing run of `test`.
