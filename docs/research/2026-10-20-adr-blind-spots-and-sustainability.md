# ADR blind spots and application sustainability

**Research date:** 2026-10-20. **Examined HEAD:** `9099b3ef7a72b0919b1572f116490ed57514d43a`.
**Status:** research only; no decisions, implementation tickets or acceptance are created here.

This is a changing, extensively dirty working tree, not an atomic snapshot. At inspection,
ADRs 0013–0031 and `GLOSSARY.md` were untracked; their accepted/deprecated text is current
policy, not commit provenance at the HEAD above. Production files, plans and other research
also had concurrent changes. Only this new report was intentionally written. Recheck the
specific source and ticket before acting.

## Executive answer

**We need a better coverage map and a few precise compatibility decisions, not an ADR for
each remaining module.** The earlier ADR investigations were strongest on accepted-state
durability, captured ownership, CAD and physical topology. Their own exclusions leave
substantial application behavior untraced; neither the number of ADRs nor a module inventory
establishes whole-application assurance ([coverage evidence](#how-coverage-was-reconstructed)).

This investigation deepened **five seams**, rather than repeating the previous five leads:

1. **Layout evolution → input identity → binding cleanup → firmware ordering.** Row deletion
   already moves surviving metadata with the surviving key; generic shrink/disable/removal
   takes materially different paths. The remaining question is the promised lifecycle across
   these operations, not whether a keymap is stored by ID ([D2](#d2-input-identity-across-layout-evolution)).
2. **Catalogue data → placement snapshots → current generator executable → export.** Project
   override precedence does not freeze generator behavior or omitted defaults. There is already
   a documented pad-order migration invariant, but not a complete release compatibility
   promise for changed geometry/defaults ([D1](#d1-project-compatibility-when-generators-change)).
3. **Routed-board import → visual source → physical PCB operand → electrical handoff.** The
   read-only routed source can supply the unpopulated PCB in mechanical STEP/collision while
   electrical export still regenerates the accepted Board. A single “preview-only” label is
   insufficient to explain the source matrix ([D5](#d5-manufacturing-source-authority-is-output-specific)).
4. **Packaged release → offline cache → worker bootstrap → request validation.** Existing
   identity guards and delayed activation are not a build-compatibility handshake. Real
   two-release transition evidence remains missing from the investigations examined here
   ([D3](#d3-release-coherence-is-not-request-freshness)).
5. **Vendor provenance → staged model → portable archive/PCB ZIP.** Bytes travel; rich source
   and licence metadata do not automatically travel with them. This is now a concrete
   propagation gap, not just the earlier attribution research lead ([D4](#d4-portable-provenance-and-distribution-boundaries)).

**Recommended documentation priority:** discuss generator upgrade compatibility and input
identity first. Explain the manufacturing-source matrix in ordinary references; investigate
release transitions and provenance obligations before choosing mechanisms. There are
**two conditional standalone-ADR candidates** here (input identity and provenance), **two
likely existing-ADR amendments** (generator compatibility and release coherence), and **one
ordinary-reference clarification** (manufacturing source authority). **Zero new ADRs are ready
for acceptance from this research alone.** These are dispositions, not a five-ADR queue.

## How coverage was reconstructed

Read [AGENTS](../../AGENTS.md), [domain instructions](../agents/domain.md),
[GLOSSARY](../../GLOSSARY.md), the [ADR index](../adr/README.md), **all records 0001–0031**,
[PRODUCT](../../PRODUCT.md), [DESIGN](../../DESIGN.md), [architecture](../architecture.md)
and the [documentation index](../README.md). The installed domain-modeling skill and
`ADR-FORMAT.md` require meaningful reversal cost, surprising behavior and a real trade-off;
source presence alone satisfies none of the historical-intent requirements.

Coverage below comes from explicit scope, cited production paths, named tests and outcome
limits—not session memory. The principal earlier evidence is:

| Evidence | What it actually inspected or records | What it cannot establish |
| --- | --- | --- |
| [General ADR inventory: method and exclusions](2026-10-19-adr-gap-inventory.md#method-provenance-and-limitations), [G1–G12](2026-10-19-adr-gap-inventory.md#ranked-candidate-table) | Selected Session/storage/export/CAD/archive/native-test/platform/module contracts and their named tests/history. Its subsequent-decision table maps these to 0013–0023. | An exhaustive security, hardware or UI audit; complete firmware, renderer or recovery UI coverage. Its original strict vendor direction is superseded. |
| [Topology investigation: scenarios](2026-10-19-hardware-topology-adr-gaps.md#scenarios-delivered-behavior-versus-proposal-or-unknown), [limits](2026-10-19-hardware-topology-adr-gaps.md#validation-and-limits) | Board/instance identity, setup proposals, electrical planning, per-instance firmware requests, module gates and mechanical survivor materialization. | All topology-transition combinations or delivered 0024–0028 policy. Read the subsequent-decision notices first. |
| [Coverage/candidates investigation: method](2026-10-20-adr-coverage-and-candidates.md#method-and-authority), [C1–C5](2026-10-20-adr-coverage-and-candidates.md#ranked-further-decision-candidates) | All then-current ADR text, selected keymap/catalogue/renderer/model/export paths. | C2's complete delete/recreate lifecycle, C3's executable upgrade behavior, end-to-end notices, or real GPU restoration. C1 is now settled by 0031. |
| [Input-boundary audit research](audit-input-boundaries-2026-10-18.md#scope-and-evidence-limits), [hardening map](../plans/rust-best-practices/map.md#implementation-outcomes-2026-10-19) | Six bounded counterexamples: WRL traversal/concavity, script emissions, receipts, generator escaping and PCB UUID import; subsequent hardening records narrower outcomes. | A fresh run here, general parser/geometry conformance or broad offline/multi-tab acceptance. Historical findings are not all still open. |
| [KiCad projection investigation](2026-10-19-kicad-board-projection-deepening.md#evidence-inventory-and-corrected-assumptions) | Preview versus exact extraction, original-source parsing, via spans, source IDs, transforms and module-import admission. Published follow-up is [182](../plans/typed-core-edits/issues/182-reuse-parsed-kicad-board-source.md). | An unexamined subsystem, a new common acceptance policy, or completion of the proposed parsing refactor. |
| [Editor extraction research](editor-workspace-extraction.md#2-current-structure-and-lifetime-map), [mounted coverage limits](editor-workspace-extraction.md#6-mounted-testing-what-is-available-and-what-is-missing), [current map](../plans/editor-workspace-extraction/map.md) | App/Editor/visible-child lifetimes, routing, drafts, camera/selection coordination and concrete composition-test gaps. | Runtime project-lifecycle or real GPU audit. The current map publishes implementation work; publication is not delivery. |
| [Matrix consistency map](../plans/matrix-layout-consistency/map.md#limitations-of-this-plan), [generator parameters](../plans/generator-parameters/map.md#progress), [module deepening](../plans/module-deepening/map.md#out-of-scope) | Scoped placement/parameter/settlement efforts, named outcomes and exclusions. | A general input-identity, catalogue upgrade or project lifecycle contract. Old checkpoint prose can lag later outcomes. |
| [CAD map](../plans/cad-backend-replacement/map.md#slices--current-status), [research index](../README.md#research-and-historical-evidence), [delivery queue](../plans/delivery-queue.md) | Rich CAD construction/qualification/performance history; finite current tickets. | Universal model/fit certification. The delivery queue is dated 2026-10-17 and is not the latest status authority for every effort. |

The actual [workspace manifest](../../Cargo.toml), top-level directories and Core module
inventory were compared with [architecture's responsibility map](../architecture.md).
Application, contracts, Core, footprints, renderer, CAD/provider, Host/Runtime/catalogue,
workspace UI, library data and build/deploy flows all enter the matrix below. Inventory and
responsibility headers are **orientation**, not evidence that the implementation underneath
was reviewed.

### Reading the classifications

- **Well traced for selected contract:** earlier/current evidence follows a named ownership
  or failure contract through connected source and tests. Not “fully audited.”
- **Partly traced:** a representation, layer or bounded scenario is examined, but important
  callers, lifecycle transitions, providers or outputs remain untraced.
- **Not traced in these investigations:** no sufficient connected implementation trace was
  found in the investigations examined. This does **not** mean nobody ever reviewed it.

## Application coverage matrix

Prior-research citations describe the scope of those records; D1–D5 describe this report's
additional source inspection. Existing test results are historical reports, never fresh runs.

| Subsystem / flow | Scope actually inspected earlier and now | Governing records | Coverage / remaining untraced work and consequence |
| --- | --- | --- | --- |
| Project creation, Open, switching, duplicate, delete | Prior G1/G2 follows Session publication; Editor research follows mount/hiding; current D1 samples variant creation/restore only. | [0013](../adr/0013-save-before-publication-and-explicit-recovery.md), [0014](../adr/0014-document-authority-and-captured-lifetimes.md); [Runtime lifecycle follow-up](../plans/module-deepening/map.md#out-of-scope) | **Partly traced.** Whole Library → Runtime → store deletion/restore/recovery UI and interrupted project transitions remain untraced as one workflow; risk is stranded work or wrong active project, not a missing new authority ADR. |
| Multi-tab saving, unreadable records, asset reclamation | [G8](2026-10-19-adr-gap-inventory.md#g8-multi-tab-saves-and-conservative-asset-reclamation) cites atomic content-token writes and named storage lifetime tests. No repeat audit here. | [0019](../adr/0019-reject-stale-saves-and-reclaim-assets-conservatively.md), 0013 | **Well traced for selected contract.** Real mixed-release tabs, storage eviction/quota and every recovery UI path remain partly/untraced; conservative cleanup is already policy. |
| Portable project/import boundary | [G9](2026-10-19-adr-gap-inventory.md#g9-portable-project-archives-are-a-core-trust-boundary) follows archive bounds/hash rejection; D4 follows optional bundled embedding and local-owner precedence. | [0004](../adr/0004-document-format-version-and-id-rename.md), [0020](../adr/0020-validate-portable-project-packages-in-core.md) | **Well traced for selected integrity contract; partly traced for portability.** Byte integrity does not establish semantic generator reproducibility or portable attribution. |
| Edit ordering, drafts, Undo and identity | Prior G1–G4, settlement/module maps and Editor research; D2 adds structural deletion/remapping and queued reuse guard. | [0005](../adr/0005-resolve-queued-edits-at-execution.md), [0006](../adr/0006-replace-document-for-import-and-recovery.md), 0013/0014, [0031](../adr/0031-preserve-pcb-handoff-protection-across-undo.md) | **Well traced for selected settlement contract; partly traced for identity lifecycle.** Not every deletion/resize path uses the same cleanup. Broad Undo audit is still listed in [backlog](../backlog.md). |
| Matrices, constraints, outline ownership | Matrix/typed-Core maps and [general inventory's existing coverage](2026-10-19-adr-gap-inventory.md#existing-adr-coverage-map); D2 inspects compaction/removal dependencies. | 0001/0002/0007/0008/0010/0011/0012 | **Well traced for selected contracts.** Not a fresh complete solver/outline audit; generic ID reuse and all output/lifecycle combinations remain partly traced. Do not reopen linked/fixed semantics. |
| Catalogue/custom definitions, assembly presets and upgrades | Earlier C3 only merge/legacy behavior; D1 follows snapshots, explicit preset application, generator dispatch/defaults and export. | 0003/0004/0009; [Parts catalog](../../DESIGN.md#parts-catalog), [assembly reference](../features/assembly-preview.md) | **Partly traced.** Data snapshots are not executable snapshots. Whole custom/import/module refresh lineage across releases remains untraced; changed pads/defaults can affect saved designs. |
| Key identity, position, keymap ordering | Earlier C2 inspected binding representation/edit tests; D2 follows removal routes, serialization/Open and firmware request/source ordering. | [PRODUCT](../../PRODUCT.md#capabilities-and-constraints), 0010/0024/0026; [ZMK reference](../design/zmk-keymap-reference.md#binding-and-layer-contract) | **Partly traced, materially deeper now.** Shrink/regrow, individual delete/recreate, push-suffix cleanup and shared-board instance meanings need a public workflow cross-product, not another map-by-ID observation. |
| Electrical planning and protected PCB handoff | [Topology enforcement trace](2026-10-19-hardware-topology-adr-gaps.md#enforcement-and-preservation-what-is-actually-checked), prior C1; D5 reads full package builder, not all planner branches. | 0024–0028, 0031 | **Well traced for selected planning/protection contract; partly traced for new topology policy.** Accepted deletion/conversion/per-half rules are not proven delivered. |
| Mounted and embedded hardware | Prior G12/topology; KiCad projection research traces module import, gating and source ownership. | [0023](../adr/0023-distinguish-mounted-daughterboards-from-embedded-circuits.md), 0010; [onboarding](../hardware/component-onboarding.md) | **Well traced for selected ownership contract; partly traced for qualification.** Whole catalogue/harness/driver and physical bench evidence is not established. |
| Firmware layers/macros/behaviors/peripherals | Earlier report samples keymap edits and firmware package; [dated ZMK investigation](../design/zmk-keymap-reference.md#validation-boundaries) explicitly researched upstream behavior; D2 follows key lowering. | [0030](../adr/0030-export-firmware-as-an-editable-repository-template.md), 0024/0026/0027; [feature limits](../features/keycaps-and-keymap.md#historical-implementation-verification) | **Partly traced**, not untouched. Complete target build/device behavior, every macro/layer/sensor combination and new optional-peripheral handoff policy remain untraced here. No live-device feature is inferred. |
| KiCad output, footprint source and board copper | Projection research already inspects parsers; D5 follows accepted Board → prepare/render/finalize → ZIP, plus imported-footprint source patching and native test bodies. | 0003/0009/0015/0016/0023/0031; [assembly reference](../features/assembly-preview.md) | **Partly traced.** Selected source preservation is evidenced; all KiCad dialects, every generated footprint and whole-board electrical correctness are not. |
| Gerber/drill and mechanical plate handoff | [Mechanical reference](../features/mechanical-assemblies.md#export-and-validation) records separate FR4 plate/KiCad path; current D5 reads mechanical package orchestration. | 0015/0016; same reference | **Partly traced.** Actual plot/drill correctness for all process/profile combinations and downstream routing/DRC remain outside this investigation. Direct Gerber generation is not an inferred feature. |
| Exact cases/PCBs, vendor previews, collision | Extensive CAD research/map; general G5–G7; D5 follows the imported physical PCB source seam. | 0015/0017/0018; [current successor graph](../plans/cad-backend-replacement/spec.md#successor-task-graph) | **Well traced for selected construction/ownership contracts; partly traced for provider completeness.** No universal repair certificate, whole-source clearance or physical-fit assurance. Published 015–018 are not delivery evidence. |
| Keycap geometry/printing and fit | CAD map 009, feature reference and Editor fit-navigation research; no new provider audit here. | 0015; [keycap reference](../features/keycaps-and-keymap.md) | **Partly traced.** Nominal profiles, unsupported sockets/printing tolerances and actual fit require separate evidence. Corrected mesh-only wording is not still an open ADR gap. |
| Renderer, picking, camera, context loss | Prior [C4](2026-10-20-adr-coverage-and-candidates.md#c4--renderer-authority-is-recoverable-automatic-context-restoration-is-not) follows GPU ownership, Host fail-stop and current-pick guards; input audit covers WRL; Editor research covers shared camera coordination. | 0014/0021; [cleanup 03](../plans/cleanup/issues/03-deduplicate-renderer-host-page-snapshot.md) | **Partly traced.** Real driver/context restoration, all Case/Parts pick paths and remount/camera experience are not traced end to end. Automatic recovery remains a research lead, not an accepted decision. |
| Assets, model bytes, licensing and provenance | Prior C5 maps IDs/digests; D4 follows staging, archive metadata and PCB packaging. | 0003/0017/0020/0023; [onboarding evidence](../hardware/component-onboarding.md#required-evidence) | **Partly traced, materially deeper now.** Source notices are richer than portable metadata; every vendor's distribution obligations and per-output propagation remain unresolved. No legal-compliance conclusion. |
| Browser startup/offline/preferences/workspace persistence | Prior G11 traces SW policy; Editor research traces guide/panel preferences and lifetimes; D3 follows package/cache/bootstrap boundaries. | 0014/0019/0022; [DESIGN](../../DESIGN.md#layout) | **Partly traced.** Real two-release transition, private/evicting storage, preference corruption and browser support behavior remain untraced as product journeys. |
| Accessibility, keyboard-only completion and narrow-screen workflows | PRODUCT/DESIGN requirements and scoped UI/Editor research, not a complete source-to-browser accessibility investigation. | [product principles](../../PRODUCT.md#product-principles), [design limitations](../../DESIGN.md#dos-and-donts), 0021 | **Not traced in these investigations as an end-to-end accessibility contract.** Individual controls/tests do not prove a designer can complete all workflows without a pointer or distinguish every failure state. Ordinary UX acceptance before another ADR. |
| Worker/provider compatibility, caches and releases | Prior G3/G11 cover lifetime/platform; input audit/hardening covers malformed receiver input; D3 reads Core/CAD wire and packaging/update paths. | 0014/0016/0022; [hardening 08](../plans/rust-best-practices/issues/08-replace-fragile-unwraps-and-unreachables.md) | **Partly traced.** Epoch/revision identity is not software-version compatibility. Mixed release/provider/cache behavior needs actual transition evidence. |
| CI, deploy, release/update support strategy | Current workflow files inspected; [cleanup map](../plans/cleanup/map.md#tickets) already owns deploy gating. | 0021/0022; [cleanup 06](../plans/cleanup/issues/06-gate-pages-deploy-on-checks.md) | **Partly traced.** No complete rollback/support-window/user-update strategy or hosting atomicity established. Existing gating work is implementation, not a new ADR discovery. |

## Ranked next decisions and exact coverage deltas

### D1: Project compatibility when generators change

**Candidate sentence:** “A saved project owns its component data, and a release must explicitly
define which generator/default/asset changes may change that project's resolved manufacturing
result without a reviewed upgrade.”

**Delta:** prefer an **ADR-0003 amendment**, referencing 0004/0009, plus a small compatibility
reference. Do not create a broad new catalogue ADR merely to repeat project override precedence.
A standalone variant/refresh ADR becomes justified only if a broader ownership choice emerges.

**Newly traced lifecycle:**

- [Catalogue `merge_project_overrides`](../../web/crates/catalogue/src/catalogue.rs#L167-L191)
  gives project data precedence by ID; the inspected
  `imported_and_project_overrides_keep_the_first_id_position` tests that bounded merge.
  [Matrix `snapshot_definition`](../../web/crates/catalogue/src/matrix_setup_operation.rs#L338-L355)
  clones the definition, changes its ID and extends generator parameters. It does not capture
  a generator executable. `prepare_matrix` stores operation-specific switch/companion definitions.
- [Assembly placement helpers](../../web/crates/parts/src/parts/assembly_presets.rs#L291-L301),
  `matrix_with_assembly`/`document_with_assembly`, separate recipe edits from existing placement
  snapshots. [Explicit preset application](../../web/crates/layout/src/objects/matrix_inspector_controller.rs#L145-L176),
  `matrix_preset_resolver`, resolves against the accepted matrix and refuses a changed size;
  `prepare_preset_matrix` refuses a different definition under an already occupied ID
  ([preparation](../../web/crates/layout/src/objects/matrix_inspector_controller.rs#L1974-L2015)).
  Thus even an explicit refresh is not evidence of a general upstream-version upgrade system.
- [Registry `render`](../../footprints/src/registry.rs#L157-L249) supports **one** version string,
  `bundled-1`, merges definition/part values and supplies current declared defaults for missing/null
  values. It runs the currently compiled body. [Core export `render_jobs`](../../core/src/artifact/kicad/generation.rs#L23-L59)
  invokes that registry from captured data; saving data does not pin historical code or defaults.
- [Normalizer](../../footprints/src/definition.rs#L82-L110) preserves saved pad IDs/nets **by index**
  and explicitly says pad count/order must not change without migration. This is recovered
  compatibility guidance, not a newly invented precaution. [Core normalization regression](../../core/src/generators.rs#L440-L457),
  `normalization_regenerates_pads_and_keeps_saved_ids`, checks the index-preservation behavior.
- `normalization_leaves_non_generator_definitions_alone_and_rejects_unknown_ones` in
  [framework tests](../../footprints/tests/framework.rs#L289-L308) rejects an unsupported version;
  [matrix setup tests](../../web/tests/matrix_setup_operation.rs#L155-L230),
  `matrix_setup_builds_the_reference_mx_rgb_cell_recipe_and_owned_definition_snapshots`, inspect
  owned data and construction flags. Their synthetic generator version/old-ID fixture is not
  an old-release/new-release rendering oracle.

**Recovered / inferred / unknown:** DESIGN already promises retained legacy/custom choices
and explicit preset updates; commit `12f721efb520cd0f0f5ae6af07f862ac6a3c6eb2` expressly preserves
legacy/custom parts and preset companions. `6014ce0d96152c7855ec891c086452e71b132000` records the
current Core-rendering route. Neither supplies a deliberate choice about bug fixes changing
old project output. Reproducibility risk from current bodies/defaults is an **inference from
the traced mechanism**, not proof that a specific release corrupted a saved board.

**ADR test / maturity:** reversal cost **high** (pads/nets/fit and previously handed-off work),
surprise **high**, trade-off **real but unsettled** for executable changes. Data ownership is
high-confidence; complete upgrade behavior medium; historical upgrade rationale unknown.
Documented alternatives include authoritative retained KiCad source versus executable generation
([D5](#d5-manufacturing-source-authority-is-output-specific)); no historical decision to ship
multiple old generator bodies or bake all footprints was recovered.

**Targeted scenario:** a bug fix moves a switch pad by 0.3 mm without changing parameter types
or pad order. Must an old project's next export preserve its prior geometry until explicit
upgrade, or use the corrected geometry with an explicit compatibility notice/revision review?
This is a real choice; “what does the current code do?” is already answered above. Evidence
still needed: two source-bound releases exporting the same saved generated definition,
including omitted defaults, model datum, terminal mapping and protected handoff consequences.

### D2: Input identity across layout evolution

**Candidate sentence:** “Distinguish a surviving input moved by structural edits from a
replacement input reusing its coordinate, and define binding retirement/reactivation consistently
across delete, disable, resize and recreate.”

**Delta:** **conditional new domain ADR**, if the owner chooses a durable lifecycle guarantee;
ordinary architecture/test documentation first. Do not reopen 0024's per-half assembly mapping
or invent a persisted hardware-revision object. Product already disclaims automatic semantic
keymap migration; that disclaimer does not describe every current cleanup path.

**Newly traced lifecycle:**

- [`member_id`, `removed_members`, `set_matrix`](../../core/src/matrix.rs) derive key IDs as
  `matrix/{matrix}/r{row}c{column}`. A surviving key moved without changing its coordinate keeps
  that ID. Shrinking drops out-of-bounds cells/parts and net/constraint membership; regrowth
  computes the same coordinate IDs. The `set_matrix` cleanup shown here does not itself clear
  typed keymap/legacy binding maps.
- [`matrix::deletion::remove_axis` → `remap_parts`](../../core/src/matrix/deletion.rs#L119-L262)
  simultaneously renames survivors and remaps typed bindings, sensors, keycaps, net pins,
  constraints, board/layout membership and electrical key bindings/assignments. It intentionally
  leaves protected handoff baselines immutable. The comment explicitly prevents a removed slot
  donating metadata to its next survivor; this is not “bindings follow the coordinate” universally.
- `remove_cells` records a durable `MatrixCell.deleted` flag and uses dependency cleanup;
  `SetMatrixCellsEnabled` instead invokes `set_matrix`. [Core dispatch](../../core/src/lib.rs#L1805-L1885)
  shows generic `RemoveParts` and whole `RemoveMatrix` also have their own cleanup route.
  The structural deleted-cell marker is **not** a promised deleted-input keymap tombstone or
  automatic semantic migration system. Do not “fix” PRODUCT's disclaimer merely because a
  test uses the word tombstone.
- [`keymap::validate`/`apply_edit`/`source`](../../core/src/keymap.rs) validate binding values and
  current edit targets, but source emission walks the **current supplied key order** and looks
  up bindings by ID, with legacy base-layer and transparent higher-layer fallbacks. Validation
  does not prove every stored binding key is a currently present input. [Firmware adapter](../../web/crates/runtime/src/firmware_request_adapter.rs#L90-L171)
  builds keys from electrical assignments and appends auxiliary pushes; `keys_for_bindings`
  reads board-scoped legacy values. [`firmware::keymap`](../../core/src/firmware.rs#L773-L800)
  combines halves and lowers the typed map using that order.
- This yields a **static, bounded concern**, not a reproduced new defect: retained map entries
  and coordinate reallocation can reconnect after shrink/regrow or generic removal, while
  structural deletion deliberately prunes/remaps. No compensating universal binding cleanup
  was established in this trace. Exact behavior after save/reopen, encoder `/push` identities,
  native mirror exceptions and every UI producer still needs workflow verification.

**Named test bodies inspected:** [Core matrix tests](../../core/tests/core.rs#L3713-L4066),
`deleting_matrix_row_preserves_surviving_part_and_binding_identity`,
`deleting_matrix_row_remaps_nets_constraints_keycaps_and_direct_wiring`,
`deleting_last_matrix_row_removes_matrix_and_key_metadata_and_is_undoable`, and
`deleting_matrix_cells_propagates_tombstones_and_survives_reopen`, establish selected
compaction/cleanup/serialization contracts. `matrix_shrink_discards_out_of_bounds_edits`
([same suite](../../core/tests/core.rs#L2649-L2699)) checks geometry, not binding reactivation.
[`queued_key_resize_retires_when_row_delete_reuses_the_selected_id`](../../web/crates/layout/src/objects/keycap_size_tests.rs#L1073-L1139)
uses real Runtime/Core with a held deletion and rejects a queued edit targeting reused
coordinate identity. This is valuable evidence of the identity hazard and a bounded guard,
not proof that all controls/bindings implement it.

**Recovered / inferred / unknown:** `39c9bbac2c05d138415aa49df8389dd292374e51` introduces the
bulk/deletion work, and [matrix-consistency notes](../plans/matrix-layout-consistency/map.md#notes)
explicitly preserve it. PRODUCT/DESIGN disclaim migration. Source comments recover the
survivor-remapping rationale. No earlier choice between universal identity persistence and
reset-on-layout-revision was recovered; the different existing routes are not evidence that
their divergence was intentionally selected.

**ADR test / maturity:** cost **high**, surprise **high**, actual lifecycle trade-off **not
settled**. Confidence high on the inspected branches, medium on complete lifecycle. Existing
documented behaviors—not invented historical alternatives—are survivor remapping, explicit
deletion, disable/re-enable and product's no-migration promise.

**Targeted scenario:** bind the last row to A, shrink it away, save/reopen, then grow the row
back without Undo. Should that input return bound to A, be unassigned, or require explicit
review? Contrast deleting an interior row: the surviving next-row key already carries its own
binding during compaction. Before implementation, add public workflow evidence for both
typed and legacy bindings; do not ask the user to rediscover these code facts.

### D3: Release coherence is not request freshness

**Candidate sentence:** “Treat page, workers and provider assets as a coherent release, and
state what happens when an old active editor encounters a newer or unavailable lazy asset.”

**Delta:** likely **ADR-0022 amendment plus release/runbook documentation**, not another
local-first/platform ADR. Existing deploy gating is [cleanup 06](../plans/cleanup/issues/06-gate-pages-deploy-on-checks.md),
not a new implementation ticket to manufacture. Whether compatibility needs a handshake,
immutable URLs or another mechanism is **not decided here**.

**Newly traced boundary:** [build `copy_runtime_assets`/`embed_offline_worker`](../../scripts/build-web.py)
packages Core/CAD workers, renderer, CAD module and models, then emits a release-specific
cache name and route inventory. Worker entry URLs are stable paths such as
`assets/core-worker/entry.js`; the inventory version is not a negotiated worker API version.
[SW `install_policy`/`activate_policy`/`cached_fetch`](../../web/src/service_worker.rs) precaches,
waits for existing clients, removes older same-scope caches on activation, serves current
cached assets/document fallback and otherwise uses network fetch. Waiting is already
accepted under 0022 and explained by `8d481b7a8743ed7c8b62f47253e6d00b3096bca6`.

[Core bootstrap](../../web/src/core_worker.rs#L29-L42) reports a fixed ready string;
[Core client](../../web/crates/host/src/host/core_client.rs#L379-L464) checks request ID,
executor epoch, kind and decoded reply identity. [CAD `WireRequest`/`WireReply`](../../web/crates/host/src/cad_worker.rs#L47-L81)
carry job/operation/snapshot identity, not a release digest. Its ready path and strict
receiver/payload validation protect current request contracts. These are not proof that
two software releases can interoperate merely because the document format is unchanged.
The observed lack of a build handshake is a fact about these interfaces, **not evidence
that a mixed release has actually been served or that adding a handshake is necessary**.

**Tests inspected:** [build tests](../../scripts/test-build-web.py),
`test_offline_worker_waits_for_old_clients_before_activation`, is a source-string check;
`test_build_assembles_requested_routes_from_dirty_worktree` uses fake build/embed functions;
`test_offline_worker_bootstrap_passes_route_manifest` checks generated packaging. None runs
an old editor through a real deployment. [Core framing test](../../web/crates/host/src/core_protocol.rs),
`text_frame_preserves_full_u64_revision_without_a_js_number_round_trip`, protects numeric
identity, not release matching. [CAD receiver regression](../../web/crates/host/src/cad_worker.rs#L3317-L3473),
`raw_worker_preview_without_prepared_or_keycap_input_fails_cleanly_and_worker_recovers`,
uses a controlled provider and postMessage target; it is not a two-release rollout test.

**Operational evidence at investigation time:** the former `pages.yaml` workflow built/deployed
on its own trigger and deploy depended on its build only. [Validation and release](../../.github/workflows/check.yaml)
now share a workflow under Cleanup 06; its first `main` run remains unverified. Repository config alone cannot reveal actual hosted branch
protection, cache headers or deployment atomicity; these remain unknown here.

**ADR test / maturity:** cost **meaningful** (offline installed clients, persisted upgrades),
surprise **yes**, trade-off **recovered only for delayed activation**; no recovered choice
among mixed-version compatibility strategies. High-confidence packaging/wire observations,
low-confidence need for a new mechanism. First inspect real A→B rollout, then decide.

**Targeted scenario/question:** with release A open, install B; request CAD for the first
time both online and offline, then close A and reopen. If the old lazy provider cannot load,
is the promised experience “keep editing and explain unavailable generation until explicit
reopen,” or must generation keep working? Test installed/uncontrolled clients and separate
same-origin tabs before claiming either guarantee. Do not weaken 0013/0019 safety to update.

### D4: Portable provenance and distribution boundaries

**Candidate sentence:** “Source/variant/licence evidence has an explicit propagation boundary
when library assets enter a project and leave it in a portable or downstream handoff.”

**Delta:** **conditional new ADR only for a genuine distribution/ownership choice**. First
an onboarding/output documentation and metadata-requirement investigation; amendment of
0020 is sufficient if the task merely enforces an already required portable record. Integrity
and licensing are different contracts; no blanket licence or legal conclusion is proposed.

**Newly traced path:**

- [Onboarding](../hardware/component-onboarding.md#onboarding-workflow) requires exact variant,
  revision, licence, units/hashes and portable source evidence. [Footprints NOTICE](../../footprints/NOTICE.md)
  preserves upstream revisions and mixed generator licences; [library boundary](../../ergogen/library/BOARDSTUDIO.md)
  says the retained assets/manifests own model provenance. This is richer than a SHA hash.
- [Stager `catalogue`/`stage_models`](../../scripts/stage-ergogen-models.py) maps vendor paths to
  safe static filenames and copies/symlinks model bytes. [Runtime `BundledModel`](../../web/crates/runtime/src/bundled_models.rs#L1-L21)
  carries identity, filename, media type, source-relative path, URL and digest, but no licence
  or upstream revision field. Its digest test checks source bytes, not redistribution terms.
- [`pack_document`](../../web/crates/runtime/src/runtime.rs#L7227-L7273) gathers generated and
  directly saved references and local assets. [`portable_archive::prepare_archive`](../../web/crates/runtime/src/portable_archive.rs#L47-L96)
  embeds bundled model bytes into a cloned document with `license: None` and the generic source
  `bundled footprint library`. Existing document asset records retain their own metadata/bytes;
  [Asset](../../core/src/model.rs#L638-L649) can store optional licence/source. The inspected
  `pack_round_trip_keeps_local_bytes_and_optionally_adds_opaque_model_bytes` confirms this
  bounded propagation and preserves a document-owned model over the current bundled copy.
- [PCB `load_model_files`/`build_handoff`](../../web/crates/runtime/src/pcb_handoff.rs#L65-L275)
  verifies model hashes and writes `models/<hash>.<extension>`, board ZIP, wiring report,
  assembly instructions and jumper SVGs. This complete builder does not add a separate
  attribution/licence inventory. Some retained source/model files may themselves contain
  notices; the trace does not establish that all do, or that an extra file is legally required.
- [Routed-board import](../../web/crates/pcb/src/pcb_board_reference.rs#L278-L325) and model
  attachments initialize optional licence/source as absent. That records lack of metadata,
  not permission to redistribute or a conclusion that the user's file is unlicensed.

**Recovered / inferred / unknown:** `9582132875d3e8fa9958a19bd3da53b4c61241ff` supplies portable
lookup/build-provenance history, not a legal-policy choice. Source/byte preservation is
documented. Failure to reconstruct rich provenance from generic metadata is a supported
engineering inference. Per-vendor obligations, licence application to generated artifacts,
permitted use, and any complete rejected distribution alternatives remain **unknown**.
The vendored README's explanatory legal commentary is not adopted as BoardStudio legal advice.

**ADR test / maturity:** reversal cost **potentially high** once third parties receive files,
surprise **yes** (self-contained bytes are not self-contained provenance), chosen trade-off
**not recovered**. High confidence in the metadata/output trace, low confidence in the exact
policy needed. App notices and embedded/project metadata are documented existing locations;
no earlier choice making either sufficient for every output was found.

**Targeted scenario/question:** a collaborator receives only the PCB handoff ZIP containing
two vendors' models and a user-modified footprint. Which upstream identity, notices and
authored-change attribution must remain discoverable without this checkout? First inventory
actual vendor manifests/notices and output-contained headers for a representative mixed
package, and obtain qualified licence interpretation where needed. Do not force the user
to answer facts that the manifests can establish.

### D5: Manufacturing source authority is output-specific

**Candidate sentence:** “A retained routed-board reference is read-only electrical data but
can supply a physical unpopulated PCB operand; generated electrical handoff, case construction
and visual projection retain distinct source owners.”

**Delta:** **ordinary architecture/feature-reference source matrix first**, with an optional
0015 clarification if confirmed as an enduring constraint. No standalone “KiCad import ADR”
or bidirectional round-trip feature is justified. Much is already explicit in the assembly
reference and 0015/0023; the new work is tracing their less obvious combined consequences.

**Newly traced flow:** [`import_routed_board`](../../web/crates/pcb/src/pcb_board_reference.rs)
stores original hashed bytes and a `BoardReference`, not imported host nets/copper.
[`LayoutSourceCapture::capture`/`artifact_request`](../../web/crates/runtime/src/layout_viewer_source.rs#L306-L400)
select the enabled accepted asset for visual projection. [`captured_pcb_source`](../../web/crates/host/src/cad_jobs.rs#L1162-L1248)
captures the same raw reference before physical view projection and composes instance
reflection once. [`prepare_physical_step_assembly`](../../web/crates/runtime/src/runtime.rs#L5267-L5424)
then reads source curves/drills, rejects mismatched declared thickness or a modified nominal
reference operand, prepares it through Core, and substitutes **the PCB reference body**.
It does not rewrite the accepted Board's electrical data or regenerate case settings from
the source's copper. [`mechanical_package_bytes`](../../web/crates/runtime/src/runtime.rs#L5427-L5751)
uses that preparation and labels source-backed versus nominal unpopulated PCB with source
ID/hash/thickness in `assembly.json`; component solids are not claimed included.

Conversely, [`build_handoff`](../../web/crates/runtime/src/pcb_handoff.rs#L83-L160) calls
`ArtifactRequest::ExportPcb` with the accepted document/Board contours. [Planning](../../core/src/artifact/kicad/planning.rs)
selects generated/imported/authored **footprint** handling; [finalization](../../core/src/artifact/kicad/output.rs#L537-L877)
writes accepted Board traces/vias and generated objects. An imported source-backed footprint
is a different ownership mode from an imported routed Board. Its original spans are patched
for placement, nets and identity via [`patch_footprint`](../../core/src/artifact/source.rs#L123-L147),
not rebuilt from an incomplete preview. [`compile_authored`](../../core/src/artifact/compile.rs#L4-L20)
explicitly refuses both authoritative KiCad source and generator-backed fallback.

**Existing evidence:** [assembly reference](../features/assembly-preview.md#preview-limits)
already rejects electrical replacement and explains approximation limits;
[mechanical reference](../features/mechanical-assemblies.md#profiles-and-source-geometry)
separates retained electrical source from assigned plate geometry. The earlier
[KiCad investigation](2026-10-19-kicad-board-projection-deepening.md#duplicated-rules-versus-legitimate-differences)
already establishes distinct preview/exact/import admission and is not newly discovered here.
`pcb_source_capture_keeps_original_reference_and_composes_physical_reflection_once` in
[Host tests](../../web/crates/host/src/cad_jobs.rs#L1676-L1783) checks capture/nonmutation and
ambiguous/missing/stale input. [KiCad integration test body](../../core/tests/kicad_integration.rs#L774-L831),
`native_board_export_preserves_untouched_source_spans_and_kicad_10_parses_and_plots_both_sides`,
asserts retained rich source, transforms, UUID references and actual plotting when run.
It is not executed here. `6014ce0d96152c7855ec891c086452e71b132000` records export routing;
complete original selection rationale for the source matrix was not recovered.

**ADR test / maturity:** cost **high** if ownership is reversed, surprise **yes**, actual
recovered trade-off **partial and already substantially covered**. High confidence in the
source matrix, medium in user-facing disclosure/readiness. Do not promote implementation
into an intentionally chosen policy just because it is coherent.

**Targeted scenario/question:** the routed reference has the same thickness but a larger
outline than the editable Board. The assembled STEP/collision PCB operand and a fresh
electrical handoff use different source authorities today; does the user need an explicit mismatch
review before the affected case handoff, or is clearly labelled independent-source review
the intended boundary? No universal geometric-congruence gate or automatic electrical
reconciliation is selected here. Inspect the actual UI disclosure and representative
combined package before proposing a behavior change. Downstream DRC/plot/fit evidence is
still separate; [mechanical export scope](../features/mechanical-assemblies.md#export-and-validation)
leaves Gerbers/drills to KiCad and does not promise direct fabrication serialization.

## Already covered, excluded and implementation debt

- **0031 is settled**, including package creation → current owner → saved protection before
  download, protection surviving Undo and later delivery failure. Do not repeat prior C1's
  success-boundary interview ([0031](../adr/0031-preserve-pcb-handoff-protection-across-undo.md)).
- **0024–0028 are intended policy**, not invitations to match older code: shared routing with
  real per-half assembly mapping; exclusive removed-design/accessory deletion; either side
  central; incomplete designs saveable; explicit conversion with prior distinct-half deletion
  and an unrouted mirrored second design on separation. Their own implementation notices and
  [prior reconciliation](2026-10-20-adr-coverage-and-candidates.md#e6--known-implementation-debt-must-not-be-misreported-as-contradictory-intent)
  distinguish delivery debt. This report does not re-audit or ticket their full implementation.
- **0017/0018 admit useful partial/repaired previews and approximate observations**, with
  honest errors/uncertainty and preserved originals. No universal completeness/certification
  gate is reintroduced. At inspection, the [CAD map](../plans/cad-backend-replacement/map.md#slices--current-status)
  and ticket headers had 015–018 `ready-for-agent`, dependencies 015→016→{017,018}, while
  parent 007 remained `claimed`. No duplicate successor ticket or claim was created.
- **0029 remains deprecated/out of product scope.** Retained script compatibility/removal
  needs separate bounded work; no scripting feature, plugin or placement-ownership decision
  is inferred ([scope correction](../adr/0029-constrain-geometry-scripts-and-apply-results-atomically.md#scope-correction-2026-10-20)).
- **0030 is source-template delivery with external compilation**, including disclosed manual
  work for unsupported optional peripherals when the base keyboard is valid. Do not classify
  every unsupported peripheral as a reason to block the whole handoff or invent pins/drivers
  ([amendment](../adr/0030-export-firmware-as-an-editable-repository-template.md#amendment-unfinished-optional-peripheral-support-2026-10-20)).
- **E1–E4 wording was reconciled.** Current keycap reference says mesh/STL, 0004 distinguishes
  format6 chronology, DESIGN separates accepted manufacturing capture from disposable preview
  currency, and architecture/0006 mark retained scripting out of scope. The prior report's
  [notice](2026-10-20-adr-coverage-and-candidates.md#executive-summary) and current texts were
  checked; those original wording findings are not repeated as open defects. E5 amendment
  reading guides remain optional readability work, not a new decision.
- Exact crate edges, CSS tokens, compiler flags, every resource cap and dependency pin are
  not automatically architectural choices. Preserve guards and documented domain ownership;
  prefer ordinary source/reference maintenance. No new support matrix, server requirement,
  live firmware editing, automatic keymap migration or general CAD certification is inferred.

## Small next batch, not an ADR-writing campaign

1. **Generator compatibility conversation (D1).** Use the concrete changed-pad/default
   scenario and current single-version renderer. Decide the compatibility promise before
   specifying an upgrade mechanism; likely amend 0003, not write a catalogue manifesto.
2. **Input lifecycle conversation (D2).** Contrast interior-row survivor remapping with
   shrink/save/reopen/regrow. Decide the user's expected binding outcome and distinguish
   same input from replacement. This is the strongest conditional new domain ADR.
3. **Optional distribution conversation (D4), after manifest/output inventory.** Agree what
   a collaborator must receive; do not ask the user to adjudicate unknown licence facts.

Before another interview, an agent can obtain facts for **D3's real release transition** and
**D5's combined source disclosure/mismatch**. Renderer restoration is lower-maturity: the
previous investigation already traced fail-stop/cleanup and candidate-hit authority, but no
real-driver recovery contract is established. Extend that evidence first rather than asking
“should we use a scene graph?” or writing another general authority ADR.

The largest remaining blind spots are consequently **whole project recovery/deletion journeys,
actual old/new offline release behavior, complete layout-binding evolution, hardware/firmware
device qualification, mixed-source manufacturing review, portable provenance, and complete
keyboard/accessibility workflows**. They are not equally unexplored: the matrix identifies
which portions already have strong bounded evidence. More documentation without those
traces would create false assurance, not sustainability.

## Verification, provenance and limits

- CodeGraph was used first for code/named files. Returned source was treated as read;
  raw reads filled omitted test/source ranges and searches located uncovered details.
  Some exact test names were not served by the index, so their current bodies were read
  directly. No source copies, hash manifests, generated fixtures or additional artifacts
  were created.
- Named test bodies above were **inspected, never run**. They include real Core matrix
  history/serialization, real Runtime with controlled gates, generator framework/normalizer,
  portable archive round-trip, source-patching/KiCad integration, controlled CAD receiver
  and fake-build packaging checks. Each establishes only its described seam. Reading a
  test containing a KiCad subprocess does not establish an executed KiCad check.
- Stable Git objects inspected for bounded history: HEAD above;
  `39c9bbac2c05d138415aa49df8389dd292374e51` (bulk key/deletion work),
  `12f721efb520cd0f0f5ae6af07f862ac6a3c6eb2` (curation/legacy preservation),
  `6014ce0d96152c7855ec891c086452e71b132000` (Core generator/export route),
  `9582132875d3e8fa9958a19bd3da53b4c61241ff` (portable lookup/provenance), and
  `8d481b7a8743ed7c8b62f47253e6d00b3096bca6` (delayed activation).
  Commit messages establish stated implementation intent, not independent user approval.
  Retrieve with `git show <object>`; current untracked ADR acceptance is not attributed to them.
- Checked 136 local links/anchors, 18 named test declarations and six cited commit objects:
  all resolved; report whitespace was clean and HEAD remained the examined revision.
  This is document integrity, not application verification. Concurrent edits can
  invalidate line numbers after inspection; symbols and decision headings identify intent.
- **Unresolved tensions:** PRODUCT's stable-ID/no-migration language is not a full description
  of coordinate-ID remapping/reactivation (D2); snapshot/legacy preservation is not executable
  reproducibility (D1); an electrically read-only routed reference still participates in a
  physical manufacturing reference operand (D5). These require sharper contract/disclosure,
  not silent reversal of accepted policy. Provenance propagation and mixed-release behavior
  remain evidence gaps, not proven security/legal/update failures.
- No builds, tests, browser automation, firmware compilation, mutation checks, physical QA,
  external documentation fetches, commits, subagents, ADR/glossary/index/plan/ticket or
  production edits. No full security, legal, geometry, accessibility or release audit is claimed.
