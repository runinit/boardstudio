# Test audit, phase 1 (inventory, timing, classification, proposal)

Ticket: [24](../issues/24-test-audit.md) · Map: [map.md](../map.md) · Status: **proposal only, nothing deleted,
edited or merged.** The user approves the action list in section 6 before phase 2 executes any of it.

Measured on 2026-10-10 at `92d08fbf2` (the `tce-matrix` worktree, branch `typed-core-edits/mechanical`), 16 CPUs,
`rustc 1.98.0` (stable). Per-test native times come from the libtest `--report-time --format json` flags, enabled
on a stable toolchain with `RUSTC_BOOTSTRAP=1` at run time only (no rebuild, no source change). Sandbox lifted
only for the CAD container, wasm-pack and Chrome. Runs were sequential and in the foreground. `check.py release`
was not run.

## 1. Headlines

- **About 2,150 tests**: 1,407 native Rust, 619 wasm/browser, 124 Python. Plus about 14 MB of goldens.
- **Time is concentrated in two fixture builds, not in the number of tests.** Of the 546 core integration tests
  (191 s of test body time run alone), **505 take under 0.5 s and 418 under 0.05 s**; everything outside the 30
  slowest sums to 25 s. Deleting small tests saves seconds. Deleting all 418 of the fastest saves 1.7 s, and all 505 under 0.5 s saves 18.7 s of CPU.
- The native `test` step is **74.5 s**. Its critical path is **one core integration binary at 66 to 70 s**, and that
  is two copies of the same work: the shipped-keyboard recipes are built **twice in debug**, once by
  `demo_projects::native_recipes_preserve_all_examples_and_embedded_model_bytes` (55 s of its 58 s is the
  `prepare()` call) and once by the shared `included_keyboards::projects()` `OnceLock` (about 50 s). Fifteen tests
  that touch `projects()` all wait for that one build, so their individual times look slow (51 s, 6 s) while their
  bodies are cheap. The `build` step runs the same recipes a third time (`prepare_demo_projects`).
- The `browser` step is **159 s** (commands run four at a time); the slow wasm suites spend their time in **fixed
  real-time sleeps** (`TimeoutFuture::new(50)` in loops of 15, `rendered()` of 60 ms), not in assertions.
- Real redundancy exists but is small and mostly in names rather than seconds: two near-copies of
  "the shipped split reproduces", keycode/delay twin tests, version-literal assertions in about 14 places, and two
  goldens that pin more than their purpose (a whole-document transcript, a whole-document baseline).
- Honest conclusion for the user: **the time win comes from speed-only changes (section 7); the deletion list
  (section 6) is about maintenance cost and version-bump churn**, and is short because most tests do guard a real
  behaviour.

## 2. Inventory

### Native Rust (`cargo test`)

| Binary / suite | Tests | Wall alone | Notes |
| --- | ---: | ---: | --- |
| `boardstudio-core` lib (unit) | 304 (3 ignored) | 0.46 s | `mechanical::*`, `artifact::*`, `migrate`, `archive`, `matrix::*`, `case`, `firmware` unit tests |
| `boardstudio-core` `integration` (45 modules) | 546 | 69.8 s (check.py), 190.8 s on one thread | table below |
| `boardstudio-core` `matrix_performance`, `performance` | 3 (all ignored) | 0 | run explicitly |
| `boardstudio-application` | 34 | 0.01 s | `durable_session` 30, `outline_queue_orders` 4 |
| `boardstudio-renderer` | 27 | 0.69 s | |
| web crates, native half: runtime 110, layout 78, parts 42, case 35, catalogue 24, pcb 23, host 17, ui-model 8, ui-shared 8, keymap 8, keycaps 7, library 5 | 365 | under 1 s in total | non-mounted logic; the same crates have a separate wasm half |
| `boardstudio-footprints` (lib 7 and 9 binaries) | 73 | 11.0 s | `generators.rs` 2 tests (4.8 s + 0.1 s alone; 9.9 s as reported) |
| CAD native (`cad/wasm`, container) | 55 (4 ignored benchmarks) | 23.4 s | 51 run |
| **Native total** | **1,407** | `test` step 74.5 s | the three commands run in parallel |

Core integration modules (count, source lines, time alone): `core_engine` 54 · `mechanical_integration` 50
(3,858 lines, 1.6 s) · `gasket_features` 32 · `mirrored_half` 30 (2,947 lines, 17.1 s) · `outlines` 29 ·
`mechanical_settings_edits` 27 · `key_move_assemblies` 23 · `kicad_integration` 23 (11.1 s, needs `kicad-cli`) ·
`generator_parameters` 22 (56.6 s, almost all the shared build) · `mounted_modules` 20 · `outline_edits` 18 ·
`encoder_inputs` 17 · `included_keyboards` 15 (1,066 lines, 11.7 s) · `matrix_placement` 14 (877 lines, 0.02 s) ·
`outline_linked_resolution` 13 · `mechanical_workflow` 12 · `electrical_wiring` 11 · `thqwgd001_library` 11 ·
`keymap_workflow` 10 · `generator_export` 8 (18.0 s) · `current_schema` 8 · `outline_repairs` 8 · fourteen
modules of 1 to 7 tests, `demo_projects` 1 (58.3 s). Only the modules above 1 s are worth a time
figure; all others are under 0.6 s each.

### wasm/browser (`check.py browser`)

| Suite | Tests | Suite wall (test run only) | Command wall |
| --- | ---: | ---: | ---: |
| `web/crates/layout` (two commands) | 167 | 13.7 s + 0.4 s | 28.6 s + 13.3 s |
| `web/crates/case` | 81 | 30.6 s | 44.2 s |
| presentation (`web`, 2 filters) | 70 | not printed | 48.4 s |
| `web/crates/parts` | 68 | 16.1 s | 29.9 s |
| `web/crates/pcb` | 49 | 26.1 s | 38.5 s |
| `web/crates/keymap` | 40 | **37.0 s** | 46.4 s |
| `web/crates/keycaps` | 30 | 2.6 s | 12.6 s |
| `web/crates/runtime` | 29 | 1.5 s | 11.6 s |
| `web/crates/ui-shared` (two commands) | 27 | 8.7 s + 0.1 s | 19.0 s + 10.1 s |
| `web/crates/library` | 16 | 11.1 s | 21.3 s |
| `web/crates/catalogue` | 12 | 0.25 s | 7.3 s |
| `web/crates/host` (`host::storage::`) | 10 | 0.5 s | 8.1 s |
| `web/crates/ui-model` | 5 | 0.01 s | 8.3 s |
| CAD browser smoke checks | 15 | 3.0 s | 3.0 s |
| **Total** | **619** | | step wall **159.1 s** (sum of command walls 337 s; four at a time) |

Each wasm-pack command carries 7 to 13 s of fixed start-up (page load, module instantiation) on top of the test
run, so the 14 browser launches spend about 130 s on start-up before any test runs. Merging launches (they are
split for isolation and for the 120 s runner bound) is a possible speed-only change, not measured here.

Per-test wasm times do not exist (wasm-bindgen-test prints none). Filtered runs of the slowest suites gave:
`keymap::queued_edit_tests` 15 tests in **27.7 s**; `keymap::macro_controller` 11 in 9.0 s;
`case mechanical_settings::` 28 in **24.0 s**; `pcb_module_inspector` 6 in 10.2 s; `parts_definition_name` 17 in
9.5 s; `ui-shared pending_edit_helpers` 16 in 8.5 s; `pcb_wiring` 21 in 7.2 s; `pcb_physical_setup` 9 in 6.7 s;
`case_controller` 7 in 2.2 s. The layout matrix toolbar suite (19 tests) runs in 1.0 s.

### Python tooling (`repo`, `tooling`, `security`)

121 tests in 12 files, 3.0 s in total (`repo` 0.1 s, `tooling` 2.9 s; `test-run-wasm-tests.py` 40,
`cad/scripts/test-cadrum-build.py` 21, `test-check.py` 14, `test-check-quick.py` 11, `test-catalogue-tools.py` 10,
`test-build-web.py` 7, `test-check-wasm-tests.py` 6, `test-check-doc-links.py` 4, `test-serve-web.py` 3,
`test-dev-web.py` 2, `test-stage-ergogen-models.py` 2, `test-import-kicad-parts.py` 1), plus 3 in
`security-audit.test.py` (not in the default steps). They guard the check tooling itself and are cheap. No change.

### Goldens and fixtures

| Fixture | Size | Read by | Churn this effort |
| --- | ---: | --- | --- |
| `core/tests/fixtures/boundary/transcript.json` | 1.1 MB, 67 core + 11 archive replies | `boundary_parity` (native, 0.3 s) and the wasm boundary parity run | 14 commits since 2026-10-01; two version bumps changed 120 lines each |
| `core/tests/fixtures/demo-projects-baseline.json` | 2.5 MB, 20 projects | `demo_projects` | 7 commits since 2026-10-01 |
| `core/tests/fixtures/matrix-projections.json` | 41 KB | `core/src/matrix.rs` unit test | 0 this month |
| `core/tests/fixtures/reviung41-outline-original.json` | 103 KB | `outlines` | version literal bumped twice |
| `core/tests/fixtures/internal-gasket-v1`, `mechanical` | 21 KB, 25 KB | gasket, mechanical | none |
| `footprints/tests/golden` (generators, manifests, vectors) | 6.3 MB | footprint tests (port parity with the original generators) | none |
| `web/crates/layout/tests/fixtures`, `web/tests/fixtures` | 132 KB, 136 KB | layout and web tests | `sofle-v2.json` version literal bumped twice |

## 3. Slowest 30 tests

Native only (per-test times exist only there). "Alone" is one test thread (clean); "16 threads" is the same test in
the default parallel run (inflated by waiting on a shared build).

| # | Seconds (alone) | Seconds (16 threads) | Suite | Test |
|---|---|---|---|---|
| 1 | 58.25 | 66.40 | core integration | `demo_projects::native_recipes_preserve_all_examples_and_embedded_model_bytes` |
| 2 | 51.22 | 57.56 | core integration | `generator_parameters::applying_a_boards_wiring_writes_only_values_the_parameter_check_accepts` |
| 3 | 15.25 |  | CAD native | `model::source_validation_tests::pinned_vik_step_meshes_preserve_source_bounds_and_hashes` |
| 4 | 15.02 | 15.84 | core integration | `generator_export::every_bundled_generator_exports_a_board_kicad_parses` |
| 5 | 11.70 |  | CAD native | `model::construction::equivalence::construction_preserves_edge_cases_and_step_material` |
| 6 | 8.23 |  | CAD native | `model::construction::validation_tests::source_module_pcb_keeps_front_and_back_placement_in_exported_step` |
| 7 | 5.96 | 62.92 | core integration | `included_keyboards::closure_proposals_on_a_paired_half_are_made_once_from_the_construction_source` |
| 8 | 5.90 |  | CAD native | `model::construction::validation_tests::core_internal_gasket_fixtures_export_connected_positive_regions` |
| 9 | 4.82 |  | footprints | `generators::bundled_generators_match_the_golden_baseline` |
| 10 | 2.59 | 59.21 | core integration | `included_keyboards::every_split_half_exports_to_kicad_with_its_own_closure_holes` |
| 11 | 2.47 | 3.95 | core integration | `mirrored_half::two_hosted_modules_mirror_with_exact_ids_poses_and_correspondences` |
| 12 | 2.10 | 6.29 | core integration | `kicad_integration::kicad_10_parses_the_exported_board` |
| 13 | 2.06 | 3.59 | core integration | `mirrored_half::reuse_board_copies_no_module_and_refuses_one_pinned_to_the_source` |
| 14 | 1.85 | 58.07 | core integration | `generator_parameters::every_value_a_core_writer_produces_passes_the_parameter_check` |
| 15 | 1.85 | 5.72 | core integration | `kicad_integration::exports_front_and_back_copper_traces_and_vias` |
| 16 | 1.56 | 2.85 | core integration | `mixed_project_acceptance::mixed_host_project_keeps_module_ownership_and_public_export_gates_together` |
| 17 | 1.56 | 4.74 | core integration | `kicad_integration::kicad_reports_copper_clearance_independently_of_unrouted_nets` |
| 18 | 1.55 | 3.18 | core integration | `mirrored_half::a_module_the_mirror_cannot_place_refuses_the_record_and_changes_nothing` |
| 19 | 1.48 | 4.62 | core integration | `kicad_integration::kicad_reports_unrouted_nets_independently_of_serializer_validity` |
| 20 | 1.37 | 3.17 | core integration | `kicad_integration::native_board_export_preserves_untouched_source_spans_and_kicad_10_parses_and_plots_both_sides` |
| 21 | 1.27 | 2.77 | core integration | `kicad_integration::imported_source_without_a_pad_angle_preserves_the_absolute_angle_on_both_board_sides` |
| 22 | 1.26 | 2.82 | core integration | `mirrored_half::a_circuit_and_a_module_on_one_board_mirror_together` |
| 23 | 1.20 | 2.28 | core integration | `module_service_clearance::splitter_service_clearance_is_bounded_and_preserves_only_holes_that_survive_offset` |
| 24 | 1.18 | 3.13 | core integration | `mirrored_half::a_board_with_embedded_circuits_mirrors_with_valid_namespaces_and_records` |
| 25 | 1.18 | 2.29 | core integration | `mirrored_half::anchored_geometry_lands_on_the_mapped_world_point_at_every_heading_and_map` |
| 26 | 1.17 | 2.22 | core integration | `mirrored_half::every_mirrored_module_raises_one_handed_module_warning_and_a_source_module_raises_none` |
| 27 | 0.89 | 1.66 | core integration | `mirrored_half::every_shipped_split_mirror_follows_the_switch_and_companion_rules` |
| 28 | 0.88 | 1.77 | core integration | `mirrored_half::the_heading_finding_is_one_located_warning_per_part_and_absent_without_a_pair` |
| 29 | 0.87 | 3.03 | core integration | `module_preview_footprints::mounted_module_resolves_source_footprints_in_host_frame_without_host_ownership` |
| 30 | 0.86 | 2.94 | core integration | `mounted_modules::catalogue_retains_all_twenty_nine_rows_and_real_source_variants` |

Not in the table: all wasm per-test times (not measurable, see section 2).

## 4. Classification

Classes: **P** protects real behaviour · **D** duplicates another · **R** pins a replaced mechanism ·
**G** pins incidental output (golden noise) · **T** trivial · **S** slow for what it proves.
"Read" means I read the test body; "names" means the classification is from names, doc comments and timings
only and must be re-read before phase 2 touches it.

| Group | Tests | Class | Basis | Note |
| --- | ---: | --- | --- | --- |
| core lib `mechanical::*`, `artifact::*`, `migrate`, `archive`, `matrix::*` | 301 | P | names | unit tests, 0.46 s together; no change proposed |
| `matrix_placement` | 14 | P (13), R (1 borderline) | read | the typed placement meanings of ADR-0007; `placement_survives_save_and_reopen_unnormalized` still guards "opening never converts", so keep |
| `mirrored_half` | 30 | P (27), D (2), partly D (1) | read | see 5.1; the timings of the module and circuit tests (1 to 2.5 s each) are fixture builds, not logic |
| `included_keyboards` | 15 | P (11), R (1, retire with 23), S (2) | read | see 5.2 |
| `demo_projects` | 1 | P (frozen key/matrix/wiring/model bytes), S (58 s), R (v4 to v5 pin), G (recorded `hardware`, `parts`, `definitions`, `boards`) | read | see 5.3 |
| `mechanical_integration`, `mechanical_settings_edits`, `mechanical_workflow`, `gasket_features` | 121 | P | names, sampled 2 | fast (under 2 s in all); cover Core mechanical rules |
| `outline_*`, `outlines` | 117 | P | names | fast |
| `kicad_integration` | 23 | P, S | read head | 11.1 s: each test spawns `kicad-cli`; independent parser, worth keeping |
| `generator_export` | 8 | P (7), S (1) | read | `every_bundled_generator_exports_a_board_kicad_parses` is 15 s for 36 separate kicad-cli runs |
| `generator_parameters`, `closure_generator_parameters` | 25 | P | read 2 | slow only through the shared build (5.4) |
| `core_engine`, other core integration modules | about 180 | P | names | fast |
| `current_schema` | 8 | P (7), R (1) | read | `open_refuses_documents_that_are_not_at_the_current_version` hard-codes 4, 6 and 5 |
| `boundary_parity` | 2 | P, G (golden) | read | see 5.5 |
| web native (365) and application (34) | 399 | P | names | under 1 s |
| web wasm: layout matrix transform toolbar | 19 | P | names + 1.0 s run | current ADR-0007 design; no stagger-via-old-toolbar test remains |
| web wasm: layout matrix inspector + Relations panel | 38 + 5 | P, some D | names | `inspector_stagger_lands_as_a_placement_edit...` and `toolbar_stagger_and_inspector_stagger_land_the_same_matrix` overlap by design (one-edit parity) and should stay |
| web wasm: presentation `layout_component_inspector_tests` + `inspector::tests` + `locked_position_tests` | 28 + 3 + 1 | P, probable D | names | about 5 pairs of the same position-draft/failure behaviour at different mounting points (see 6, item M3) |
| web wasm: keymap | 40 | P, S, D | read 3 | fixed-sleep settle loops; keycode/delay twin tests (item M2) |
| web wasm: case `mechanical_settings` | 28 | P, S | names + 24 s | `rendered()` sleeps |
| web wasm: host `storage_lifetime_tests` | 10 | P, R (6 version literals) | read lines | |
| footprints | 73 | P | names | golden parity with the original generators; real value, keep |
| CAD native | 51 | P | names | real kernel behaviour; 15 s (VIK mesh hashes) and 11.7 s are the cost of exactness |
| CAD browser smoke | 15 | P | read output | 3 s |
| Python | 121 | P | names | 3 s |
| Tests with no surviving trace of the old matrix toolbar stagger, per-key closure suggestions or recipe mirror arithmetic | | | grep | search found only the intended `old-toolbar column_offsets[c].y` compatibility test and the recipe-bay comparisons in 5.1 |

## 5. Detail on the named areas

### 5.1 `core/tests/mirrored_half.rs` (30 tests, 2,947 lines)

- Three tests assert that a mirror reproduces a shipped half: `corne_second_half_reproduces_the_shipped_right_half`
  (109 lines), `source_right_splits_reproduce_the_shipped_left_half` (lily58, klor) and
  `every_shipped_split_mirror_follows_the_switch_and_companion_rules` (all ten). The third runs the first two's switch
  comparison on every shipped split and adds `assert_reflects_every_part`. What the first two add is the comparison of
  `U1`, `RST`, `TRRS` against the recipe's reflected bay (recipe agreement), and, for Corne, that the pair
  carries the closure mounts and openings authored by the source. Classification: second is **D**; first is
  **D** except its closure/openings tail.
- `a_rotated_pair_map_...` (18 lines), `a_vertical_axis_map_...` (34) and `anchored_geometry_lands_on_the_mapped_world_point_at_every_heading_and_map`
  (93) share the "reflect through any map" rule. The third is the strict one (every heading and map for anchored
  geometry); the first two pin the key/companion pair rule at two maps. Keep all three: distinct input classes and
  tiny.
- The module and circuit tests cost 1 to 2.5 s each because each rebuilds a module-hosting project from
  scratch (`two_hosted_modules...` 2.47 s, `reuse_board_copies_no_module...` 2.06 s,
  `a_module_the_mirror_cannot_place...` 1.55 s, `a_circuit_and_a_module...` 1.26 s). That is a speed item (7.2),
  not a deletion.
- The ticket-numbered doc comments (`ticket 27`, `28`, `29`, `30`) are fine as history but make the assertions read as
  per-ticket regressions; phase 2 can group them by rule when it merges.

### 5.2 `core/tests/included_keyboards.rs` (15 tests, 1,066 lines)

- Overlap with `mirrored_half`: the shipped-pair checks `every_second_half_part_is_the_reflection_of_the_first_halfs_part`
  and `the_split_halves_outlines_are_mirror_images_through_the_pair_map` test the **recipe output** (the shipped
  data) against the pair rule, while `mirrored_half` tests the **operation**. They are not the same behaviour, but
  the first one pins "recipe reflection arithmetic: right = offset - left.x", which ticket 23 replaces by
  `CreateMirroredHalf`. It is **R, retire with ticket 23** (not now, it is the only guard of the shipped
  controller bay, reset and TRRS reflection until then).
- `closure_proposals_on_a_paired_half_are_made_once_from_the_construction_source` (6.0 s) loops nine shipped
  projects, each with an engine reopen, a settings edit and two KiCad exports. The rule it protects is also
  covered synthetically by `mechanical_integration::closure_proposals_come_from_the_recorded_construction_source`
  and `..._fall_back_to_the_first_member_without_a_recorded_source`. **S**: keep two projects (Corne and Sofle RGB,
  one measured, one Sofle) and drop seven.
- `every_split_half_exports_to_kicad_with_its_own_closure_holes` (2.6 s) is the only guard that every shipped half
  exports; keep.
- Overlap with `demo_projects.rs`: none in assertions. `demo_projects` pins keys, matrices, wiring and bytes;
  `included_keyboards` pins mechanical pair structure. But both rebuild the recipes (5.4).

### 5.3 `core/tests/demo_projects.rs` (1 test, 58 s)

One test: build 20 projects through `demo_projects::prepare` (55 s in debug), then for each: compare the document
with the baseline to 1e-9, run the v4 to v5 migration pin (`version_4_form_migrates_unchanged`), unpack the
archive, reopen it in an engine, assert no `generator-parameter:` finding, render every generator definition, and
compare asset bytes. It is one test, so it cannot be spread over cores, and its single run is the critical path of
the whole `test` step. It is also the only guard that the checked-in `web/assets/fixtures` recipe output has not
drifted from the shipped baseline.

### 5.4 The shared build is the real cost

`included_keyboards::projects()` runs `measured_project` for 7 splits, `sofle_project` for 3, and
`reused_pcb_project`, then holds them in a `OnceLock`. Measured alone, the first test to touch it takes about 50 s
(`generator_parameters::every_value_a_core_writer_produces_passes_the_parameter_check` 51.1 s alone, then 1.85 s
when it ran second; `included_keyboards::every_second_half_part_is_the_reflection...` 49.6 s alone, 0.00 s as a
cached caller). Ten of the baseline's 20 projects are the same ten splits (`measured-corne` and its siblings,
`sofle`, `sofle-rgb`, `sofle-choc`), and the baseline test already proves that recipe output equals those
documents. Only `reused-pcb` is not in the baseline.

### 5.5 The goldens, and the smallest surface worth pinning

- **Boundary transcript** (1.1 MB). Its purpose is native/WASM parity of the wire protocol. It pins whole `document`
  (261 KiB in the core part) and `scene` (218 KiB) bodies in all 67 core replies, and the repeating
  `check_core`/`check_outline` functions in `core/tests/support/boundary.rs` (about 500 lines) already assert the
  behaviour of the same replies by name. Every version bump or new field changes 120 lines. Minimum surface: per
  reply `id`, `kind`, error text, `revision`, the changed-ID lists and finding ids and severities, the sorted key
  set of `document` and `scene` (so a new field is a deliberate bless), and **full bodies for five anchor replies**
  (the first open, one commit, one error, matrix projections, archive unpack). Parity is kept because both
  transports produce the same projection.
- **Demo baseline** (2.5 MB: parts 846 KiB, matrices 625 KiB, nets 493 KiB, definitions 181 KiB, boards 164 KiB,
  hardware 82 KiB, outline 74 KiB). The file's own header says what is intentionally frozen: key positions, sources,
  matrices, wiring, model hashes. Yet `revision`, `hardware`, `parts`, `definitions` and `boards` of the ten
  paired splits have been re-recorded three times (typed-core-edits 04, generator-parameters 02 and 06). Minimum
  surface: keep full records for the 10 non-paired projects; for the 10 paired splits keep matrices, key switch
  poses and the wiring/net tables, and replace the mechanical-derived fields by the behaviour tests that already
  exist in `included_keyboards`. This is the baseline's own stated contract.
- **Footprint goldens** (6.3 MB): recorded output of the original generators; parity of a port. Keep as they are.

## 6. Proposed action list (for approval)

Nothing here is executed in phase 1. "Protected by" names what survives. Evidence column: **read** means I read
the bodies, **names** means re-read before the change.

| # | Action | Target | Behaviour it protected | What still protects it | CPU saved | Evidence |
| --- | --- | --- | --- | --- | ---: | --- |
| D1 | **Merge** into another | `mirrored_half::source_right_splits_reproduce_the_shipped_left_half` (64 lines) | lily58 and klor right-source mirrors reproduce the shipped left half | `every_shipped_split_mirror_follows_the_switch_and_companion_rules` (already runs both, and `assert_reflects_every_part`); the dropped extra is the `U1/RST/TRRS` match against the recipe bay, which ticket 23 replaces | 0.2 s | read |
| D2 | **Trim** | `mirrored_half::corne_second_half_reproduces_the_shipped_right_half` to its closure/openings and source-untouched tail (about 45 of 109 lines) | switch equality and bay agreement for Corne | same test as D1; closure and openings carry-over stays here | 0.07 s | read |
| D3 | **Trim loop 9 to 2** | `included_keyboards::closure_proposals_on_a_paired_half_are_made_once_from_the_construction_source` keep Corne and Sofle RGB | proposals come from the construction source and are linked | the same test on 2 projects; `mechanical_integration::closure_proposals_come_from_the_recorded_construction_source` and `closure_proposals_fall_back_to_the_first_member_...`; `every_split_half_exports_to_kicad_with_its_own_closure_holes` (all ten export) | about 4.5 s | read |
| D4 | **Rewrite** (not delete) | `generator_export::every_bundled_generator_exports_a_board_kicad_parses` to one board holding the 36 parts and one `kicad-cli` run, reporting which part failed | every bundled generator's footprint parses in KiCad 10 | same test, one process; `standalone_export_batches_every_generator_...` keeps the batch and file count | about 13 s | read |
| D5 | **Rewrite as behaviour tests** | version literals: `current_schema::open_refuses_documents_that_are_not_at_the_current_version` (4, 6, 5), `demo_projects` (`formatVersion == 5`), `core/src/migrate.rs` (4 sites), `core/src/archive.rs:413`, `host::storage_lifetime_tests` (6 sites), `outlines.rs` and the two fixture JSONs | open refuses older/newer, migration lands on the current version, archive and storage write the current version | the same tests with `migrate::CURRENT_VERSION` (and `CURRENT_VERSION - 1`, `+ 1`); the v4 to v5 and v3 to v5 chain tests keep their literal inputs | 0 s; removes about 14 edit sites per bump | read |
| D6 | **Delete** helper | `demo_projects::version_4_form_migrates_unchanged` run on 20 projects | a v4 document migrates changing only `formatVersion`, and a pair without new fields stays unpaired | `migrate::tests` (v4 to v5 case, `core/src/migrate.rs`), `current_schema::a_pair_without_a_source_or_halves_stays_unpaired` | small (inside the 58 s test) | read |
| D7 | **Rewrite** goldens | boundary transcript to the projection in 5.5 (full bodies for 5 replies) | wire parity native and WASM, precise changed IDs, errors | same test plus `check_core` and `check_outline` assertions, which already name the behaviours | 0 s; transcript about 1.1 MB to about 0.2 MB; bump diff about 120 lines to a few | read |
| D8 | **Rewrite** golden | `demo-projects-baseline.json`: drop recorded `hardware`, `parts`, `definitions`, `boards` for the 10 paired splits | none of these is frozen by contract; they were re-recorded 3 times | `included_keyboards` (pair structure, closure holes, export), `mirrored_half::every_shipped_split_...` (poses), keys/matrices/wiring still frozen | 0 s; baseline about 2.5 MB to about 1.3 MB | read (file header) |
| M1 | **Rewrite** (speed, no loss) | fixed-sleep settle loops in `keymap::queued_edit_tests` (`settle`: 15 x 50 ms), `case::mechanical_settings` (`rendered` 60 ms), `pcb_*`, `parts_definition_name` | n/a | same assertions, wait until the runtime queue and render are quiet | see 7.1 | read 2 files |
| M2 | **Merge** twins | `keymap::macro_controller::mounted_step_tests`: keycode/delay pairs (`a_failed_keycode_restores...` and `a_failed_delay_restores...`, `a_newer_keycode_draft...` and `a_newer_delay_draft...`, `raw_step_keycode_whitespace...` and `raw_step_delay_leading_zeros...`, `raw_step_newer_keycode...` and `raw_step_newer_delay...`) into four table-driven tests; same for the layer-name/macro-number/macro-text "newer draft" and "failed unchanged" variants in `queued_edit_tests` | the same draft-versus-failure rules for each field kind | table-driven tests with one row per field kind | about 6 to 10 s of the 27 s (7 to 10 tests) | **names, verify** |
| M3 | **Review, probably merge** | `presentation::layout_component_inspector_tests` position-draft tests versus `presentation::inspector::tests::mounted_*position*` and `locked_position_tests` (about 5 pairs) | newer-draft preserved across landing/failure, locked/driven component has no position field | one surviving test per behaviour | about 5 to 8 s of the presentation run | **names, verify** |
| R1 | **Retire with ticket 23** | `included_keyboards::every_second_half_part_is_the_reflection_of_the_first_halfs_part`, `the_split_halves_outlines_are_mirror_images_through_the_pair_map`, `mirrored_half` recipe-bay comparisons | the recipe's reflection arithmetic | `mirrored_half::every_shipped_split_...` and the new operation-built halves | 0.4 s | read |

Not proposed for removal (looked at, they earn their time): every `matrix_placement` test, the toolbar nudge and
shortcut tests, the Relations panel tests, `inspector_stagger_lands_as_a_placement_edit...` together with
`toolbar_stagger_and_inspector_stagger_land_the_same_matrix` (one edit path is the point of ADR-0007), the footprint
goldens, the CAD tests, Python tooling, `kicad_integration`.

Deletions alone would remove under 1 percent of test time. Counting D1 to D6 as test or line changes:
about 3 tests merged or trimmed in `core`, 1 helper deleted, 1 loop trimmed; M2 and M3 could remove 10 to 15 wasm
tests once verified. Most of the value of D5, D7 and D8 is avoided churn at the next format bump or mirrored-half
change.

## 7. Speed-only options (no test deleted)

1. **Build the shipped keyboards once, not twice (and not from scratch).** `included_keyboards::projects()` could
   read the ten split documents from `demo-projects-baseline.json` (already proven equal to the recipes by
   `demo_projects`) and build only `reused-pcb`. CPU saved about 45 to 50 s. Wall saved is smaller (about 10 s)
   until option 2 lands, because `demo_projects` is then the single critical path.
2. **Make the recipes themselves faster in debug.** `prepare()` takes 55 s in the unoptimized `boardstudio-core`.
   The workspace already sets `opt-level = 2` for dependencies (about 2.4 times faster for hashing and
   parsing) but not for the workspace crates. Candidates, **not measured here because they rebuild the shared
   `target`**: `opt-level = 1` for `boardstudio_core` under `[profile.dev.package]`; or running `prepare` per fixture
   across threads (the 20 fixtures share one engine now, so this needs a recipe change); or the `build` step's
   fixture output reused by the test instead of regenerated. Together the `test` step could plausibly drop from
   74.5 s toward the CAD command's 24 s (the next longest), since after options 1 and 2 and D4 the longest
   core test is the 6 s closure proposal loop. This is an estimate.
3. **`generator_export` KiCad parse (D4)**: 15 s to about 2 s, the next critical path item.
4. **Silent skip risk**: `kicad_parses` returns `true` when `kicad-cli` is absent, so a machine without it passes
   `generator_export` unseen. Not a time item; worth an explicit skip message.
5. **wasm sleeps (M1)**: `keymap::queued_edit_tests` is 27.7 s for 15 tests because `settle` is 15 x (pending +
   50 ms). Polling for quiescence should bring these to about 1 s each or less; `case mechanical_settings` (24 s,
   28 tests) and `pcb`/`parts` suites share the pattern. Browser step estimate: keymap 46 s to about 25 s, case 44 s
   to about 30 s; since commands run four at a time the step wall improves by perhaps 15 to 30 s. This is an
   estimate from the filtered runs above.
6. **Browser launches**: 14 `wasm-pack test` commands, each with 7 to 13 s of fixed start-up. Reducing launches is
   a trade against isolation and the 120 s runner bound; not measured.
7. **Module/circuit fixtures in `mirrored_half`**: four tests of 1.3 to 2.5 s each rebuild a module-hosting project;
   a shared builder could cut about 3 to 4 s of CPU. Low value for wall time.

## 8. Estimated time per `check.py` step

| Step | Now (measured 2026-10-10, warm) | After options 1, 2 and D4 | After D1 to D3 and D6 | Note |
| --- | ---: | ---: | ---: | --- |
| `test` | 74.5 s | about 25 to 35 s (estimate) | minus 5 s of CPU, no wall change | critical path moves to the CAD command (24 s) |
| `browser` | 159.1 s | about 125 to 145 s with M1 (estimate) | M2, M3: a few seconds | unmeasured scheduling effect |
| `tooling`, `repo` | 2.9 s, 0.1 s | unchanged | unchanged | |
| `build` | not run in phase 1 (206.8 s at the Batch A boundary) | the third recipe build is the same work as option 2 | | |
| `lint`, `typecheck` | not affected | | | |

## 9. What I could not measure

- Per-test wasm times (the runner prints none); only filtered-run wall times, each including start-up.
- The effect of `opt-level` or parallel `prepare` (would rebuild the target directory shared with the main checkout).
- Browser step scheduling effects, and the share of the 48 s presentation run per test.
- The `build` step and a full `check.py` before and after (phase 2 records those, per the ticket).
- Test bodies outside the groups marked **read**: the classification of about 400 core tests and 300 wasm tests is
  from names, timings and doc comments.
- GitNexus index is stale (it reported `4b372b2`); I used `rg`, test lists and timings, not the graph.
