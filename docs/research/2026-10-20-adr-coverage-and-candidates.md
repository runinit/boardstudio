# ADR coverage, integrity and further decision candidates

Research date: 2026-10-20. Examined HEAD: `9099b3ef7a72b0919b1572f116490ed57514d43a`.
**Research only; no decisions are accepted by this report.** The checkout contains extensive
concurrent changes. In particular, ADRs 0013–0030 and the glossary are untracked at the
examined HEAD; their accepted/deprecated text is current product authority, not evidence
that those decisions are present in that commit. Source links describe inspected working-tree
contents, not an atomic snapshot. Only this report was intentionally written.

## Executive summary

**Subsequent documentation reconciliation (2026-10-20):** E1–E4's identified
reference conflicts were corrected in the keycap feature reference, ADR-0004,
DESIGN, architecture and ADR-0006. The edits align wording with existing decisions;
they do not establish implementation acceptance or remove retained scripting code.
The original findings below remain the record of the inspected pre-correction text.

The major durability, CAD, topology and firmware-delivery gaps found yesterday now have
records. **Do not generate another batch of those ADRs.** All 30 current records were read:
29 are accepted and 0029 is deprecated. Their overlaps mostly represent useful layers,
not duplicates to delete ([coverage map](#current-coverage-map)).

The highest-value existing-document work is:

1. Reconcile the current keycap reference's Cadrum/exact-STEP promises with 0015's
   mesh-only keycap scope; the docs index still presents that reference as current.
2. Clarify 0004's format-6 history: framed constraints required the version transition;
   later optional reservation counters did not require another bump.
3. Stop older Case export wording from making disposable preview currency an export
   ownership requirement; distinguish a legitimate prepared-input gate from preview state.
4. Keep scripting's removal/compatibility question separate from a product feature;
   0029 explicitly rejects that feature premise.

These are documentation findings, not evidence of four newly discovered code defects.
Known disagreements with 0017/0018 and 0024–0028/0030 are **policy implementation debt**,
not reasons to reverse the user's decisions. CAD successor tickets 015–018 already own
the published forgiving-preview work; their publication is not implementation acceptance.

**Five further areas were assessed:** one historically grounded new-ADR candidate
(PCB handoff protection), two candidates needing a concrete ownership decision
(key identity/layout evolution and catalogue refresh), and two research leads
(renderer recovery and portable provenance). **None is ready to be accepted without
confirmation or further bounded research.** Renderer authority itself is better an
0014 amendment; ordinary coordinate conventions belong in architecture, not a new ADR.

## Method and authority

Read [root instructions](../../AGENTS.md), [domain instructions](../agents/domain.md),
[glossary](../../GLOSSARY.md), [ADR index](../adr/README.md), every current ADR,
[PRODUCT](../../PRODUCT.md), [DESIGN](../../DESIGN.md), [architecture](../architecture.md),
and [documentation index](../README.md). Read both earlier inventories as orientation:
[general](2026-10-19-adr-gap-inventory.md) and
[hardware topology](2026-10-19-hardware-topology-adr-gaps.md); their subsequent-decision
notices take precedence over their original recommendations.

The installed domain-modeling skill and `ADR-FORMAT.md` require all three:
**meaningful reversal cost, surprising behavior without context, and a genuine
alternative/trade-off**. A source file proves behavior, not that its author chose a
product policy. A current accepted record can establish today's choice without
proving an older historical rationale. Proposed alternatives below are explicitly
questions, not invented previously rejected options.

CodeGraph was used before source exploration. Sampled connected production paths,
test bodies and Git objects rather than attempting full implementation acceptance
of 30 records. Classifications used below:

- **Doc conflict:** current text gives incompatible guidance.
- **Implementation debt:** code still differs from an explicitly newly accepted policy.
- **Historical supersession:** old text is retained but later dated authority governs.
- **Unknown:** a universal claim or complete workflow was not established.
- **Actual code contradiction:** requires a traced behavioral disagreement; it is not
  automatically an unrecognized bug when the ADR already identifies the debt.

## Current coverage map

All numbers and statuses are from the [current index](../adr/README.md). Grouping is
for navigation; preserve record identities and amendment histories.

| Records | Covered decision and overlap disposition |
| --- | --- |
| [0001](../adr/0001-fixed-edited-outlines.md), [0002](../adr/0002-linked-outline-refinements.md), [0012](../adr/0012-bounded-outline-validation.md) | Fixed versus linked outline ownership; edit/save versus dependent-output validity; exact grid contacts and per-path budget. Complementary, not three competing outline rules. Keep 0012's named budget in its existing record; do not create more limit ADRs. |
| [0003](../adr/0003-rust-footprint-generators-in-core.md), [0009](../adr/0009-generator-parameters-checked-in-core.md) | Rust generator execution/manual upstream ports/licences versus parameter admission and legacy warning asymmetry. Refresh ownership remains narrower uncovered territory, not another generator-language decision. |
| [0004](../adr/0004-document-format-version-and-id-rename.md), [0011](../adr/0011-part-offset-is-its-own-constraint-kind.md) | Versioned load migrations and older-reader refusal versus a new tagged constraint kind rather than a silently ignored property. 0011's version-5 discussion is historical context, not today's version number. Amend 0004's history, not 0011's original rationale. |
| [0005](../adr/0005-resolve-queued-edits-at-execution.md), [0006](../adr/0006-replace-document-for-import-and-recovery.md), [0007](../adr/0007-matrix-gestures-edit-named-parameters.md) | Execution-time intents, settlement and strict captured routes; typed-edit direction with named replacement exceptions; matrix semantics/Core axes. 0006 explicitly is not an already-enforced universal ban. |
| [0008](../adr/0008-linked-halves-stay-linked.md), [0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) | Default linked domain behavior versus hybrid stored/reconciled representation, exceptions, identity, headings and creation ownership. Keep both; 0010 needs a current-rule reading guide, not renumbering or wholesale splitting. |
| [0013](../adr/0013-save-before-publication-and-explicit-recovery.md), [0014](../adr/0014-document-authority-and-captured-lifetimes.md), [0016](../adr/0016-export-ownership-is-separate-from-preview-currency.md) | Durable publication/recovery, single authority/operation-specific lifetimes, export active-work versus delivery ownership. These narrow rules explain different failures; consolidation would lose useful boundaries. Renderer ownership can amend 0014. |
| [0015](../adr/0015-exact-generated-cad-and-mesh-preview-scope.md), [0017](../adr/0017-forgiving-vendor-previews-with-preserved-source.md), [0018](../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md) | Exact generated case/PCB output versus mesh keycaps; useful repaired/partial vendor display with preserved originals; approximate observations versus supported clearance conclusions. Neither completeness nor universal proof is a display/check admission rule. |
| [0019](../adr/0019-reject-stale-saves-and-reclaim-assets-conservatively.md), [0020](../adr/0020-validate-portable-project-packages-in-core.md) | Multi-tab stale-write refusal/Undo-aware reclamation versus portable package integrity. Neither establishes a licence/provenance propagation guarantee. |
| [0021](../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md), [0022](../adr/0022-static-local-first-rust-application-with-worker-execution.md) | Real native Session/Core tests plus mounted checks; static local-first Rust/worker/offline platform. No new ADR for exact crate edges or compiler flags. |
| [0023](../adr/0023-distinguish-mounted-daughterboards-from-embedded-circuits.md) | Source-owned mounted circuitry versus independent embedded host copies and explicit joins. Catalogue refresh is different from circuit-copy identity. |
| [0024](../adr/0024-share-reversible-pcb-routing-not-half-firmware-configuration.md), [0025](../adr/0025-delete-a-removed-halfs-exclusive-pcb-design.md), [0026](../adr/0026-decouple-firmware-role-from-physical-side.md), [0027](../adr/0027-save-incomplete-split-designs-with-output-specific-readiness.md), [0028](../adr/0028-convert-reversibility-explicitly-without-merging-designs.md) | Shared routing/assembly-specific mapping; exclusive-design/accessory deletion; independent side/role; incomplete-design saving; explicit reversibility conversion. These settle distinct user scenarios. Do not replace with a vague “boards versus halves” ADR. |
| [0029](../adr/0029-constrain-geometry-scripts-and-apply-results-atomically.md) — **deprecated** | User-authored geometry scripting is outside product scope. Retained code safety does not reauthorize the feature. Preserve the correction and number. |
| [0030](../adr/0030-export-firmware-as-an-editable-repository-template.md) | Editable repository-template handoff, external compilation and disclosed unsupported optional peripherals. Not binary generation, automatic repository provisioning, or proof of delivered optional-peripheral support. |

The index already records the historical reuse of number 0003. Original application
ownership is preserved in Git, with living rules reconstructed in 0013/0014/0022;
do not restore the obsolete browser-trial/platform/package requirements wholesale.
The deletion commit describes retirement of React/migration machinery, not reversal
of every ownership rule (H1/H2 below).

## Ranked existing-record and connected-document findings

### E1 — Current keycap reference promises an output deliberately removed

**Classification: doc conflict, high confidence.**
[Keycaps and keymap](../features/keycaps-and-keymap.md) says the current 3D path uses
Cadrum/OCCT and offers keycap STEP with separate inlays. The
[docs index](../README.md#current-references) links it as a current reference.
[0015](../adr/0015-exact-generated-cad-and-mesh-preview-scope.md) and the
[CAD scope decisions](../plans/cad-backend-replacement/map.md#scope-decisions-2026-10-17-user)
instead explicitly limit exact generated output to cases/PCBs and make keycaps
display/optional-print meshes. The CAD map's resolved 009 records mesh/STL delivery;
this is not evidence from a fresh run here. H3 pins the scope-change provenance.

**Disposition:** reconcile the feature reference and mark the old implementation/output
paragraph as historical or replace its current guidance. Preserve the historical test
receipts with dates. No new ADR or user choice is needed; do not reinstate keycap STEP
merely to match an obsolete feature page. Complete keycap provider correctness was not audited.

### E2 — Format-6 explanation obscures which change needed the bump

**Classification: misleading/incomplete historical wording, high confidence.**
[0004](../adr/0004-document-format-version-and-id-rename.md#follow-up) says “Format
version 6 adds” optional mechanical reservation counters and then “No migration step
or version bump was needed.” It does not explain the framed-constraint transition.
[0010's framed-constraint amendment](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md#amendment-selective-exceptions-and-framed-constraints-2026-10-11)
does. [Migration source](../../core/src/migrate.rs), `CURRENT_VERSION`, `migrate_value`
and `version_5_migration_only_stamps_the_format_version`, establishes current version
6 and an ordered 5→6 step. `version_4_document_migrates_without_changing_its_payload`
explicitly identifies framed constraints as the version-6 addition. H4 shows the
counter paragraph was a later addition, not the original framed-format migration.

**Disposition:** amend 0004 to distinguish the existing framed-constraint bump from
subsequent compatible counters within format 6; cross-link 0010. Do not infer that
current migration code is wrong or create a counter-specific ADR. The counter
trade-off itself is already recorded; this is chronology repair.

### E3 — Case readiness wording can recouple export to disposable previews

**Classification: doc conflict/overbroad requirement; implementation equivalence unknown.**
[DESIGN's Layout section](../../DESIGN.md#layout) says Case export requires current
generation, preview, committed scene and assembly revisions. That unqualified
“preview” requirement conflicts with [0016](../adr/0016-export-ownership-is-separate-from-preview-currency.md)
when read as a disposable-view gate. [Architecture](../architecture.md) correctly
separates preview currency from export ownership. [Core capabilities](../../core/src/lib.rs),
`authored_case_geometry_ready` and `case_preparation_ready`, also distinguish authored
Case preparation from blanket PCB readiness; [KiCad preparation](../../core/src/artifact/kicad/planning.rs),
`require_mechanical_export_clearance`, gates PCB-scoped violations rather than all
Case conflicts. These are examples of output-specific authority, not proof that every
button currently implements it correctly.

**Disposition:** explain that current accepted/prepared manufacturing inputs and their
applicable blockers are required; a disposable preview does not grant/revoke an export
lease. Amend DESIGN/reference wording, not 0016. A narrower actual UI gating audit is
needed before alleging a new runtime defect. No universal “all warnings allow export”
rule should be inferred across unrelated output scopes.

### E4 — Scripting remains described as ordinary architecture/migration work

**Classification: scope drift in connected docs; removal behavior undecided.**
[Architecture's responsibility table](../architecture.md) still lists geometry scripts
in shared UI. That can accurately inventory retained code, but it lacks
[0029's scope correction](../adr/0029-constrain-geometry-scripts-and-apply-results-atomically.md#scope-correction-2026-10-20).
[0006](../adr/0006-replace-document-for-import-and-recovery.md) still lists geometry-script
apply among bulk workflows needing classification. It predates the user's rejection
of the feature premise; it is not a fresh mandate to implement script ownership.

**Disposition:** annotate retained scripting as compatibility/removal work, and mark
0006's script example superseded by 0029 while preserving its dated rationale. Decide
saved-project compatibility/removal separately before deleting code. No script
sandbox/plugin/placement-ownership ADR is proposed. This research does not establish
how many saved projects contain scripts or authorize their silent removal.

### E5 — Amendment precedence is correct but easy to misread

**Classification: historical supersession/readability risk, not a new policy conflict.**
[0005](../adr/0005-resolve-queued-edits-at-execution.md) initially describes an explanation
for vanished targets, then its silent-retirement amendment says retired tickets show
no message. A Core refusal and an observation retired because its owner departed are
not necessarily the same terminal outcome. [Architecture](../architecture.md) gives
the current settlement summary: silent retirement, inline failure, no Saved status.
Do not resolve this wording by guessing that every target disappearance must be silent.

[0010](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) retains earlier
off-grid/pad-symmetry discussion, later explicitly rejects pad checks, and finally
supersedes adapter-owned mirrored setup creation with atomic Core creation. These
are traceable amendments, not simultaneously enforceable instructions. The older
linked-layout description in [DESIGN](../../DESIGN.md#workspace-navigation-and-preview-switching)
also must not be generalized to native whole-half reflection under 0008/0010.

**Disposition:** add a short current-rule/precedence guide to the existing records or
their references, distinguishing error from retirement and legacy layout links from
native physical pairs. Preserve the rejected proposals, dates and numbering. No new
ADR and no inferred implementation-completion claim.

### E6 — Known implementation debt must not be misreported as contradictory intent

**Classification: acknowledged policy/code disagreement, not newly discovered scope.**

- [0017/0018](../adr/0017-forgiving-vendor-previews-with-preserved-source.md) allow useful
  partial/repaired display and approximate observations, not universal certification
  admission. [Current CAD policy](../plans/cad-backend-replacement/map.md#current-vendor-preview-policy-2026-10-19)
  explicitly overrides older strict text. [Successor spec](../plans/cad-backend-replacement/spec.md#successor-task-graph)
  already publishes 015→016→{017,018}; parent 007 remains claimed. The inspected
  [STEP tests](../../cad/monstertruck/tests/step_import.rs),
  `dropped_face_or_unreferenced_solid_cannot_succeed_with_partial_mesh`, still assert
  strict refusal in a bounded fixture. That is not reason to preserve blanket refusal;
  nor should every strict safety test simply be inverted.
- [0025](../adr/0025-delete-a-removed-halfs-exclusive-pcb-design.md) chooses exclusive
  design/accessory deletion, whereas [setup](../../web/crates/catalogue/src/physical_setup.rs),
  `propose_topology`, clones the document and replaces instances without deleting
  boards. [0028](../adr/0028-convert-reversibility-explicitly-without-merging-designs.md)
  chooses reviewed conversion, whereas `propose_reversible` normalizes definitions
  and derives flips from labels. These source contradictions substantiate the ADRs'
  own “not implemented” notices, not a new retention/normalization rationale.
- [0026](../adr/0026-decouple-firmware-role-from-physical-side.md) permits either central
  side where supported; [Runtime](../../web/crates/runtime/src/runtime.rs),
  `firmware_bytes`, still requires left-central/right-peripheral. Do not amend the
  accepted policy to match this restriction.
- [0024](../adr/0024-share-reversible-pcb-routing-not-half-firmware-configuration.md),
  [0027](../adr/0027-save-incomplete-split-designs-with-output-specific-readiness.md)
  and [0030's amendment](../adr/0030-export-firmware-as-an-editable-repository-template.md#amendment-unfinished-optional-peripheral-support-2026-10-20)
  explicitly limit their implementation claims. This report does not prove every
  per-instance assignment, incomplete-save or optional-peripheral scenario works.

**Disposition:** retain intended-policy and implementation-status distinctions. Use the
existing CAD tickets, not four duplicate successor tickets. Topology/firmware delivery
planning is separate work; do not claim acceptance from this inventory.

## Ranked further decision candidates

### C1 — A PCB handoff creates durable external-world knowledge outside Undo

**Candidate sentence:** successful PCB package preparation and accepted protection
establish a signal-assignment baseline that ordinary Undo cannot erase; explicit
review is required to start a remapped PCB revision.

**Scope delta:** 0013/0016 cover saving and export ownership, 0019 covers Undo asset
retention, and 0024 binds firmware to real assembly. None records why a manufacturing
handoff's protection survives restoring an earlier document. This is not firmware
repository-template delivery under 0030.

**Current behavior and tests:** [electrical source](../../core/src/electrical.rs),
`preserve_handoff`, explicitly says a downloaded handoff is outside history;
[Core](../../core/src/lib.rs), `history`, reapplies the prior protected baseline
before publishing Undo/Redo. `handoff_assignments` protects fabricated signal
terminals rather than conflating population GPIO aliases. The public Runtime PCB
path calls [package_then_protect](../../web/crates/runtime/src/pcb_handoff.rs), which
packages first, validates ownership, commits protection, then checks delivery.
`failed_package_retry_and_stale_owner_use_session_protection_order` exercises failure,
retry and reopened-owner rejection. [Electrical tests](../../core/tests/electrical_wiring.rs),
`handoff_cannot_be_silently_overridden_by_new_locks`, checks protected-pin diagnostics.
These inspected tests do not establish browser download completion or fabrication.

**Recovered rationale:** [DESIGN](../../DESIGN.md#wiring-and-physical-assemblies) explicitly
requires protection even for draft handoffs and survival across Undo; H5 contains
that rule and its implementation provenance. The source comment supplies the
external-world explanation. Ordinary snapshot restoration is a genuine contrasting
behavior, not a recovered separately discussed rejected design. The exact product
definition of “successful handoff” remains worth confirming: package/protection can
succeed before actual user download completes.

**ADR criteria:** reversal cost **high** (previously routed/fabricated connections);
surprising **yes** (Undo does not restore this field literally); trade-off **supported**
(external knowledge versus ordinary history restoration), but historical user
selection among protocols is not independently recovered.

**Confidence/action:** high behavior/documented contract; medium original choice
provenance. **New ADR after confirmation**, narrowly about external handoff protection,
not a universal exception to Undo. Do not silently combine it with all export limits.

### C2 — Key identity versus firmware position when the layout evolves

**Candidate sentence:** bindings belong to explicit current input identities, while
changes to physical layout create a new hardware/keymap revision rather than promising
automatic semantic migration to visually similar replacement keys.

**Scope delta:** 0010 says key bindings are not mirrored; 0024/0026 distinguish physical
population, mapping and role. None fully chooses identity survival after deletion,
recreation, row/column changes or different logical ordering. This is not a new firmware
delivery-format decision.

**Current behavior and tests:** [keymap](../../core/src/keymap.rs), `KeymapLayer`, stores
typed bindings by string key ID; `apply_edit` admits current switch/matrix press or
qualified push identities; `source` looks up bindings using the supplied ordered
key list, falling back to legacy bindings/none on the base layer and transparent on
higher layers. [Keymap workflow tests](../../core/tests/keymap_workflow.rs),
`public_keymap_edit_and_undo_preserve_keycaps_and_legacy_data`, exercises typed/legacy
compatibility and history. [Keymap edit tests](../../core/tests/keymap_edits.rs),
`bindings_create_defaults_preserve_geometry_and_support_history` and
`encoder_push_bindings_use_the_same_membership_and_merge_path`, cover narrower identity
and board membership boundaries. No inspected test proves deletion/recreation across
all matrix operations or per-half typed-binding independence for a shared Board.

**Rationale:** [PRODUCT](../../PRODUCT.md#capabilities-and-constraints) and
[DESIGN](../../DESIGN.md#wiring-and-physical-assemblies) explicitly disclaim automatic
migration and deleted-key tombstones. H5 shows the disclaimer existed by September 26;
it does not establish why that trade-off was selected. Persistence by ID is observed;
“a new hardware revision” is not proof of a persisted revision object or automatic
export migration. Do not infer those features.

**ADR criteria:** reversal cost **high** (saved binding meaning); surprising **yes**
(same-looking replacement need not be same key); genuine trade-off **not recovered**
for deletion/reuse/position migration. Identity-following versus position-following
are proposed choices for clarification, not previously rejected alternatives.

**Confidence/action:** high representation, medium complete lifecycle, low historical
rationale. **Ask user after a bounded delete/recreate trace**, then new ADR only if
the scenario establishes a durable choice. Explain the existing typed/legacy maps in
architecture meanwhile; do not promise or implement tombstones from this report.

### C3 — Project-owned component snapshots versus upstream catalogue refresh

**Candidate sentence:** a placed project's definitions and assemblies remain its own
snapshots; upstream catalogue changes become project changes only through an explicit
reviewed refresh, not by silently replacing same-named parts on reopen.

**Scope delta:** 0003 already chooses manual Rust upstream generator ports and retained
licences; 0009 chooses admission; 0023 chooses mounted versus embedded circuitry.
The missing question is update authority and variant lineage across a library release
and a project, not another “copy versus mount” ADR.

**Current behavior and tests:** [catalogue](../../web/crates/catalogue/src/catalogue.rs),
`merge_project_overrides`, gives project definitions precedence by ID;
`imported_and_project_overrides_keep_the_first_id_position` tests that behavior.
`saved_vik_parts_are_not_new_catalogue_choices` separates saved resolution from new
placement eligibility. [Assembly reference](../features/assembly-preview.md) says saving
an assembly does not rewrite existing placements; placement snapshots definitions,
and explicit matrix application retains layout/key IDs and remaps logical terminals.
[DESIGN's Parts catalog](../../DESIGN.md#parts-catalog) explicitly retains retired/custom
assignments and describes an undoable Update assembly preset. H6 records the earlier
curation/legacy-preservation intent. These facts do **not** prove that updated generator
bodies cannot change a saved definition's rendered result: generator execution is
still current Rust code under 0003. Snapshot data is not a pinned executable generator.

**Rationale:** preserving saved/custom choices and avoiding involuntary replacement is
documented; complete upstream-version/variant refresh semantics and rejected update
strategies were not recovered. Automatic update versus explicit project refresh is a
proposed scenario, not a claim that all current refresh paths implement either policy.

**ADR criteria:** reversal cost **high** (footprints, nets and mechanical fit in saved
projects); surprising **yes** (same catalogue ID can have project-owned data);
genuine trade-off **partly documented**, not settled for upstream executable changes.

**Confidence/action:** high merge/legacy behavior, medium overall policy. **Ask user**
about a concrete changed-footprint release, and distinguish data refresh from generator
semantic compatibility first. Prefer an **0003 amendment** if the only decision is
generator upgrade compatibility; use a new ADR only for a broader confirmed project
variant/update ownership contract.

### C4 — Renderer authority is recoverable; automatic context restoration is not

**Research sentence:** rendering owns GPU/live camera state and returns candidate hits;
application owners validate current semantic identity, while context loss must settle
visibly without damaging document authority.

**Scope delta:** most authority/lifetime content already belongs to 0014. A genuinely
new choice would be whether and how to reconstruct a lost GPU context and preserve
camera/selection across it; no current ADR promises automatic restoration.

**Current behavior and tests:** [Renderer](../../renderer/src/wasm.rs), `Renderer`,
`orbit`, `zoom` and `pick`, owns camera/GPU state and intersects visible triangles,
prioritizing editing handles. [Host](../../web/crates/host/src/renderer_host.rs),
`install_lifecycle`, marks context lost, cancels frames and reports failure; camera
operations ask the user to reopen the preview. `page_operations_refuse_disposed_and_context_lost_renderers`
and `context_loss_settles_callers_stops_rendering_and_unmount_releases_everything`
exercise terminal guards/cleanup with a scripted renderer, not real-driver restoration.
[Layout source](../../web/crates/runtime/src/layout_viewer_source.rs),
`LayoutPreviewSnapshot::part_for_current_pick`, validates captured ownership and
resolves a current unique part; `picks_require_full_current_scope_source_generation_and_unique_board_reference`
rejects stale, cross-instance and ambiguous hits. Thus renderer IDs alone are not
document selection authority. Case's full selection behavior was not exhaustively audited.

**Recovered rationale:** H1 §§2/5 explicitly distinguishes renderer live 3D camera
from Session state and rejects making the private renderer payload a universal domain
scene or building a universal scene graph. H2 retires migration machinery, not evidence
that this distinction was reversed. H7/its [cleanup ticket](../plans/cleanup/issues/03-deduplicate-renderer-host-page-snapshot.md)
preserve the checked host contract, but supply no recovered product choice between
automatic restoration and explicit reopen.

**ADR criteria:** authority reversal cost **meaningful**, surprise **yes**, documented
alternatives **yes**, but substantially covered by 0014; context-restoration cost
**unestablished**, surprise **possible**, actual recovery trade-off **unknown**.

**Confidence/action:** **amend 0014/explain architecture** for recovered authority;
**research, no new recovery ADR now**. Do not promote current fail-stop behavior into
a permanent prohibition on automatic recovery or assume camera persistence is promised.

### C5 — Portable attribution/provenance is not the same as archive integrity

**Research sentence:** source identity, licence and qualification evidence should have
an explicit propagation boundary when a component/model moves from catalogue to
project archive and downstream manufacturing package.

**Scope delta:** 0020 authenticates packaged bytes; 0017 preserves original vendor
geometry; 0023 preserves circuit source ownership; 0003 retains generator licences.
None by itself states which notices/source evidence every exported package must carry
or how a customized variant identifies its upstream versus authored changes.

**Current evidence and tests:** [onboarding](../hardware/component-onboarding.md#onboarding-workflow)
requires source revision/licence/units/hashes and distinguishes imported, previewable
and fabrication-qualified. [Bundled models](../../web/crates/runtime/src/bundled_models.rs),
`BundledModel` and `bundled_model`, map stable identities to staged files and digests;
`catalogue_digest_matches_the_exact_staged_vendor_source_file` checks staged bytes and
`lookup_does_not_turn_unknown_identifiers_into_static_paths` rejects invented paths.
`footprint_export_model_ids` enumerates model dependencies, and [PCB handoff](../../web/crates/runtime/src/pcb_handoff.rs),
`build_handoff`, packages board/models and assembly reports. These inspected paths
are not an end-to-end audit of attribution contents. H8 establishes earlier portable
lookup/build-provenance work, not a comprehensive legal-distribution policy.

**Rationale/alternatives:** source reproducibility is explicit, but no primary evidence
was recovered choosing between an app notice, per-project notices and per-output
attribution. These are possible boundaries to investigate, not legal conclusions or
previously rejected options. No licence violation is established by this report.

**ADR criteria:** reversal cost **potentially high** (lost provenance in distributed
artifacts); surprise **plausible** (valid hash is not permission/qualification);
actual selected trade-off **unknown**.

**Confidence/action:** **research first**, then amend onboarding/0020 if it is simply
enforcing an existing requirement. New ADR only if a meaningful portability/scope
choice remains. Do not create one ADR per vendor, component or licence.

## First domain conversations and concrete questions

Only three conversations are justified now; these are not a five-ADR writing queue.

1. **PCB externalization (C1):** “A valid draft PCB ZIP is packaged, protection saves,
   but the browser download fails or is cancelled. Should remapping still require
   review? Current ordering can retain protection before delivery completes. Is that
   the intended success boundary?” Also preserve 0025's exclusive-design deletion:
   a future deletion implementation must decide how a removed board's handoff baseline
   is retired/restored without dangling configuration; this is not permission to
   retain the removed PCB contrary to 0025.
2. **Layout/keymap evolution (C2):** “Delete a bound matrix key, save, then recreate a
   key in that cell without Undo. Should it inherit the prior binding, remain
   unassigned, or require explicit review? Moving an existing input and replacing it
   must be distinguishable.” Agent work first: trace actual ID allocation/removal,
   surviving binding maps, export ordering and reopen, rather than asking the user
   what current code does. Per-instance meanings must remain compatible with 0024.
3. **Catalogue refresh (C3):** “An upstream switch revision moves a pad/model datum
   while your saved project overrides that definition. Must opening preserve the
   existing result, and what explicit review applies the new revision? Does the answer
   differ when only the Rust generator body changes?” Existing precedence and legacy
   resolution are facts above; the unanswered question is the promised upgrade boundary.

Renderer recovery and provenance packaging should start with narrower agent traces,
not broad user interviews about facts discoverable from source. No unresolved question
here reopens the already-decided CAD admission, topology deletion, role, conversion,
scripting scope or firmware template direction.

## Exclusions and no-ADR conclusions

- **Physical/design coordinates:** millimetres are already product/reference policy;
  0007 assigns matrix axes to Core, 0010 owns reflection/frames, and 0015 owns accepted
  manufacturing geometry. [Onboarding](../hardware/component-onboarding.md#onboarding-workflow)
  requires composing placement/face transforms once. [STEP import tests](../../cad/monstertruck/tests/step_import.rs),
  `absent_units_mean_mm_and_metre_coordinates_convert_once` and
  `shared_exact_solid_keeps_both_rotated_and_translated_occurrences`, provide bounded
  scale/placement examples. Record an architecture transform table before proposing
  a global coordinate ADR; no recovered alternative justifies another record now.
- **Undo broadly:** [Core history](../../core/src/lib.rs), `history` and
  `publish_prepared_edit`, uses document snapshots and a bounded edit history; `Open`
  clears it. 0013 already limits history survival on reopen/executor failure and 0019
  protects live assets. Full-snapshot versus delta history or the numeric cap is not
  newly ADR-worthy merely because implemented. C1 isolates the substantive external
  handoff exception. Monotonic mechanical IDs are already explained in 0004 and the
  [identity-reservation ticket](../plans/rust-best-practices/issues/02-preserve-monotonic-mechanical-identities.md).
- **Imported routed board authority:** [assembly reference](../features/assembly-preview.md)
  explicitly makes a routed `.kicad_pcb` read-only assembly source, not replacement
  electrical/export data. `LayoutSourceCapture::capture` selects imported preview
  bytes, while `build_handoff` prepares the accepted Board's generated export.
  `imported_capture_selects_matching_accepted_asset_and_existing_preview_operation`
  tests preview source choice. This is a valuable boundary to explain in architecture
  or alongside C1, but a separate ADR's original trade-off was not recovered. No
  bidirectional KiCad round-trip or imported-board fabrication authority is inferred.
- **Catalogue readiness:** onboarding already separates preview from qualification;
  0023/0027/0030 constrain output-specific behavior. Do not create a competing blanket
  “unknown peripheral blocks everything” rule, particularly after 0030's amendment.
- **Not traced fully:** every CAD contract/provider, 30 ADR implementations, all legacy
  script compatibility/removal paths, every key deletion/resize/reuse combination,
  whole catalogue refresh and licence propagation, physical hardware qualification,
  real GPU context restoration, every saved-project recovery UI, or complete PCB
  deletion with surviving handoff records. No full security/geometry/legal audit claim.
- No ADR for CSS tokens, exact crate graphs, routine resource limits, compiler flags,
  individual fixtures or per-component exceptions. No builds, tests, browser automation,
  firmware compilation, mutation runs, child agents or commits.

## Historical evidence examined

Git object references are stable provenance, not links to missing live files. Retrieve
with `git show <commit>:<path>`. Commit messages establish stated implementation intent,
not an independent transcript of user approval.

| ID | Verified object | Bounded use in this report |
| --- | --- | --- |
| H1 | `323967ff3a67b9b62db4f201b477396d62b55ccd:docs/adr/0003-rust-application-ownership.md` | Read original record, particularly §§2/5: camera/document/selection ownership, neutral provider contracts and actual alternatives. Exclude obsolete trial/Cadrum/exact package-graph proposals. |
| H2 | `3cdeb2ac2b2b3e11891d10122b03e6dc835e03dd` | Retirement commit message explicitly removes React/migration machinery, not every living ownership principle. |
| H3 | `8206af04a1fb38758141cc615d0815debd0fe203` | CAD checkpoint provenance referenced by accepted 0015; current user scope is read from 0015 and the CAD map, not inferred solely from this WIP commit title. |
| H4 | `770cde44e931a4050edfd107844af81df1ee6f6e` | Inspected exact 0004 diff adding the reservation-counter paragraph after existing migration work. |
| H5 | `e7f1628b8cadf947460d8d075f26bf0da742693f:DESIGN.md` | Read historical Wiring and physical assemblies: keymap migration disclaimer, protected draft handoffs, Undo survival and explicit remap. Commit message names exported mapping protection. |
| H6 | `12f721efb520cd0f0f5ae6af07f862ac6a3c6eb2` | Curation commit message explicitly preserves legacy/custom parts and companion choices, not a complete future refresh/versioning contract. |
| H7 | `eab77de9aeac5722d46b018657ca0e050e804035` | Renderer host consolidation provenance, read with current cleanup ticket's checked contract/Answer; no new claim of executing its historical receipts. |
| H8 | `9582132875d3e8fa9958a19bd3da53b4c61241ff` | Portable model lookup/catalogue build-provenance implementation marker only; detailed rejected licensing/distribution alternatives remain unknown. |

## Verification and limitations

- Read the complete report back for unsupported intent/completion claims. Verified
  98 local links/anchors, 18 named test declarations and nine cited commit objects
  (including HEAD). Named production symbols were inspected in their owning source.
  Historical receipts were read as documentation, not regenerated.
- All named tests are **inspected, not run**. In particular, scripted-renderer tests
  are not real-driver recovery evidence, keymap edit tests are not full layout-evolution
  acceptance, and package/protection tests are not proof of a completed browser download.
- Checked the report path for conflict before creation. No existing ADR, glossary,
  index, plan, source, test or concurrent change was intentionally edited, staged or
  reverted. The source tree can continue changing after inspection; recheck the
  specific area before implementing any recommendation.

**Next step:** reconcile the existing references, then discuss C1–C3 in concrete
scenarios. Do not manufacture additional ADRs merely to reach a target count.
