# Rust audit, first pass (2026-10-30, branch audit/rust-dev-20261030 @ 510861b11)

Tools: cargo-machete 0.9.2 (`--with-metadata`), jscpd 5.4.2 via npx run from outside the repo
(min-tokens 100, min-lines 10, ignore vendor/target/generated). Raw output: `.evidence/audit/`.
Nothing in the repo was modified. No dependency was removed or built; every item below is **unverified until `cargo check` after removal**.

## 1. Actionable (candidates, verify before fixing)

| # | Where | Finding | Evidence | Verify with |
|---|-------|---------|----------|-------------|
| A1 | `web/crates/{keycaps,keymap,layout,library,parts,pcb,catalogue,runtime}/Cargo.toml` | 19 deps flagged, no code use in any of them (one comment mention of serde in `layout`) (full list in `machete-findings.txt`). Includes `boardstudio-web-host` x4, `serde` x5, `sha2` x5, `js-sys` x2, `serde-wasm-bindgen`, `wasm-bindgen-futures`, `boardstudio-application` | grep of each crate's `.rs` files | remove, then `cargo check --all-targets` for host **and** wasm32 target, plus each feature set |
| A2 | `web/Cargo.toml` | `boardstudio-web-catalogue`, `futures-channel`, `serde-wasm-bindgen`, `sha2` have no reference in `web/src`, `web/tests`, `web/examples` (the `web/crates/*` hits are those crates' own deps) | grep | as A1; `catalogue` and `sha2` are optional and feature-listed in `page`/`cad-worker`, so the feature lists must change too |
| A3 | `web/crates/pcb/src/pcb_wiring.rs:1172` vs `pcb_wiring/mode_owner_tests.rs:116` | `test_fixtures` (`document`, `test_part`, `test_definition`, about 168 lines) is copied into `mode_owner_tests.rs`. **Verified identical** apart from `pub(crate)` and the field order of `anchor: None`. **Cause is structural:** on native builds `lib.rs:94` replaces `pcb_wiring` with a shim that includes `mode_owner_tests.rs` by `#[path]`, and `test_fixtures` lives in the wasm-only `pcb_wiring.rs`, so the native tests cannot see it | diff of both bodies | one shared fixtures file included by both modules via `#[path]`; run native pcb tests and the wasm tests |
| A4 | `web/crates/runtime/src/runtime.rs:7689-7758` vs `core/tests/electrical_wiring.rs:6,75` | 85-line clone between a core integration test and the runtime test module; same fixture in two crates | jscpd exact | decide the owner (core test-support?) |
| A5 | `web/crates/layout/.../matrix_setup_controller.rs:472` vs `mirrored_pair_controller.rs:893`; `keymap/binding_controller.rs:837` vs `macro_controller.rs:549` | Near-miss clones (similar/ast) between sibling controllers | jscpd similar | read both; extract only if they must change together |

## 2. Intentional duplication / dependencies

- `cad/monstertruck-qualification/src/unit_normalization.rs` <-> `cad/monstertruck/src/step/units.rs` (about 3,500 lines, only about 179 differing lines) and `step_gate.rs` <-> `step/mod.rs`, `step_meshing.rs` <-> `step/tessellation.rs`: documented as "Gate-binary containment copies ... independent oracles" (`docs/plans/cad-deepening/handoff.md:26`). Keep. Maybe pin with a comment/test that drift is deliberate.
- `footprints/src/generators/ceoloide/mcu_nice_nano.rs` <-> `mcu_supermini_nrf52840.rs` (526 lines), `infused_kim/conn_molex_pico_ezmate_1x02` <-> `1x05`: per-part footprint generators. Likely intentional (pin-compatible variants); a parametrised helper is possible but trades review clarity.
- 244 of 558 exact clones are test-to-test (mostly `web/tests/*`). Treat as fixture duplication, low priority.

## 3. False positives

- `tokio` in `web/Cargo.toml` (`[dev-dependencies]`): used by `web/tests/*.rs` (`#[tokio::test]`, `tokio::time::timeout`).
- All machete and jscpd hits under `**/vendor/**` (`monstertruck-*`, `chromiumoxide`) are out of scope and were excluded from this report.

## 4. Open questions

- Why do ~9 `web/crates/*` crates carry deps they never use? Leftovers from extraction out of `web`, or planned use (rust-ui migration)? If planned, add to `[package.metadata.cargo-machete] ignored` with a reason instead of deleting.
- Is machete blind to anything here? `--with-metadata` resolves names but is still text-based; macro-generated or `cfg`-gated paths (many deps are in `cfg(target_arch = "wasm32")`) need the wasm build to confirm.
- Which test files should be exempt from the duplication gate (fixtures in `web/tests/*`)?
- Is `cad/wasm` meant to stay out of the workspace (own lockfile)? Machete reports it clean.
- `tools/browser-harness` could only be analysed with the sandbox lifted (cargo registry is read-only in the sandbox); only vendored `chromiumoxide` was flagged.

## 5. Not yet done

- Step 3 for tests: no mutation checks run yet. Suggested first targets: A3 (`mode_owner_tests`), and any test whose only difference from its twin is a fixture.
- `cargo-shear` / `cargo-udeps` cross-check (not installed).
- jscpd pass on tests-only, `--ignore-identifiers` renamed-clone pass.

## Running log

- 2026-10-30: step 1 (machete), step 2 (jscpd exact + similarity), first report. Report moved from `.evidence/audit/` to this file; raw output stays in `.evidence/audit/` (gitignored).
- 2026-10-30: step 3 started on the pcb mode/apply gates. Baseline `cargo test -p boardstudio-web-pcb --locked --lib mode_owner`: 17 passed. Mutation spec `.evidence/audit/pcb-mutations.json` (8 gate-neutralising mutants in `mode.rs` / `apply.rs`), run with `scripts/mutation-check.py`. Results below.

## Mutation results

Spec `.evidence/audit/pcb-mutations.json`, command `cargo test -p boardstudio-web-pcb --locked --lib` (all 17 `mode_owner_tests` are native `#[test]`s, none wasm-only, so the survivors are not explained by a missing wasm run). Working tree restored; evidence in `.evidence/audit/pcb-mut/`.

| Mutant | Neutralised check | Result |
|--------|-------------------|--------|
| m1 | `mode.rs` `identity != &request.identity.plan` | survived |
| m2 | `mode.rs` `current_mode == request.mode` -> `Resolution::Unchanged` | survived |
| m3 | `mode.rs` `active_board_id` mismatch | survived |
| m4 | `mode.rs` `active_instance_id` mismatch | survived |
| m5 | `mode.rs` `session_epoch` mismatch | survived |
| m6 | `mode.rs` `document.id` mismatch | survived |
| m7 | `apply.rs` retire when board mode changed | **killed** (1 test) |
| m8 | `apply.rs` `plan.revision != identity.revision` | survived |

**Finding T1 (actionable, needs judgement).** 7 of 8 survived.
- `mounted_owner_rejects_retained_controls_after_each_source_identity_change` (`mode_owner_tests.rs:871`) loops six "changed" dimensions and asserts only that `mutation_count` is unchanged. It cannot say *which* gate rejected the edit, so any single gate can be removed while another still rejects.
- No test in `web/` references `CurrentSnapshotBlocker::{ActiveBoard,ActiveInstance,SessionEpoch,DocumentId}`; those four checks (`mode.rs:371-381`) have no direct coverage.
- No test submits a same-mode request and expects `Resolution::Unchanged` (m2).
- Open question for review: are m1/m3-m6/m8 *equivalent mutants* (defence in depth: `matches_action_context`, runtime-level rejection also catch it), or genuinely redundant gates that could be deleted? The answer decides test-vs-delete. Next step: re-run each survivor with a direct `current_snapshot_probe` assertion, or mutate the *other* layer too and see whether both must be removed.
- Mutation confidence caveat: one mutation per gate, neutralised via `&& false`; no mutant tried the inverted condition.

## For the deeper review (remove / refactor / redesign)

- A4 (verified): `definition()` in `core/tests/electrical_wiring.rs:6` and `runtime.rs:7689` is identical apart from field order; `wired_document` differs only in name and a trailing `doc.hardware` block. The `PartDefinition` builder pattern appears in about 9 files (`core/tests/{contracts,core,electrical_wiring,matrix_performance}.rs`, `core/src/artifact/kicad.rs`, `parts_custom_definition.rs`, pcb x2, runtime). A real fix needs shared test API in core, so it is a design decision, not a mechanical dedupe.
- Architecture review of A3/A4 (Opus subagent, codebase-design vocabulary; read-only, nothing compiled):
  - Corrections to the audit: the kicad fixtures are in `kicad/output/tests.rs:46,122`, not `kicad.rs`; `core/tests/contracts.rs` has no copy. Only two pairs are the same concept: pcb `test_fixtures`/`mode_owner_tests`, and `electrical_wiring`/`runtime`. The rest stay local. `PartDefinition` has no `Default`, so a new field breaks every literal loudly and copies cannot silently drift on it.
  - Refactor: move `test_fixtures` to `pcb_wiring/test_fixtures.rs`, declared `#[cfg(test)]` in both module trees; delete the `mode_owner_tests.rs` copy.
  - Optional refactor: shared `definition`/`wired_document` in `core/tests/support/wiring.rs`, included by `#[path]` (as `web/crates/host/src/lib.rs:21` does). Needs a browser test run.
  - Rejected: a core `test-support` feature (adds fixtures to core's public interface); a dev-only fixtures crate (shallow; dev-dependency cycle gives core unit tests two `boardstudio_core` types, inferred).
  - Redesign later, pcb only: always compile `pcb_wiring` and move `cfg` to its browser-only leaves. The module tree is declared twice (`lib.rs:81`, `:93`) and `lib.rs:8` allows `dead_code`/`unused_imports` natively. Known blockers: `controller.rs:27` (`spawn_local`) and `crate::runtime`. Needs a spike; 1-2 days is an estimate. Native `#[path]` shims appear in 6 of 12 crates; do not sweep them.
- A1/A2: dependency removal batch. Note `boardstudio-web-host` entries carried `features = ["page"]`: removal can change feature unification, which `cargo check` does not report. Decide leftovers vs planned use first.
- A3: shared fixtures across the native shim and wasm module; a design smell, not just a duplicate (two compile paths for one module).
- Tool notes: jscpd 5.4.2 has `--baseline`/`--changed`, usable to gate new clones later. `cargo-shear`/`udeps` not installed.

## Batch results (2026-10-30, uncommitted)

- **A1/A2 removed:** 23 dependency entries (19 in `web/crates/*`, 4 in `web`) from 9 manifests, plus their `web` feature-list entries (`dep:boardstudio-web-catalogue`, `dep:sha2`). `Cargo.lock` diff is deletions only.
- **A3 done:** fixtures moved to `web/crates/pcb/src/pcb_wiring/test_fixtures.rs`, mounted in both module trees; the `mode_owner_tests.rs` copy is deleted.
- **Gates, run over both batches:** `check.py build`, `browser`, `test` and `lint` all pass. `test`: all 2378 baselined native tests executed. Per-crate `cargo check` passes on host and wasm for every changed crate, and each `web` feature set passes, except the item below.
- **Not caused by this work:** `cargo check -p boardstudio-web --no-default-features --features page` fails on the host with an unresolved `boardstudio_web_runtime::runtime` in `ui-shared`. A clean `HEAD` fails the same way.
- **Not caused by this work:** `check.py test` reports 27 tests missing from the native baseline; this batch adds no tests. Baseline not updated.
- **Not verified:** runtime behaviour change from dropping `boardstudio-web-host` `features = ["page"]` edges. Builds and tests pass, and the shipped `web` crate still enables `boardstudio-web-host/page` itself.

## ADR 0021 (reviewer opinion, for the deeper review)

Keep it; move (a) fits it. Weak points: `runtime_test_stub.rs` grew from 65 to 309 lines and must keep matching the browser Runtime, with only gate behaviour shared via `GateDriver`; no decision covers how wasm-only modules get native tests (source of the shims and the `lib.rs:8` allow). Only alternative worth revisiting: compile the real Runtime natively behind host ports. Trigger: a real bug native tests passed but mounted tests or production caught. No search for such a bug was done.

After removal, `cargo machete --with-metadata web` flags only `tokio` (known false positive).
