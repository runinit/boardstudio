# Duplication & refactoring opportunities — BoardStudio research report

**Source of truth:** current repo checkout only. Git HEAD `5f8d3ac39974d5801f04ee2d327d604eb84c973e` (Sun Oct 18 2026), working tree clean at start (`git status` empty, no diff vs HEAD). No builds, tests, installs, production edits, or commits were performed. This is a research document only; it proposes no code change.

**Method:** read connected implementations and representative tests before proposing sharing. Distinguished *textual similarity* from *duplicated policy*, and *moving code* from *net deletion*. Rejected candidates that are distinct contracts (not deduplicated) are listed explicitly with reasons. No new traits/frameworks, no crate splitting by size, no collapsing of strict gesture/export identity paths into ordinary settlement.

---

## Parent verification and corrected recommendation

The parent independently compared both renderer files and both `OwnedEdits` files after the research agent returned. **Duplication is confirmed; implementation safety is not.** No builds or tests were run.

- **Best small first slice: `OwnedEdits`.** The copies differ in visibility, a tuple alias, and Keymap's `bound_draft`. Sharing them would widen visibility across crates, so approval is required under `AGENTS.md`; preserve owner retirement, signal rebinding and submitted-draft behavior. Estimate roughly 90–100 lines net deletion before any added tests.
- **Largest candidate: renderer snapshot consolidation.** This is explicitly a temporary private snapshot, not an accidental copy. [`renderer_host_page.rs`](https://github.com/runinit/boardstudio/blob/5f8d3ac39974d5801f04ee2d327d604eb84c973e/web/crates/runtime/src/renderer_host_page.rs#L1) documents why the unchecked method is omitted; [`renderer_host_source_sync.rs`](https://github.com/runinit/boardstudio/blob/5f8d3ac39974d5801f04ee2d327d604eb84c973e/web/crates/runtime/src/renderer_host_source_sync.rs#L1) guards exact normalized equality. [`RendererPageHost::submit_scene`](https://github.com/runinit/boardstudio/blob/5f8d3ac39974d5801f04ee2d327d604eb84c973e/web/crates/runtime/src/renderer_host_page_extensions.rs#L117) and adjacent methods access private `host.inner`, `call_method` and `schedule_frame`. A re-export alone therefore cannot replace the snapshot. Design a narrow checked host interface first; do not expose internal fields or substitute unchecked `update_scene`. Gross duplicate removal is 927 lines; net deletion is **unknown** until the interface and tests are designed. Any visibility widening needs approval.
- **Footprint helper extraction is real but not yet scoped precisely.** The identical pin table, `pin_label`, `traces_row` and `traces` span approximately lines 105–209 in [`mcu_nice_nano.rs`](../../footprints/src/generators/ceoloide/mcu_nice_nano.rs#L105), offset by one line in [`mcu_supermini_nrf52840.rs`](../../footprints/src/generators/ceoloide/mcu_supermini_nrf52840.rs#L106). `EXTRA_PIN_ROW` differs and is used outside those trace helpers. Do not parameterize helpers with values they do not use. Net savings remain an estimate pending a scoped diff; preserve exact generated geometry/net assignments and both part-specific extra-pin rows.
- **Python loaders: lower-priority lead, not a validated 17-file common harness.** Repeated `importlib` loading is observable, but test files load different tools; common browser mocks across all files were not established. No mixin or extension to production `check.py` is justified by this evidence. Inventory exact loader requirements before estimating savings.
- **Coverage limit:** the CAD qualification/production copy leads supplied during research were not resolved in the returned report. Mechanical-feature comparison also did not exhaust every repeated lookup. Treat these as open research, not rejected duplication. “Already in one file” alone does not prove field mappings cannot be consolidated internally.

Recommended order: investigate the small `OwnedEdits` consolidation first (subject to visibility approval), design renderer sharing second, extract narrowly shared MCU helpers third, defer Python harness cleanup. Rank by maintenance value and behavior risk, not line count alone.

The agent's detailed candidate notes below are retained with corrections. Their unverified savings and safety judgments are not acceptance evidence.

## Scope and framing

Per the repository's documented architecture (`docs/architecture.md`), every product edit is submitted as intent and settled through one edit ticket; Core's typed `ApplyMechanicalEdit` owns mechanical settings rules; gesture/export routes keep strict captured-revision identity. The edit-settlement cleanup audit (`docs/investigations/edit-settlement-cleanup-audit.md`) confirms remaining submission records retain distinct owner/post-landing semantics. I did not attempt to collapse those.

Five clusters were investigated as requested, plus three additional leads surfaced by an exact-token scan:

| # | Cluster | Status |
|---|---------|--------|
| 1 | renderer host (`RendererHost`) copied between `host` and `runtime` | **Confirmed — strongest** |
| 2 | `OwnedEdits` helper copied between `case` and `keymap` | **Confirmed** |
| 3 | footprint generator `traces_row`/`pin_label` logic copied between two MCU generators | **Confirmed, modest** |
| 4 | Python tool test-loader boilerplate across `scripts/test-*.py` | **Confirmed, small/standard** |
| 5 | mechanical-settings field metadata / patch mapping | Investigated — already centralized (not a finding) |
| 6 | accepted-edit resolver / submission plumbing (`layout_component_edits.rs`) | Investigated — distinct contracts per action (rejected) |
| 7 | mirrored-half family-specific feature lookup | Investigated — distinct operations (rejected) |
| 8 | switch generator `PARAMS` tables (`switch_mx` vs `switch_gateron_ks27_ks33`) | Investigated — distinct part data (rejected) |

Only 1–4 are carried forward as ranked recommendations; 5–8 are documented as rejected/investigated.

---

## Ranked findings

### 1. `RendererHost` duplicated in full between `web/crates/host` and `web/crates/runtime` (~920 lines) — Rank 1 (strongest)

**Exact repeated block.** `web/crates/host/src/renderer_host.rs` (934 lines, active under `#[cfg(all(target_arch = "wasm32", feature = "page"))]`, declared at `web/crates/host/src/lib.rs:31`) and `web/crates/runtime/src/renderer_host_page_base.rs` (927 lines, textually included into `renderer_host_page.rs` at line 8 via `include!("renderer_host_page_base.rs")`) define the same `RendererHost` + `RendererInner` + `mount`/`attach` machinery. A diff excluding only comment style (`//!` vs `//`) shows the sole substantive difference is host's extra `update_scene` method (~7 lines). Imports, both structs (all fields), `mount`, and `attach` are line-for-line identical.

**Why changes must currently be repeated.** `runtime` already depends on `host` (`boardstudio-web-host = { path = "../host", features = ["page"] }`, `web/crates/runtime/Cargo.toml`), yet re-defines `RendererHost` instead of reusing host's. Any change to the attachment-lifetime guard, disposal tracking, or frame scheduling must be mirrored in both files.

**Refactor prerequisite.** Keep one canonical host implementation only after providing the narrow operations required by `RendererPageHost`. The wrapper currently reaches private internals in the included snapshot; importing the other crate's type is not a working replacement. Preserve increasing scene sequence, the boolean scene-acceptance result, liveness checks and conditional frame scheduling. Do not replace these with unchecked `update_scene` or make internal fields public.

**Net savings.** Gross duplicate removal: 927 lines. Net savings are unknown until the shared operations and regression tests are designed; import/re-export plumbing alone is insufficient.

**Public-interface impact.** Both define `pub struct RendererHost` in different crates (`boardstudio_web_host::renderer_host` vs `boardstudio_web_runtime::renderer_host_page_base`). Collapsing to host changes the path runtime code uses; any runtime module naming `renderer_host_page_base::RendererHost` must be re-pointed. The include-based text sharing in `renderer_host_page.rs` would be replaced by a normal dependency import.

**Behavior risks.** Material: runtime intentionally omits unchecked `update_scene` and implements sequence-checked submission through private access. Consolidation must preserve that stronger contract, attachment lifetime, cleanup and scheduling. Native source equality currently detects snapshot drift, but does not prove browser behavior.

**Existing tests / verification.** [`renderer_host_source_sync.rs`](https://github.com/runinit/boardstudio/blob/5f8d3ac39974d5801f04ee2d327d604eb84c973e/web/crates/runtime/src/renderer_host_source_sync.rs#L1) is a native equality test, not a production caller. Add focused checks for non-increasing sequence rejection, rejected-scene scheduling, liveness and cleanup before changing ownership. Runtime is not a presentation-crate browser launch; inspect the supported test scopes before choosing commands. Native tests alone do not compile or validate the page-only WASM path; page compilation and relevant mounted/packaged renderer checks are required. No verification commands were executed. Confidence: **high** on duplication, **unresolved** on the exact consolidation design.

**Ordering.** Independently implementable first; no other candidate depends on it.

---

### 2. `OwnedEdits<K>` helper copied between `case` and `keymap` (~100 lines) — Rank 2

**Exact repeated block.** `web/crates/case/src/owned_edits.rs` (100 lines) and `web/crates/keymap/src/keymap/owned_edits.rs` (108 lines) are identical modulo: doc comment ("Case controller" vs "Keymap controller"), visibility (`pub(crate)` vs `pub(super)`), a `type BoundField<K>` alias in `case` vs an inline tuple in `keymap` (structurally identical), and keymap's extra `bound_draft` method (lines 76–82). All logic — `owner_changed`, `follow_owner`, `bind_field`, `unbind_field`, `remember`, `is_pending`, `pending`, `has_terminal`, `prune` — is the same generic owner-lifetime edit-observation helper.

**Why changes must currently be repeated.** Both observe one controller's edits for one `Scope`/generation lifetime, retiring old observations on owner change and re-binding field Signals across owner changes. The logic is not policy; it is a reusable generic data structure. Any fix to owner-change retirement or `prune` must be applied to both.

**Smallest coherent refactor yielding net deletion.** Move the generic `OwnedEdits<K>` into `web/crates/ui-shared` (both crates already depend on `boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals`) as a single `pub` type, and have `case` and `keymap` use it. keymap's `bound_draft` either moves along or stays as a thin keymap wrapper. Delete both local copies.

**Net savings.** Honest estimate: ~90–100 lines deleted (two ~100-line files removed, one ~100-line shared file added → net ~90–100 lines removed). BoundField alias unification removes the trivial alias/inline difference.

**Public-interface impact.** Visibility changes from `pub(crate)`/`pub(super)` to `pub` in ui-shared. Callers within `case` and `keymap` re-point their `use`. `bound_draft` is `pub(super)` in keymap only; case has no equivalent, so no contract conflict.

**Behavior risks.** Bounded but nonzero: owner-change retirement, field rebinding and submitted drafts must remain identical. Cross-crate sharing widens visibility and requires approval under `AGENTS.md`; it is not just a private file move. Existing indirect panel tests are useful but not proof that every helper contract is covered.

**Existing tests / verification.** Covered by case and keymap panel/browser tests (`python3 scripts/check.py browser --only case keymap`; subset only, not full acceptance). Confidence: **high**.

**Ordering.** Independently implementable; lower risk than finding 1, good second task.

---

### 3. Footprint generator `traces_row` / `pin_label` copied between two MCU generators (~60 lines × 2) — Rank 3 (modest)

**Exact repeated block.** `footprints/src/generators/ceoloide/mcu_nice_nano.rs` and `footprints/src/generators/ceoloide/mcu_supermini_nrf52840.rs` both define identical `fn pin_label`, identical `const PIN_NAMES`, and an identical `fn traces_row` (same KiCad segment template, same `e0`–`e23` coordinate expressions). `EXTRA_PIN_ROW` differs (9 vs 10), and `PARAMS`/`SPEC` differ (legit: distinct part data — different 3D models, pin counts, features). Confirmed via `grep -rln "fn traces_row"`: exactly these two files.

**Why changes must currently be repeated.** The trace-segment drawing logic is generic to this 2×12-pin MCU layout; it is not part-specific. A change to trace width/layer/net assignment must be edited in both generators.

**Smallest coherent refactor yielding net deletion.** Extract `pin_label` and `traces_row` (parameterized by `PIN_NAMES`/`EXTRA_PIN_ROW` and the coordinate constants) into `footprints/src/generators/util.rs`, call from both generators. Keep `PARAMS` and `SPEC` in each file (distinct data, not deduplicated — see rejected candidate #8).

**Net savings.** Honest estimate: ~30–50 lines deleted (two ~60-line copies replaced by one shared helper + two call sites). Modest; only two generators share this exact layout.

**Public-interface impact.** `util.rs` items are `pub(crate)` already; both generators are in the same crate (`boardstudio-footprints`), so no cross-crate change.

**Behavior risks.** Low — same crate, pure extraction. The shared helper must preserve the `EXTRA_PIN_ROW` and coordinate constants per generator (pass as parameters).

**Existing tests / verification.** Footprint generator tests (native `cargo test -p boardstudio-footprints`). Confidence: **medium** (real duplication, but small and only two sites).

**Ordering.** Independently implementable; do after findings 1–2 if footprint work is already planned; otherwise defer.

---

### 4. Python tool test-loader boilerplate across `scripts/test-*.py` (~10 lines × 17 files) — Rank 4 (small, standard harness)

**Exact repeated block.** 17 files under `scripts/` (e.g. `scripts/test-check.py`, `scripts/test-check-quick.py`, `scripts/test-build-web.py`, `scripts/test-serve-web.py`, …; 24 test files total, 17 use the importlib loader) open with the same ~10-line harness:

```python
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))  # or parents[1]/scripts
import check  # noqa: E402
...
```

plus a `setUp` that mocks `check.browser_tools.locate`. Confirmed in `scripts/test-check.py:1`–`30` and `scripts/test-check-quick.py:1`–`20`.

**Why changes must currently be repeated.** Each test file independently loads `check.py`/`check_quick` and mocks browser-tool location. A change to the loader protocol must be repeated in all 17.

**Smallest coherent refactor yielding net deletion.** Extract a shared `test_harness.py` module (or extend `check.py`) with `load_checked_module(name)` and a `BrowserToolsMocker` mixin, and have the 17 test files import it. Keep each file's assertions.

**Net savings.** Honest estimate: ~120–170 lines deleted across 17 files (~7–10 lines each). Small per-file; real but low-value aggregate.

**Public-interface impact.** None external; test-only.

**Behavior risks.** Low — test harness only. Must preserve the `mock.patch.object(check.browser_tools, "locate", …)` semantics and `ROOT` path resolution.

**Existing tests / verification.** `scripts/test-check.py` and `scripts/test-check-quick.py` themselves test `check.py`/`check_quick`; the harness change is covered by those. Confidence: **medium-high** on duplication; **low** on value (standard unittest boilerplate, not product logic).

**Ordering.** Independently implementable; lowest priority, cosmetic-adjacent (moving boilerplate, not product deletion). Consider deferring.

---

## Rejected / investigated-not-a-finding candidates

### Investigated: mechanical-settings field metadata / patch mapping — already centralized (not duplicated)

`web/crates/case/src/mechanical_settings.rs` defines `MechanicalDimensionExt::{field_id, rule, step}` (lines 284–387), `MechanicalSettingsPatchExt::field_id` (lines 389–569), `mount_collection_id` (571–576), and `is_dimension_field` (235–282) — all in one file. `web/src/presentation/case_viewer.rs` references `MechanicalSettingsPatchExt` but defines no duplicate `field_id` mapping (grep confirmed no `fn field_id` / `impl MechanicalSettingsPatchExt` in case_viewer.rs). The `MechanicalTextIntent::patch` (lines 198–233) maps text intents to patches in one place. Conclusion: this metadata/patch mapping is **already centralized**; no duplication to remove. Do not open a refactor here.

### Rejected: accepted-edit resolver / submission plumbing (`layout_component_edits.rs`) — distinct contracts

`web/src/presentation/layout_component_edits.rs` builds many `EditResolver::new(...)` instances (position, outline, nudge, remove_constraint, etc., e.g. lines 77, 144, 199, 234, 397, 474, 506, 564, 620, 651). Each resolver carries distinct business logic, distinct `Resolution::Retire` messages, and distinct edit semantics per action. Per the edit-settlement audit, every action uses its own resolver; these are **distinct contracts**, not duplicated blocks. Do not deduplicate — collapsing them would merge unrelated edit policies and change Undo/rejection semantics. Not a finding.

### Rejected: mirrored-half family-specific feature lookup — distinct operations

`web/crates/case/src/mechanical_pair.rs` performs pair projection (`pair_projection`, `member_record`, fingerprinting) while `core/src/mechanical/edit/replacement.rs` performs replacement validation (`feature_ids`, `find_feature_value`, `reject_omitted_reintroduction`, `validate_unambiguous_feature_ids`). They share concepts (features, members, pairs) but are different operations on different data; no identical function/block repeats. Not a finding.

### Rejected: switch generator `PARAMS` tables — distinct part data

`footprints/src/generators/ceoloide/switch_mx.rs` (41 params) vs `switch_gateron_ks27_ks33.rs` (34 params) differ in parameter sets, 3D models, and features (hotswap, solder, stabilizers, etc.). These are distinct part data tables, not duplicated logic. Not a finding.

### Explicitly out of scope (per prior-audit notes)

- `renderer/vendor/cgmath-0.18.0` — explicitly retained unused fallback, not a requested code-duplication refactor.
- `core/src/mechanical/edit.rs` `reconcile/prepare.rs` — ACTIVE via reexports into `core/src/lib.rs:246/560/750` despite stale ticket-111 dormant comments; not dead code.
- `layout_component_inspector_tests.rs` — already a separate file.
- `catalogue_json.py` — only serializer; importer is `import-vik-modules.py`.
- Match counts do not establish bad design; earlier LOC claims are not accepted as proof of bloat.

---

## Independent implementation order (recommended)

1. **`OwnedEdits` consolidation** — smallest well-bounded shared helper; obtain visibility approval and preserve owner/draft behavior.
2. **Renderer snapshot sharing design** — largest gross duplication; resolve private access and preserve checked scene submission before implementation or claiming net savings.
3. **MCU trace/label helpers** — keep geometry and distinct extra-pin behavior unchanged; determine exact savings from a narrow diff.
4. **Python loader inventory** — defer; shared mocks and savings across all claimed sites are not established.

No finding requires another. No new traits, frameworks, or crate splits are recommended.

---

## Report artifact

This is the only file created/edited: `docs/research/duplication-refactoring-opportunities.md`. No production code, tests, fixtures, config, or builds were touched.
