# W3-4 Crate Decomposition — Phase 0 Dependency-Graph Design

**Status:** Implemented and verified on `perf/w3-4-split-core-engine-and-web-layout` (see §8)
**Baseline:** `aba05d895` (`perf/cold-build-cpu`, `cpu-opt3`: 310.1 s warm / 416.2 s bypass)
**Branch:** `perf/w3-4-split-core-engine-and-web-layout`
**Spec:** Section W3-4 of [cold-build-wave3-handoff.md](cold-build-wave3-handoff.md)
**Evidence (local, gitignored):** `.evidence/w3-4-phase0-{graph,paths,scc,files,layout}.log`
(reproducible with the analysis scripts described in §2; counts below are quoted from them)

---

## 1. Goals and non-goals

Split the two monolithic crates so Cargo compiles their `lib` and unit-test targets in
parallel, without changing any public API seen downstream:

- `boardstudio-core-engine` — 114,303 LOC, 95 files, 717 native unit tests in one binary
  (90.3 s warm / 121.7 s bypass single-unit wall).
- `boardstudio-web-layout` — 71,811 LOC, 67 files, 168 native unit tests plus the bulk of
  one of the 13 browser crate test binaries (799 tests total across launches).

Non-goals: no behavior change, no floating-point reordering (see
[CODING_STANDARDS.md](../../CODING_STANDARDS.md)), no downstream call-site churn.
`boardstudio_core` ([core/src/lib.rs](../../core/src/lib.rs)) stays the facade: the only
in-repo consumer of `boardstudio_core_engine` is that facade (verified by grep over
`application/ core/ web/ renderer/`), and the only consumer of `boardstudio-web-layout`
is the page crate `boardstudio-web` (`web/src/**`, 16 files). Both facades re-export the
new crates so every existing path (`boardstudio_core::mechanical::…`,
`boardstudio_web_layout::objects::…`) resolves to the same items.

Conventions followed, matching the earlier `refactor/split-core-crate` split that produced
[core/crates/model](../../core/crates/model/Cargo.toml) and
[core/crates/script](../../core/crates/script/Cargo.toml):

- New core crates live in `core/crates/*` (already globbed by the root
  [Cargo.toml](../../Cargo.toml) workspace `members`), named `boardstudio-core-*` as the
  handoff suggests (`boardstudio-core-mirrored-half`), edition 2024,
  `publish = false`, `[lints] workspace = true`, path deps only.
- Dependency injection by fn-pointer alias instead of upward calls, exactly like
  `ScriptRunner` (today at engine `lib.rs`, `pub type ScriptRunner = fn(&mut ProjectDoc)
  -> Result<(), String>`) lets the engine stay below `boardstudio-script`.
- `pub(crate) use <dep>::<module>` path shims in a new crate's `lib.rs`, exactly like
  [web/crates/layout/src/lib.rs](../../web/crates/layout/src/lib.rs) already does for
  `runtime`, `selection`, `parts`, so moved files keep their `crate::…` paths and —
  critically — their `#[test]` module paths (native baseline keys change crate, test
  names do not; see §5).
- Web crates always use `web-sys = { workspace = true }` and copy the existing
  target-gated Dioxus specs verbatim, so no new `.rlib` hash appears under
  `target/wasm32-unknown-unknown/wasm-check/deps/` (`libweb_sys` stays 1 copy,
  `libboardstudio_web_host` stays 2 copies).

## 2. Method

For each crate, every `.rs` file was attributed to its top-level module
(`mirrored_half/…` + `mirrored_half.rs`, etc., honoring `#[path]` inclusions such as
`geometry.rs` → `outline.rs`/`outline_cleanup.rs` and `matrix_transform_operation.rs` →
`handle_rail.rs`), then:

1. LOC and `#[test]`/`#[wasm_bindgen_test]` counts per top module;
2. every `crate::<top>::…` occurrence, split into `*test*.rs` files vs production files,
   then re-run on production files with `#[cfg(test)]` items stripped by brace matching,
   so test-only edges (e.g. tests calling `crate::CoreEngine`) do not fake cycles;
3. Tarjan SCCs over the production top-module graph, treating `crate::model`,
   `crate::electrical_jumpers`, `crate::electrical_profiles` as the external leaf they
   are (re-exports of `boardstudio-model`), and crate-root free functions
   (`apply_with_ledger`, `apply_unscripted`, `prune_removed_components`) as explicit
   callback edges;
4. file-level edges inside `matrix/`, `mechanical/`, `mirrored_half/`, `artifact/`,
   `modules/`, and web `objects/`, plus resolution of the layout crate's
   `presentation::` alias and `super::`/`super::super::` sibling references;
5. downstream API inventory (`boardstudio_core::*` and `boardstudio_web_layout::*`
   occurrences) and tooling inventory (native baseline, browser launches).

Caveat: references through glob imports (`use crate::model::*`, `use super::*` in tests)
are counted once at the `use` site. Directions are exact; weights are lower bounds.
Every claimed cycle-breaker below was read at its call site, not just counted.

## 3. W3-4a — `boardstudio-core-engine` findings

Top modules (LOC includes inline and file tests; native tests from
`scripts/check-native-tests-baseline.json`,
`boardstudio_core_engine: 717`):

| Top module | LOC | Native tests | Production `crate::` out-edges (excl. `model`) |
|---|---:|---:|---|
| `mirrored_half` | 51,401 (≈22k prod) | 370 | `matrix`(99) `mechanical`(31) `constraints`(28) `electrical`(7) `modules`(6) `geometry`(5) `outline_controls`(3) `generators`(3) `keymap`(2) `outline_edit`(2) `electrical_peripherals`(2) + root callbacks |
| `mechanical` (+`mechanical_checks`) | 19,988 | 110 + 2 | `matrix`(8) `modules`(6) `case`(5) `geometry`(5) `generators`(3) `artifact`(1) |
| `matrix` | 10,538 | 23 | `outline_controls`(4) `constraints`(3) `inputs`(2) `outline_versions`(1) `generators`(1) + root callback |
| `artifact` | 9,415 | 71 | `modules`(3) `matrix`(2) `mechanical`(2) `generators`(1) `hash`(3) |
| `modules` | 3,829 | 0 (integration only) | `matrix`(4) `case`(3) `electrical_peripherals`(2) `geometry`(2) `hardware`(1) `artifact`(1) |
| crate root `lib.rs` | 3,504 | 18 | calls everything (bare paths); defines `CoreEngine`, `apply_with_ledger` dispatch (≈925 lines), scene |
| `constraints` | 2,176 | 47 | `matrix`(6) |
| `geometry` (+`outline`,`outline_cleanup`) | 2,077 | 8 | `outline_versions`(9) `outline_controls`(6) `matrix`(6) |
| `outline_edit` | 1,555 | 3 | `outline_controls` `geometry` `outline_versions` |
| `archive` | 1,449 | 24 | `migrate` `hash` |
| `electrical` | 1,368 | 0 | `inputs`(3) `electrical_peripherals`(3) `modules`(2) `hash`(1) |
| `firmware` | 1,049 | 10 | `keymap`(3) |
| `case` | 907 | 10 | — (leaf on model/contracts) |
| `migrate` | 904 (≈126 prod) | 12 | — production leaf (serde_json) |
| `keycaps` | 813 | 3 | `keymap`(1) `matrix`(1) |
| small: `outline_validation` 691, `outline_versions` 558, `validate` 528, `generators` 458, `inputs` 282, `electrical_peripherals` 275, `outline_controls` 234, `keymap` 148, `hardware` 128, `hash` 28 | ≈3,800 | 11 | see evidence log |

After stripping `#[cfg(test)]`, the production graph has exactly **one SCC** —
`{artifact, constraints, electrical_peripherals, geometry, hardware, inputs, matrix,
mechanical, modules, outline_controls, outline_versions}` (≈48k LOC) — and
`mirrored_half`, `electrical`, `firmware`, `archive`, `case`, `keycaps`, `keymap`,
`migrate`, `validate`, `generators`, `hash`, `mechanical_checks`, `outline_edit`,
`outline_validation` are already acyclic once `model` re-exports and root callbacks are
treated correctly. In particular **nothing in production code depends back on
`mirrored_half`** except `validate` (2 finding helpers) and the crate root — mirrored is
a top layer, not a leaf, so it must be extracted last, not first.

### 3.1 Cycle-breakers inside the SCC (all read at the call site)

1. **`matrix::deletion` → root `apply_unscripted`** (1 site,
   `matrix/deletion.rs:449`, dimension-1 axis removal re-dispatches
   `EditOperation::RemoveMatrix`). `remove_parts_by_id` itself is pure.
   Fix: `remove_cells`/`remove_axis` take an `ApplyUnscripted` fn-pointer
   (`fn(&mut ProjectDoc, &EditOperation) -> Result<Vec<String>, String>`, alias hosted
   by the kernel crate). Callers: root dispatch (5 sites in `lib.rs`) and
   `mirrored_half` (`reconcile.rs:6695,6765`), both already above the kernel.
2. **`matrix::layout` → `mechanical::pair::mirrored_part_heading`** (1 site,
   `matrix/layout.rs:2855`) is a mirage: `mechanical::pair` is
   `pub use boardstudio_model::mechanical::pair`. Point the call at
   `boardstudio_model` and the `matrix → mechanical` edge disappears. Same for
   `mechanical::change` uses in `mirrored_half`.
3. **`matrix ↔ constraints`** is real but tiny and stays intra-crate:
   `layout.rs` calls `constraints::retune_offset`/`mirror_drag_refusal` (2 sites);
   `constraints.rs` (engine-side resolution over `boardstudio_model::constraints`)
   calls `matrix::layout` pose helpers and `matrix::splay::rotate` (6). Keeping
   `matrix + constraints + inputs + outline_controls + outline_versions` in one kernel
   crate leaves this untouched. `inputs → matrix::mark_override` (1) and
   `outline_controls → matrix::layout` (2) likewise become intra-kernel.
4. **`modules → artifact::preview_footprint_surfaces`** (1 site,
   `modules/placement.rs:567`, inside `modules::resolve`) vs
   **`artifact → modules::resolve/placed`** (3). The preview function is
   `artifact/preview.rs:549 footprint_surfaces`, which shares its `graphic()` walker
   with board preview and needs `sexpr` + `parsed_board`. Fix: move the whole KiCad
   *source-parsing* cluster (`sexpr.rs` 224, `parsed_board.rs`, `preview.rs` 678,
   `mechanical_extract` ≈1.8k, plus `legacy_arc_parameters` from `artifact/mod.rs:20`)
   into a new leaf crate `boardstudio-core-kicad-source` below `modules`,
   `mechanical`, and `artifact`. This also fixes:
5. **`mechanical::builtin_profile` → `artifact::mechanical_extract`** (1 site,
   `mechanical.rs:2339`, production) and **`outline_validation →
   artifact::mechanical_extract::extract`** (1 site): both become calls into
   `boardstudio-core-kicad-source`.
6. **`electrical_peripherals → modules::host_requirements`** (1 site,
   `electrical_peripherals.rs:188`) vs **`modules::electrical` returning
   `PeripheralRequirement`**: the type already lives in
   `boardstudio_model::electrical_peripherals`, so no type move is needed; placing
   `electrical`/`electrical_peripherals`/`keymap` in a crate *above* `modules`
   (`boardstudio-core-electrical`) makes both directions legal.
7. **`mirrored_half` → root**: `apply_with_ledger` (1 site in
   `reconcile/prepare.rs` `apply_canonical`), `apply_unscripted` (3 sites:
   `inverse/hosted.rs:833`, `lifecycle` via `reconcile.rs:6365` staging),
   `prune_removed_components` (2 sites, an 8-line helper over
   `outline_versions::prune_removed_sources`). Root → mirrored is
   `prepare_transaction_with_ledger`, `prepare_open_with_scripts`, `create`,
   `create_with_physical_setup`, `companion_placement_disabled_reason`; the
   `SetHalfException`/`SetCompanionPlacement`/`UnlinkHalves` arms of the root dispatch
   already return an error ("require prepared transaction dispatch", `lib.rs:2626`),
   so the mutual recursion is exactly the apply callback. Fix, mirroring
   `ScriptRunner`: move the `ScriptRunner` alias to `boardstudio-model` (engine root
   re-exports it; no downstream user names it), add
   `pub type ApplyWithLedger = fn(&mut ProjectDoc, &[BoardContours], &EditOperation,
   &MechanicalIdentityLedger, Option<ScriptRunner>) -> Result<Vec<String>, String>`
   to the mirrored crate, thread it through `prepare_transaction_with_ledger` /
   `prepare_open_with_scripts`, and move `prune_removed_components` down into
   `outline_versions` (kernel), which the root also calls.
8. **Test-only upward edges** (do not block the lib DAG; handled by dev-dependencies):
   mirrored test files call `crate::apply` (23) and `crate::CoreEngine` (8);
   `constraints::framed_tests` uses `mirrored_half` (2); `matrix.rs` and
   `mechanical/edit.rs` inline tests construct `CoreEngine`. Cargo permits
   dev-dependency cycles. The remnant engine gains an empty `test-support` feature
   (precedent: `boardstudio-web-ui-model`, `-runtime`, `-ui-shared`) exposing
   `apply_for_test` (the current `#[cfg(test)] fn apply`, `lib.rs:1731`);
   `CoreEngine::with_script_runner(boardstudio_script::apply_scripts)` is already
   public, so test construction needs no new API. The feature is never enabled for
   `wasm32` builds, so no second engine hash appears in `wasm-check`.
   One path fix rides along: `mirrored_half/reconcile/prepare/tests.rs:12`
   includes `../../../../../../tests/support/synthetic_half.rs`
   (`core/tests/support/`); the relative depth changes with the crate move.

### 3.2 Proposed W3-4a DAG (7 new crates, remnant engine ≈12k)

```
boardstudio-model, boardstudio-contracts, boardstudio-footprints   (existing leaves)
        |
        +--> boardstudio-core-kernel            (≈17.4k)
        |      matrix/ matrix.rs constraints.rs inputs.rs hardware.rs
        |      outline_controls.rs outline_versions.rs outline_edit.rs
        |      geometry.rs (+outline.rs +outline_cleanup.rs) case.rs
        |      generators.rs hash.rs     deps: model contracts footprints i_overlay sha2
        |
        +--> boardstudio-core-kicad-source    (≈3.5k; deps: model contracts kernel kiutils-sexpr)
        |      sexpr.rs parsed_board.rs preview.rs mechanical_extract/
        |
        +--> boardstudio-core-modules         (3.8k; deps: kernel kicad-source model contracts i_overlay)
        +--> boardstudio-core-electrical      (1.8k; deps: kernel modules model)
        |      electrical.rs electrical_peripherals.rs keymap.rs
        +--> boardstudio-core-mechanical      (≈20k; deps: kernel modules kicad-source model contracts i_overlay)
        |      mechanical/ mechanical.rs mechanical_checks.rs
        +--> boardstudio-core-artifact        (≈6k; deps: kernel modules mechanical kicad-source
        |      artifact/ remainder            model contracts footprints cavalier_contours)
        +--> boardstudio-core-mirrored-half   (51k total / 22k prod;
        |      mirrored_half/ mirrored_half.rs  deps: kernel modules electrical mechanical model)
        +--> boardstudio-core-engine (remnant ≈12k; deps: all above + zip)
               lib.rs archive/ firmware.rs keycaps/ migrate.rs
               outline_validation.rs validate.rs
        +--> boardstudio_core (facade, unchanged paths)
        +--> boardstudio-script (existing; dev-dep of new crates' tests as today)
```

Every file moves intact under a new `src/lib.rs` that declares the same module names
(`pub mod matrix; pub mod mechanical; …`) and shims cross-crate siblings
(`pub use boardstudio_model::model;`, `pub(crate) use boardstudio_core_kernel::matrix;`
…), so `crate::matrix::layout::…` inside moved code and the baseline test paths
(`mechanical::edit::tests::…`) are preserved verbatim. The remnant engine `lib.rs`
keeps its exact public surface by re-exporting only today's public items from the new
crates (`generators`, `hash`, `outline_edit`, `layout_kernel`, the `matrix::layout`
helpers, `mechanical`, `artifact`, `electrical`, `electrical_peripherals`, …); modules
that are private today (`matrix`, `geometry`, `case`, …) stay reachable internally via
plain `use`, not `pub use`. Expected visibility work,
compiler-driven per slice: `matrix` carries 119 `pub(crate)` items, `modules` 29,
`mechanical` 23, `constraints` 18, `outline_versions` 17, `mirrored_half` 17 — those
used across the new boundary widen to `pub`; `pub(super)` items are intra-module and
mostly unaffected. External deps move with their only users (`zip` stays with
`archive` in the remnant — verify no artifact use; `kiutils_sexpr` to kicad-source and
artifact; `cavalier_contours` to artifact; `i_overlay` to kernel/modules/mechanical).

**Implementation slices** (each compiles and commits cleanly; baseline regenerated once
at the end): S1 in-place prep (alias move, `prune_removed_components` move, model-pair
redirect, deletion callback) → S2 kernel → S3 kicad-source → S4 modules → S5
electrical → S6 mechanical → S7 artifact → S8 engine `test-support` + mirrored-half
(+ `ApplyWithLedger` threading). Per slice: `cargo check -p <touched>` and
`cargo test -p <new crate> -p boardstudio-core-engine --lib` under the shared build
lock; at the end `python3 scripts/check.py repo tooling lint test build` and the
baseline regeneration in §5.

## 4. W3-4b — `boardstudio-web-layout` findings

| Top module | LOC | Tests | Internal out-edges (resolved) |
|---|---:|---|---|
| `objects` (+`objects.rs`) | 42,769 | 194 wasm + 15 native | `matrix_transform_operation`(9) `part_rail`(7) `outline_snapping`(4) `mirrored_pair_geometry`(4) `matrix_relations`(3) `anchor_pick`(2) `outline`(via `outline_lifecycle` shim: `OutlineAction`) `part_placement`(type only) `outline_settings`(1) `mirror_existing_half`(1) `mirrored_pair_lifecycle`(1) |
| `outline` | 5,599 | 15 wasm + 19 native (planner) | `objects`(30, mostly `TreeContext` = ui-model re-export) `outline_snapping`(13) `outline_settings`(7) |
| `part_placement` | 4,212 | 31 wasm | `objects`(2+: `TreeContext`, `LayoutSelectionKind`, `MatrixPlacementMount`, snap helpers) |
| `matrix_transform_operation` (+`handle_rail`) | 4,143 | 49 native | — leaf |
| `matrix_relations` | 3,460 | 30 native | `matrix_transform_operation`(2) |
| `outline_lifecycle_*tests` | 3,636 | 19 wasm + 19 native | `outline` |
| leaves: `anchor_pick` 1,221 (13 native), `anchor_badge` 859 (15), `mirrored_pair_geometry` 431 (6), `outline_snapping`+`outline_grid_rounding` 344 (1), `mirror_existing_half` 275 (2), `matrix_transform_lifecycle` 540 (4), `part_rail` 127 (5), `setup_guide_state` 134 (4), `screen_scale` 66 (3), `mirrored_pair_lifecycle` 60 (2), `outline_settings` 52 (0) | ≈4,100 | 55 native | `anchor_badge → matrix_relations`, `mirror_existing_half → mirrored_pair_geometry`; rest are pure/off-core only |
| remnant candidates: `setup_guide` 1,022, `board_inspector` 809, `layout_viewer` 717, `layout_findings` 624, `layout_findings_state` 602, `lib.rs` 106 | ≈3,900 | wasm | `board_inspector → objects`(1), `layout_findings_state → layout_findings`(7) + `objects::ScopedTreeContext` field (= ui-model type), `setup_guide → setup_guide_state`(4) |

The graph is a DAG except for two 2-cycles, both breakable without moving logic:

1. **`objects → part_placement`**: only the type
   `super::part_placement::ComponentPlacementAction` (`objects.rs:194,731`), which is
   itself `pub use boardstudio_web_ui_model::state::ComponentPlacementAction`
   (`part_placement.rs:52`). Import from `boardstudio-web-ui-model` directly.
2. **`objects → outline`**: `super::outline_lifecycle::OutlineAction`
   (`objects.rs:117,559`); `outline_lifecycle.rs` is a 3-line
   `pub use crate::outline::*` shim and `OutlineAction` lives in
   `outline/planner.rs` — a pure module (application + core model + ui-model
   `tree::TreeContext` + `outline_settings`) whose `for_tree` constructor is all
   `objects` needs. Move `planner.rs` (442 lines, with its
   `#[path = "../outline_lifecycle_tests.rs"]` native tests) down with the leaves,
   keep it as `outline::planner` inside the leaf crate (preserving the
   `outline::planner::lifecycle_tests::…` baseline names), and let the remnant
   `outline/mod.rs` re-export it.

Test-only upward edge: `objects/matrix_setup_focus_tests.rs` mounts
`crate::presentation::setup_guide::ProjectSetupGuide` — decide in-slice between moving
that one wasm test file into the remnant layout crate (preferred; it is an integration
mount) or a dev-dependency from objects to layout.

### 4.1 Proposed W3-4b DAG (2 new crates)

```
boardstudio-web-layout-core    (≈12k, dir web/crates/layout-core, NO dioxus/web-sys)
  matrix_transform_operation.rs + handle_rail.rs   matrix_transform_lifecycle.rs
  matrix_relations/  anchor_badge/  anchor_pick.rs  mirrored_pair_geometry.rs
  mirror_existing_half.rs  mirrored_pair_lifecycle.rs  outline_settings.rs
  outline_snapping.rs + outline_grid_rounding.rs   outline/mod.rs (planner only)
  screen_scale.rs  part_rail.rs  setup_guide_state.rs
  deps: application, boardstudio_core, ui-model, serde_json (dev as needed)
        |
boardstudio-web-layout-objects (42.8k, dir web/crates/layout-objects, wasm-gated like today)
  objects.rs + objects/   deps: layout-core + layout's current dep set verbatim
        |
boardstudio-web-layout (remnant ≈17k: outline hook/inspector/overlays/adapter,
  part_placement, board_inspector, layout_viewer, layout_findings{,_state},
  setup_guide, outline lifecycle browser tests)
  deps: current set + layout-core + layout-objects; re-exports every moved module
```

`layout-core` holds 153 of layout's 168 native tests and **zero**
`wasm_bindgen_test`s, so it gets **no browser launch** (it is still compiled for wasm
as a dependency of objects/layout/page) and the 799 browser count is conserved by
construction; `layout-objects` takes the 15 native `objects` tests (same cfg shim for
`keycap_resize`/`layout_align_geometry` as today's `lib.rs:94`) plus the 194 wasm
objects tests via **one added launch**. Page API is preserved by re-exports in the
remnant `lib.rs`; `cargo tree` feature checks per
[CODING_STANDARDS.md](../../CODING_STANDARDS.md) confirm one `web-sys` hash.

**Tooling touched (W3-4b):** root [Cargo.toml](../../Cargo.toml) explicit web member
list (+2); [scripts/check.py](../../scripts/check.py) `browser()` (+
`crate_launch("layout-objects")` — package/dir derivation
`boardstudio-web-{crate}` already fits) and any exact-list expectations in
`scripts/test-check.py`; `scripts/check_quick.py` file→crate mapping if it enumerates
crates (verify in slice); [scripts/wasm-test-owners.json](../../scripts/wasm-test-owners.json)
paths (currently stale `web/src/presentation/objects/…` entries — update to the layout
objects paths actually used by `check-wasm-tests.py`); doc links via
`python3 scripts/check-doc-links.py` (several `docs/plans/**` files cite
`crates/engine/src`/`crates/layout/src` paths — historical plan docs are not operating
instructions per [AGENTS.md](../../AGENTS.md); only fix links the checker flags).

## 5. Test conservation proof (both phases)

Method (must hold before each commit is declared done):

1. Snapshot the pre-split baseline from git
   (`git show HEAD:scripts/check-native-tests-baseline.json` before regeneration) and
   the pre-split browser count from the last full `check.py`/`cold.py` log (799).
2. After the split, regenerate **only** via
   `python3 scripts/check.py test --update-native-baseline` (never hand-edit).
3. Prove 1-to-1 conservation with a multiset comparison of test-name strings:
   the union over all crates after must equal the union before, except for the
   explicitly listed prefix renames below (apply the rename map, then require exact
   multiset equality and `0` dropped / `0` added unintentionally).

Expected mapping for `boardstudio_core_engine: 717` (all test strings preserved unless
noted; new baseline keys are the new crate lib names):

| New key | Tests | Composition (baseline prefixes) |
|---|---:|---|
| `boardstudio_core_mirrored_half` | 370 | `mirrored_half::*` (reconcile 230, inverse 103, matrix_addition 26, tests 8, remainder) |
| `boardstudio_core_mechanical` | 112 | `mechanical::*` 110 + `mechanical_checks::*` 2 |
| `boardstudio_core_kernel` | 95 | `constraints::*` 47 + `matrix::*` 23 + `case::*` 10 + `geometry::*` 8 + `outline_edit::*` 3 + `generators::*` 2 + `hash::*` 1 + `matrix_protocol_tests` 1 (verify home in slice) |
| `boardstudio_core_artifact` | 54 | `artifact::*` 71 − `artifact::mechanical_extract::*` 9 − `artifact::preview::*` 5 − `artifact::sexpr::*` 3 |
| `boardstudio_core_kicad_source` | 17 | the three subtractions above, **renamed**: `artifact::mechanical_extract::` → `mechanical_extract::`, `artifact::preview::` → `preview::`, `artifact::sexpr::` → `sexpr::` (parsed_board: verify count in slice) |
| `boardstudio_core_electrical` | 3 | `electrical_peripherals::*` 3 |
| `boardstudio_core_engine` (remnant) | 66 | `archive::*` 24 + root modules 18 (`mechanical_edit_engine_tests` 6, `set_wiring_mode_tests` 6, `preview_electrical_loss_tests` 3, `preview_backup_tests` 2, +1) + `migrate::*` 12 + `firmware::*` 10 + `keycaps::*` 3 — slice must reconcile to exactly 717 total |

Core integration suites (`integration` 240 + `integration_matrix` 229 +
`integration_mechanical` 230 + `integration_modules` 202 = 901 names in the
`boardstudio_core` package) compile against the unchanged facade and must be
byte-identical in the regenerated baseline. `scripts/check-core-tests.py` only
registers `core/tests/**` files and needs no change.

Expected mapping for `boardstudio_web_layout: 168` native (test strings preserved;
planner keeps its `outline::` nesting inside layout-core):

| New key | Tests | Composition |
|---|---:|---|
| `boardstudio_web_layout_core` | 153 | `matrix_transform_operation` 49 + `matrix_relations` 30 + `outline` 19 + `anchor_badge` 15 + `anchor_pick` 13 + `mirrored_pair_geometry` 6 + `part_rail` 5 + `matrix_transform_lifecycle` 4 + `setup_guide_state` 4 + `screen_scale` 3 + `mirror_existing_half` 2 + `mirrored_pair_lifecycle` 2 + `outline_grid_rounding_tests` 1 |
| `boardstudio_web_layout_objects` | 15 | `objects::*` native subset |
| `boardstudio_web_layout` | 0 native | key disappears from the native baseline; all remaining tests are wasm |

Browser (wasm) conservation: 799 tests, launches 13 → 14 (objects' 194-ish wasm tests
move to the `layout-objects` launch; exact per-launch split read from the first
post-split `check.py browser` run and checked against the pre-split per-launch counts,
total must be ≥799 with every pre-split test name accounted for).
`check-wasm-tests.py` must still pass with zero raw `launch_virtual_dom` calls.

## 6. Verification gates (per phase, under the shared build lock)

All Cargo/`check.py`/`cold.py` runs hold `~/.cache/tmp/boardstudio-build.lock` with a
blocking `flock` on fd 9 and export `BOARDSTUDIO_BUILD_LOCK_FD=9
TMPDIR="$HOME/.cache/tmp" PATH="$HOME/.cargo/bin:$PATH"`; never set
`CARGO_TARGET_DIR` for `cold.py`; logs redirected into `.evidence/`.

- **W3-4a done:** `python3 scripts/check.py repo tooling lint test build` green;
  regenerated native baseline totals **2,454** with the §5 multiset proof attached to
  the commit evidence log; 0 manifest warnings; commit.
- **W3-4b done:** `python3 scripts/check.py repo tooling lint test build browser`
  green; browser **799** conserved across 14 launches, page **126**, drag **15**, CAD
  browser **364**, CAD **566/15**; `check-wasm-tests.py` clean; commit.
- **Phase 3:** `python3 scripts/check.py security deny`; `python3 scripts/check.py
  release` with `.wasm` sizes within ≤0.1% of the `cpu-opt3` record; `cold.py`
  `w3-4-seed` → `w3-4-warm` + `w3-4-bypass --without-cache`; report
  `boardstudio_core_engine` (remnant) and the largest new unit
  (`boardstudio_core_mirrored_half` lib + test) against the 90.3 s / 121.7 s baseline
  (target <45 s / <65 s for the single largest unit, with the honest caveat in §7).
  Stop before push/PR.

## 7. Risks and open questions for implementation slices

- **Mirrored test binary stays chunky.** 370 tests / ≈29k test LOC move together; the
  win is parallelism (it no longer serializes the other 347 engine tests) and a 22k
  lib, not a small test unit. If Phase 3 misses <45 s/<65 s on that one binary, the
  follow-up is sharding mirrored tests by submodule into `[[test]]` integration
  targets — out of scope here, to be reported honestly rather than hidden.
- **`pub(crate)` → `pub` widening** enlarges the de-facto API inside the workspace
  (still `publish = false`). Each slice widens only what the compiler demands; the
  facade surface (`boardstudio_core::*`, `boardstudio_web_layout::*`) does not grow.
- **Callback threading depth** in mirrored (`apply_unscripted` staging sites) and in
  `modules::resolve` preview usage is bounded but must be confirmed function-by-function
  in slices S1/S3; if threading turns invasive, the fallback for deletion is returning
  the pending `RemoveMatrix` ids to the caller instead of dispatching inline.
- **Dev-dependency cycles** (new crates' tests → remnant engine `test-support`;
  possibly objects tests → layout) are legal Cargo, but each one re-serializes that
  test binary behind the engine/layout lib. Keep them test-only, and prefer moving a
  test file up a layer when it only uses public items.
- **Exact per-crate dependency lists** (e.g. whether artifact needs `zip`, whether
  kernel needs `serde` directly, whether `anchor_badge` needs Dioxus) are settled by
  the compiler per slice (`cargo check` after trimming), not by this document —
  manifest warnings must remain 0 and `cargo deny` clean.
- **Doc links:** `docs/` cites the old paths in ADRs and historical plans. Fix only
  what `check-doc-links.py` flags; do not rewrite historical plan documents.

## 8. Implementation outcome

The split landed as designed, in the planned slice order (S1 in-place cycle-breakers,
S2 kernel, S3 kicad-source, S4 modules, S5 electrical, S6 mechanical, S7 artifact,
S8 dispatch threading + mirrored-half; then T1 cycle break, T2 layout-core,
T3 layout-objects), with two architectural corrections found during verification:

1. **Release size duplication was architectural, and was refactored, not waived.**
   First measurement after the split (identical release scripts, baseline built in a
   temp worktree at `aba05d895`): core-worker +21.7%, cad-worker +24.7%, page +1.3%.
   Symbol-level attribution (`.evidence/w3-4-release-dup-*.log`) showed ~3.8 MB of
   generic instantiations repeated per split crate over the shared document types:
   serde `ContentDeserializer`/Deserialize (~1.8 MB), `i_overlay` (~0.65 MB),
   `alloc` BTree/Vec Clone and drop glue (~1.3 MB). Fat LTO alone recovered only to
   +12.1%. The refactor therefore: (a) moved the worker protocol JSON entry points
   into `boardstudio_model::wire` so one crate instantiates them; (b) centralized
   even-odd contour overlay and miter-offset operations in
   `boardstudio_core_kernel::geometry` (and the signed opening offset in kernel
   `case`), removing direct `i_overlay` dependencies from engine, modules, and
   mechanical; (c) folded `boardstudio-core-modules` and
   `boardstudio-core-electrical` back into `boardstudio-core-mechanical` — they had
   no consumer that does not already depend on mechanical, so their separate
   instantiation copies bought almost no parallelism (0 and 3 unit tests); and
   (d) set `[profile.release] codegen-units = 1, opt-level = 2`, restoring the
   within-crate monomorphization sharing the monoliths had without letting the
   single-unit inliner re-inflate the large serde visitors; and (e) kept the
   `ProjectDoc` and `MechanicalConfiguration` clones out of line in
   `boardstudio-model` (manual field-wise `Clone` impls identical to the
   derives) so whole-document clones share one body instead of one per
   calling crate.
2. **Final DAG (core):** model → kernel, kicad-source → mechanical (incl. modules,
   electrical, keymap) → artifact, mirrored-half → engine (remnant) → facade.
   `boardstudio-core-engine` is 9,022 LOC; largest units are mirrored-half
   (51,971 total / ~22k production + 370 tests) and web layout-objects (42,843).

Measured outcomes at the final tree (full `check.py repo tooling lint build browser
security deny release` + `test` all pass):

- Native tests: **2,454 = 2,454** multiset-identical to `aba05d895` after the five
  documented test-path renames (3 kicad-source, lifecycle tests, grid-rounding);
  901 Core integration names byte-identical.
- Browser: **799 crate tests across 14 launches** (layout 48 + objects 178),
  126 page, 15 drag, 364 CAD browser; `libweb_sys` 1 copy,
  `libboardstudio_web_host` 2 copies in `wasm-check/deps`.
- Release `.wasm` vs `aba05d895` built identically (final tree, committed profile):
  page **−13.53%**, core-worker **−1.56%**, cad-worker **−0.90%**, CAD +0.002%
  (+124 bytes), renderer −12.44%, offline −13.77%; aggregate shipped bytes
  **−7.46%**. No artifact grows by more than 0.1% over the baseline; the only
  growth at all is the CAD artifact's 124 bytes.
- Cold checkpoint (`w3-4-seed` → `w3-4-warm` → `w3-4-bypass --without-cache`,
  logs in `.evidence/cold-build-port/`): warm **380.6 s** wall / 1,930 s compiler
  CPU (882/882 sccache hits), bypass **470.5 s** / 3,039 s, vs the `cpu-opt3`
  baseline of 310.1 s / 1,763 s and 416.2 s / 2,809 s. The targeted single-unit
  outcome is met with margin: the former 90.3 s warm / 121.7 s bypass
  `boardstudio_core_engine` test unit is now 18.0 s / 19.5 s for the remnant
  engine, and the largest descendant unit (mirrored-half test) is 44.5 s / 42.0 s.
  Total cold wall and compiler CPU nevertheless regress (~+23% / +9% warm):
  the split's extra generic-instantiation and unit overhead outweighs the
  critical-path win at whole-workspace scale, where CAD, the page, and `web_sys`
  dominate. That trade — much faster localized edit/test cycles and no single
  serializing unit, at higher total cold CPU — is the honest cost of this split
  and should be weighed before landing.

**Follow-up: web consumers off the engine edge.** `boardstudio-application` no longer
depends on `boardstudio_core`: `Session::new` takes Core's Open admission check as an
`OpenValidator` function pointer (hosts pass `CoreEngine::validate_open`), the uncalled
`handle_core` adapter is gone, and document types come from `boardstudio-model`.
`ui-model` and `layout-objects` now depend on `boardstudio-model` / `boardstudio-core-kernel`
directly (`boardstudio_core` only as a dev-dependency); `layout-core` keeps one normal
`boardstudio_core` edge because `preview_anchor_poses` opens its probe through a scratch
`CoreEngine`. The facade re-exports each item from its owning crate. The web graph still
waits for the engine, though: `ui-model` needs `runtime::Runtime`, which owns the
in-process `CoreEngine` executor plus archive, artifact and firmware entry points, and
`runtime` depends on `host`, whose CAD capture calls the engine's
`case_preparation_ready`; `case` (`link_preview`) and `layout-core` also open scratch
engines in production. Lifting that gate means moving the engine-free readiness checks
and `migrate_value` out of the engine and putting `Runtime`'s executor behind an injected
trait — well beyond a contained slice.

**Follow-up: engine gate lifted.** `case_preparation_ready` and
`authored_case_geometry_ready` moved to `boardstudio_model::case_readiness`. `migrate` moved
to the kernel, and `MAX_PROJECT_ARCHIVE_BYTES` moved to `boardstudio_model::model`. The
facade still exports every name. `Runtime`'s production executor was already the Core worker,
and the in-process `CoreEngine` executor is test-support only. The remaining synchronous engine
calls now go through `runtime::core_services::CoreServices`, a pair of function pointers that
the page builds and passes to `Runtime::new`: Open admission, and a scratch-engine factory.
Three probes use the factory: `link_preview`, `preview_anchor_poses` and the library's
demo-project builder. Now `cargo tree -i boardstudio-core-engine -e normal` (host and wasm)
reaches only `boardstudio-web`. Before, it also reached 15 other web crates
(`.evidence/w3-4-engine-gate-tree-{before,after}-{host,wasm}.txt`). The 12 `migrate::tests`
names moved from engine to kernel, and the 2,454-name multiset is otherwise unchanged.


### Test-only dev-dependency cycles removed

Follow-up on §3.1 item 8 and §7 (`refactor/core/remove-test-cycles`). The kernel,
kicad-source and mechanical test binaries could not start compiling until the whole
engine graph was built (warm cold run: kernel tests started at 75.3 s although the kernel
library was ready near 60 s; mechanical at 100.4 s; kicad-source at 102.5 s). Their
dev-dependencies on `boardstudio-core-engine` (and `boardstudio-script`) are gone. The
eight tests that actually drove `CoreEngine` or the artifact request moved one layer up and
use public items only; every non-engine test stayed where it was:

- `core/crates/engine/tests/matrix_through_engine.rs`: kernel `matrix::tests` companion
  pose across refresh and `Open`.
- `core/crates/engine/tests/framed_constraints_through_engine.rs`: kernel
  `constraints::framed_tests` legacy-offset `Open` reconciliation and the two
  endpoint-versus-derive companion parity tests (these call
  `mirrored_half::companion_placement` directly, so the engine's
  `companion_placement_for_test` window is deleted).
- `core/crates/engine/tests/mechanical_through_engine.rs`: mechanical `clearance::tests`
  MH-reference boundary tests and `edit::tests` link-then-undo.
- `core/crates/artifact/tests/mechanical_extract_request.rs`: kicad-source
  `mechanical_extract::tests` artifact request (artifact already depends on kicad-source).

The mirrored-half cycle remains, deliberately. Its tests are not a few engine users that can
be lifted out: `EditDispatch` (`test_dispatch()`) is passed to private entry points
(`lifecycle::prepare_lifecycle_transition`, `prepare_exception_intent`,
`transition_exception_context`, `lifecycle_context`, `reconcile` and the prepare helpers) from
about 230 call sites across `lifecycle/tests.rs`, `reconcile/tests.rs`,
`reconcile/prepare/tests.rs`, `matrix_addition/*tests.rs`, `settings_tests.rs` and
`inverse/tests.rs`, and those modules share fixtures (`reconcile::tests::fixture_document`,
`matrix_addition::tests::pair_document`) and `use super::*` private types with the 100+ tests
that never touch the engine. Moving them to `engine/tests/` would need a `test-support`
feature on mirrored-half re-exporting dozens of `pub(super)`/private functions, types and
fields plus the fixture modules compiled into the library, which is broad widening in exchange
for the sharding that §7 already records as optional. The test binary remains the largest unit
(about 44 s) but is no longer on the critical path of kernel, kicad-source or mechanical tests.
The engine `test-support` feature stays for it, as do the `*_for_test` windows it uses.
