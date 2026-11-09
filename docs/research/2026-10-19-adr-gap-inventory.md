# BoardStudio ADR gap inventory

**Examined:** 2026-10-19. **HEAD:** `9099b3ef7a72b0919b1572f116490ed57514d43a`.
**Status:** research, not proposed or accepted ADRs. No decision dates are assigned by this report.

## Subsequent decision review — 2026-10-19

The user reviewed all 12 groups and confirmed 11 ADRs (G2 and G3 share one record).
The original findings below remain historical research, not an updated implementation
assessment. In particular, G6's strict preview-admission direction was replaced, not
reaffirmed; the accepted policy allows repaired and partial previews with diagnostics.

| Research group | Accepted record |
| --- | --- |
| G1 durable publication/recovery | [0013](../adr/0013-save-before-publication-and-explicit-recovery.md) |
| G2 authority and G3 captured lifetimes | [0014](../adr/0014-document-authority-and-captured-lifetimes.md) |
| G5 exact generated manufacturing | [0015](../adr/0015-exact-generated-cad-and-mesh-preview-scope.md) |
| G4 export ownership | [0016](../adr/0016-export-ownership-is-separate-from-preview-currency.md) |
| G6 vendor import — revised direction | [0017](../adr/0017-forgiving-vendor-previews-with-preserved-source.md) |
| G7 collision observations and conclusions | [0018](../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md) |
| G8 multi-tab saves and cleanup | [0019](../adr/0019-reject-stale-saves-and-reclaim-assets-conservatively.md) |
| G9 portable package integrity | [0020](../adr/0020-validate-portable-project-packages-in-core.md) |
| G10 native/browser test boundary | [0021](../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md) |
| G11 static local-first platform | [0022](../adr/0022-static-local-first-rust-application-with-worker-execution.md) |
| G12 mounted/embedded ownership | [0023](../adr/0023-distinguish-mounted-daughterboards-from-embedded-circuits.md) |

## Executive summary

The largest gap is not a missing ADR for each module. It is the loss of the durable
ownership, failure and asynchronous-lifetime rationale formerly collected in the
removed application-ownership ADR, followed by substantial newer CAD decisions that
live in tickets rather than the ADR index. The removal commit retired migration
machinery; it does not establish that every architectural rule in that ADR was
reversed ([historical evidence](#historical-evidence), H1–H3).

This inventory investigates **12 candidate decision groups**: **9 with recovered
historical rationale** and **3 implemented groups with incomplete or unrecovered
choice rationale**. Already-covered decisions are mapped separately. The ranking
below prioritizes changes likely to
destroy work, deliver stale output or overclaim manufacturing accuracy. It does not
recommend 12 new ADRs. Existing ADRs already cover ordinary edit intent resolution,
typed-edit direction, outline ownership, mirroring and schema compatibility
([coverage map](#existing-adr-coverage-map)).

The strongest writing candidates are:

1. Save-before-publication and explicit recovery without replaying an uncertain edit
   ([G1](#g1-durable-publication-and-explicit-recovery)).
2. One durable document authority with a headless Session, plus captured asynchronous
   identity rather than revision-only freshness
   ([G2](#g2-one-document-authority-and-headless-session-ownership),
   [G3](#g3-captured-lineage-and-asynchronous-publication)).
3. Export ownership distinct from disposable preview currency, including the
   post-settlement delivery exception ([G4](#g4-export-leases-are-not-preview-currency)).
4. Exact layered case/PCB output versus mesh-only keycaps/vendor previews, and
   evidence-bound collision conclusions that retain Unknown
   ([G5](#g5-exact-generated-manufacturing-from-accepted-inputs),
   [G6](#g6-vendor-step-source-retention-and-bounded-preview-repair),
   [G7](#g7-collision-conclusions-require-owned-qualified-evidence)).

**Important uncertainty:** vendor repair accounting is still incomplete. Neither the
34-asset compatibility checkpoint nor successful collision delivery certifies every
repair route. The working-tree CAD map changes 007 from committed `claimed` to
`ready-for-agent`, while ticket 007 still says `claimed`; that discrepancy is an
uncommitted tracker edit, not a new architecture decision. Its dated current-work
section is more specific about ongoing implementation.
Do not turn its final strict terminal policy into an already-enforced guarantee
([007 current work](../plans/cad-backend-replacement/issues/007-strict-step-import.md#current-work--2026-10-19),
[CAD map](../plans/cad-backend-replacement/map.md#slices--current-status)).

## Method, provenance and limitations

- Read root [AGENTS.md](../../AGENTS.md), [domain instructions](../agents/domain.md),
  [GLOSSARY.md](../../GLOSSARY.md), the [ADR index](../adr/README.md), all 12 existing
  ADRs, [architecture](../architecture.md) and [documentation index](../README.md).
  Followed selected source, named tests, issue answers and Git history. Source
  exploration began with CodeGraph; returned source was treated as already read.
- **Committed versus working tree:** the initial checkout had substantial concurrent
  edits. The architecture's observed diff was only `CONTEXT.md` → `GLOSSARY.md` in
  its link; ADR-0005's observed diff likewise changed glossary links, not policy.
  Domain instructions and the docs index were also modified, and `GLOSSARY.md` was
  untracked. These working-tree references are vocabulary/navigation evidence, not
  proof that a new architectural decision was committed. The module-deepening and
  check-speed maps' examined diffs likewise only rename glossary references. The CAD
  map's examined diff only changes 007's role label; its cited scope/acceptance
  decisions are unchanged from HEAD. Other dirty plans and code
  were not staged, reverted or normalized. `git status --short`, `git diff` and
  `git rev-parse HEAD` establish this distinction; this was not an atomic repository
  snapshot.
- Unless identified otherwise, implementation links below describe current source
  inspected against the recorded HEAD. Named tests are **existing behavioral
  evidence inspected, not tests executed in this research**. Code and tests show
  what happens; only explicit decision records, contemporaneous explanations or
  commit messages recover why. A plausible engineering benefit is labeled inference.
- Historical plans can contain superseded criteria. In particular, CAD 013's
  prototype-only wording and 014's early “all bounds unknown” checkpoints are not
  current production status: their later Answers/current sections and the source
  supersede those checkpoints. Conversely, a plan's approved declaration does not
  prove that declaration landed ([013 Answer](../plans/cad-backend-replacement/issues/013-layered-case-construction.md#answer),
  [014 current](../plans/cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md#current--2026-10-18-rescope)).
  The original [Monstertruck feature assessment](cad/monstertruck-feature-assessment-2026-10-16.md#finding)
  explicitly marks its broader Boolean/exact-keycap API inventory as historical
  after the 2026-10-17 scope change; its earlier “production still uses Cadrum”
  checkpoint must not be read as present status.
- No external research, build, test, browser run, benchmark or new acceptance check
  was performed. Old `.evidence/` receipts referenced by tickets were not recreated
  or independently rerun. This is a focused ownership/durability/CAD inventory, not
  an exhaustive security, hardware or UI audit.

### Historical evidence

These are stable Git object references, not links to deleted working-tree files.
Retrieve an old source with `git show <commit>:<path>`.

| ID | Commit and primary evidence | What it can establish |
| --- | --- | --- |
| H1 | `323967ff3a67b9b62db4f201b477396d62b55ccd:docs/adr/0003-rust-application-ownership.md`; originally added in `38bbe42ff7ebda72170403b66bdf98b930928baa` | The removed ADR records user approval on 2026-10-01 and explicitly discusses ownership, worker identity, preview/export separation, persistence failure and alternatives. It also contains trial-only proposals and obsolete CAD/version assumptions. |
| H2 | `3cdeb2ac2b2b3e11891d10122b03e6dc835e03dd` | “Retire React and migration machinery after Dioxus cutover”; deletes the old ADR and replaces migration delivery gates with ordinary development/release checks. Its parent is H1. |
| H3 | `f2494e5714deb726b2e92a392a86bce27de1695a` | Adds the headless durable Session, its README and controlled native tests. This is implementation provenance, not separate user approval. |
| H4 | `28b9d81d2`, `3d432f520e88bb7e8be47a1c734aeb01afe28dd6`, `11ac3786b4026980f0d0b803459cde457dcf6e5a`; [export-lease outcome](../plans/module-deepening/issues/10-export-leases.md#outcome) | Centralized export leases; subsequent correction preserves delivery after Session settlement. |
| H5 | `35991dce0`; [native Runtime decision](../plans/module-deepening/issues/02-decide-native-test-runtime.md#answer) | Explicit user choice of a real Session/Core native Runtime and shared gate vocabulary, rather than a second resolver/session implementation. |
| H6 | `5cb0a47f0276ca3b16314b728730cd6e8cbc3c9c`; its README addition “Local project safety” | Documents conflicts preserving pending edits, deferred asset cleanup, unreadable-record recovery, a 100-edit Undo limit and 128 MiB project imports. Does not enumerate rejected storage designs. |
| H7 | `8d481b7a8743ed7c8b62f47253e6d00b3096bca6` | Delays offline-worker activation until old clients close. |
| H8 | `91eb771f3526d7428b02d4ffcdba8beb44579f0a`; [CAD scope decisions](../plans/cad-backend-replacement/map.md#scope-decisions-2026-10-17-user) and [closure rescope](../plans/cad-backend-replacement/map.md#closure-rescope-2026-10-18-user) | Recorded user-approved exact/mesh split, adoption of layered construction, named refusals and bounded closure evidence rather than universal geometry certification. |
| H9 | `68224a7725074e4684de31b7516f2099cf32b325`; [fixture onboarding answer](../plans/typed-core-edits/issues/167-guide-complete-collision-test-fixtures.md#answer--2026-10-18) | Runtime owns complete collision execution; controlled-worker tests and actual-provider qualification are deliberately different evidence. |
| H10 | `2dc09ebaeba1905fb3be004629fad5d07f7941b9`; split sequence through `bb2e1ebb2` | Shared presentation vocabulary and workspace crates were extracted; commit text explains avoiding feature dependence on shell-specific types. Not a recovered decision to freeze every dependency edge forever. |
| H11 | `5a472a9426e6e38993361da402cd4ec730feb369`; [onboarding guide](../hardware/component-onboarding.md#onboarding-workflow) | Documents mounted source ownership, independent embedded copies and evidence-specific hardware readiness. The architectural choice's rejected alternatives are not recovered here. |

## Ranked candidate table

“Recovered” means rationale is present in the cited historical record, not that this
report newly accepts it. **C/S/T** rates costly reversal, surprising behavior and a
genuine choice/trade-off respectively; “?” means rationale is not recovered.

| Rank / group | Classification | C/S/T | Coverage and disposition | Confidence |
| --- | --- | --- | --- | --- |
| 1 / G1 durable publication/recovery | Historical rationale recovered | Y/Y/Y | Missing; write a bounded ADR | High |
| 2 / G2 authority and owner split | Historical rationale recovered | Y/Y/Y | Partly 0005/0006; write ownership ADR, not old migration ADR | High |
| 3 / G7 qualified collision conclusions | Historical rationale recovered | Y/Y/Y | Missing; write with explicit supported classes/Unknown limits | High |
| 4 / G5 exact generated manufacturing | Historical rationale recovered | Y/Y/Y | 0001/0002/0012 cover outline side only; write CAD decision | High |
| 5 / G6 source-retaining vendor import | Historical rationale recovered | Y/Y/Y | Missing; write delivered policy with unfinished strict policy separate | High on scope; medium on complete enforcement |
| 6 / G4 export lease versus preview | Historical rationale recovered | Y/Y/Y | 0005 strict route is partial coverage; write/export amendment | High |
| 7 / G3 async captured lineage | Historical rationale recovered | Y/Y/Y | 0005 covers edits only; write cross-operation lifetime contract | High |
| 8 / G8 multi-tab save/asset safety | Implemented; rationale partly inferred | Y/Y/? | Missing; confirm protocol choice before accepting ADR | High behavior; medium intent |
| 9 / G9 portable archive trust boundary | Implemented; rationale unknown for chosen envelope | Y/Y/? | 0004 covers migrations only; clarify rationale first | High behavior; low choice history |
| 10 / G10 real native Runtime plus mounted checks | Historical rationale recovered | N/Y/Y | Missing testing-architecture decision; small ADR | High |
| 11 / G11 static local-first Rust/worker host | Historical rationale recovered | Y/Y/Y | Missing; bound claims to current platform, not trial matrix | High ownership; medium current support policy |
| 12 / G12 mounted/embedded hardware ownership | Implemented; full rationale not recovered | Y/Y/? | Partly 0010; hold for focused hardware decision interview | Medium |
| — / edits, outlines, mirrors, parameters | Already covered | — | Do not duplicate existing ADRs | High on coverage; no blanket implementation-completion claim |

## G1 Durable publication and explicit recovery

**Decision statement to recover:** Core can advance before storage, but Session
publishes an accepted document only after persistence commits. A failed save
retains that exact result for retry, blocks dependent work, and does not replay the
edit. An unknown executor outcome requires explicit recovery/reopen.

- **What:** [Session](../../application/src/session.rs), `persist_completed`,
  `retry_save`, `core_failed` implement that split. `SaveResult::Committed` installs
  the new `AcceptedSnapshot`; an abort retains `PendingSave` and enters
  `RecoveryRequired`. [Browser persistence](../../web/crates/host/src/host/storage.rs),
  `persist_project`, awaits transaction completion rather than just request success.
- **Named tests:** [durable_session.rs](../../application/tests/durable_session.rs),
  `retry_persists_retained_commit_without_replaying_engine_edit` and
  `uncertain_worker_outcome_is_never_replayed_and_requires_explicit_reopen`;
  [storage_lifetime_tests.rs](../../web/crates/host/src/host/storage_lifetime_tests.rs),
  `normal_completion_is_saved_and_detaches_handlers` and
  `explicit_abort_preserves_caller_reason_and_rolls_back`.
- **Why recovered:** H1 §6 identifies engine/UI divergence after save failure and
  explicitly chooses fail-stop/retry rather than losing history by reopening.
  Documented alternatives are persist-first engine transactions, publish-before-save
  and immediately reopening the old snapshot. H3 lands the Session boundary.
- **Coverage/consequences:** ADR-0005 governs queued intent, not durable publication;
  ADR-0006 permits recovery replacement, not its failure policy. Editing can be gated
  during recovery; retry preserves the live engine's history, whereas worker-crash
  recovery cannot promise history survival (H1 §6; Session tests above).
- **Disposition:** new bounded ADR. Costly data/history semantics, a surprising
  “engine succeeded but accepted state has not advanced” interval, and documented
  alternatives make this a strong gap. **Question:** should the replacement ADR
  explicitly reaffirm this fail-stop behavior as the continuing user contract?

## G2 One document authority and headless Session ownership

**Decision statement to recover:** Core owns document edits/history/resolved domain
geometry; Session owns ordered application state and accepted snapshots; Runtime
executes browser-facing workflows; Host owns browser providers; presentation keeps
drafts/focus/display, not another writable document authority.

- **What:** [CoreEngine](../../core/src/lib.rs) holds the document and undo/redo;
  [Session](../../application/src/session.rs) holds queue, accepted read model and
  transient preview; [Runtime](../../web/crates/runtime/src/runtime.rs) holds Session,
  executor/persistence ports and workflow state. [Architecture](../architecture.md)
  and the [Runtime](../../web/crates/runtime/Cargo.toml)/[Host](../../web/crates/host/Cargo.toml)
  manifests separate these owners from Dioxus presentation. This does **not** mean
  Runtime contains no browser handles: it still has SVG/frame/provider state.
- **Named test:** [durable_session.rs](../../application/tests/durable_session.rs),
  `preview_edit_displays_without_changing_the_accepted_document_or_history`, checks
  transient display versus accepted document/history. G10's real-Runtime tests
  exercise the same Session/Core ownership rather than a second engine.
- **Why recovered:** H1 §2 explicitly rejects coupling business lifetimes to a
  single Dioxus store and putting browser/application coordination inside Core.
  It distinguishes renderer GPU/live-camera state from Session navigation and UI
  drafts. [Module-deepening spec](../plans/module-deepening/spec.md#constraints)
  preserves Dioxus-free Runtime and locates Signal helpers above it.
- **Coverage/consequences:** 0005/0006 and 0007 establish specific Core edits and
  projections, not the whole ownership rule. A panel unmount is not authorization
  to discard an already-owned save (H1 §2). Do not turn every later Core operation
  into another ADR; those are applications of this seam.
- **Disposition:** write one ownership ADR, with G1/G3 references. Do not restore
  obsolete “one web package,” trial namespaces or Cadrum pins from H1. **Question:**
  is the current browser-handle-bearing Runtime the intended continuing split, or
  is further Host extraction only a refactoring possibility?

## G3 Captured lineage and asynchronous publication

**Decision statement to recover:** asynchronous work belongs to a captured session,
snapshot, scope and executor/job lifetime; cancellation retires publication rights,
not merely a UI future. Matching document ID/revision alone is insufficient.

- **What:** [Session identities](../../application/src/session.rs), `Scope`,
  `AcceptedSnapshot`, `ExecutorEpoch`, `SnapshotToken` and identified completions
  distinguish lifetimes. [EditTicket](../../web/crates/runtime/src/edit_ticket.rs)
  captures a scope source and latches retirement. [Runtime collision tests](../../web/crates/runtime/src/runtime_collision_browser_tests.rs)
  exercise accepted scene/preview/row identity and stale-result rejection.
- **Named tests:** `a_scope_departure_retires_every_ticket_clone_permanently`,
  `returning_to_the_captured_board_scope_does_not_revive_observation`, and
  `same_scope_revision_changes_and_owner_departure_have_separate_lifetimes` in
  [edit_ticket.rs](../../web/crates/runtime/src/edit_ticket.rs);
  `obsolete_false_view_entry_cannot_retire_an_active_complete_check` in
  [runtime_collision_browser_tests.rs](../../web/crates/runtime/src/runtime_collision_browser_tests.rs).
- **Why recovered:** H1 §§3–4 explains why revision-only checks miss reopening the
  same ID/revision and why dropping a UI future cannot undo externally dispatched
  work. It documents worker termination versus cooperative cancellation trade-offs.
  [Scope-lineage outcome](../plans/module-deepening/map.md#progress) records the
  later captured-Scope implementation. These sources do not establish that every
  old cache-delta protocol still exists unchanged.
- **Coverage/consequences:** 0005 preserves strict gesture/export routes and silent
  retirement, but does not document the general identity tuple or observation versus
  execution ownership. Leaving and returning to a scope cannot revive old
  observation; a later same-scope revision is a different question (named tests).
- **Disposition:** one lifecycle ADR, not an ADR per cache or cancellation token.
  Keep preview cache-delta resynchronization out unless current source is traced
  separately. **Question:** which identity/cancellation rules are intended public
  cross-provider invariants, versus adapter-private implementation choices?

## G4 Export leases are not preview currency

**Decision statement to recover:** an export owns a captured accepted snapshot and
strict origin; disposable preview state cannot grant or revoke that ownership.
An export's own accepted commit can advance its lease, but not relax its origin.

- **What:** [ExportLease](../../web/crates/runtime/src/export_lease.rs),
  `source_is_current`, `advance`, `require_current` and `ExportLeases`, centralize
  snapshot/scope/executor/worker ownership. [Runtime](../../web/crates/runtime/src/runtime.rs)
  distinguishes `require_export_current` for active work from
  `export_delivery_is_current` for final delivery.
- **Named tests:** `worker_replacement_invalidates_every_export_kind`,
  `registry_finishes_and_cancels_leases_by_operation`, and
  `advancing_a_lease_requires_a_new_consistent_revision` in
  [export_lease.rs](../../web/crates/runtime/src/export_lease.rs).
- **Why recovered:** H1 §§4/6 separates preview generations and committed export
  contexts. H4 and [ticket 10](../plans/module-deepening/issues/10-export-leases.md#outcome)
  explain duplication removal and a real correction: Session drops active operation
  ownership before delivery, so reusing the active-work predicate there suppressed
  valid delivery. This is stronger evidence than assuming a generic “all async work
  uses the same currentness function.”
- **Coverage/consequences:** ADR-0005 already covers strict captured-revision export
  commits; do not rewrite that choice. The uncovered part is lease lifecycle and the
  active-work/delivery distinction. H4 documents the per-kind predecessor; no separate
  rejected universal lease architecture was recovered.
- **Disposition:** small export-ownership ADR cross-referencing 0005 and G3.
  **Question:** should the post-settlement delivery distinction be recorded explicitly
  to prevent future “deduplication” into the active-work predicate?

## G5 Exact generated manufacturing from accepted inputs

**Decision statement to recover:** accepted, revision-bound manufacturing inputs
feed exact layered 2.5D case/PCB construction; display meshes are not exact STEP
authority. Keycaps are appearance/optional-print meshes, not exact keycap STEP.

- **What:** [Host CAD preparation](../../web/crates/host/src/cad_jobs.rs),
  `prepare_captured_case_for_step`/`validate_prepared`, obtains and checks Core-prepared
  accepted inputs. [Runtime](../../web/crates/runtime/src/runtime.rs),
  `prepare_physical_step_assembly`, shares complete physical PCB preparation between
  export and collision, retaining source contours/drills and rejecting mismatch.
  [CAD build_case](../../cad/monstertruck/src/lib.rs) calls
  `layered::build_layered_case`; [Core case preparation](../../core/src/case.rs)
  rejects mixed revisions.
- **Named tests:** [layered/tests.rs](../../cad/monstertruck/src/layered/tests.rs),
  `original_round_seat_chain_matches_independent_volume_and_material`,
  `countersink_uses_exact_cone_and_independent_frustum_integral`,
  `cone_crossing_vertical_wall_is_rejected_with_feature_identity` and
  `reviewed_sealed_opening_is_refused_with_opening_context`;
  [runtime_collision_browser_tests.rs](../../web/crates/runtime/src/runtime_collision_browser_tests.rs),
  `source_pcb_collision_preserves_curves_drills_placement_and_complete_cache_identity`.
- **Why recovered:** [013 Why](../plans/cad-backend-replacement/issues/013-layered-case-construction.md#why)
  identifies Z-axis features and failures in general 3D Boolean chains; its
  [Answer](../plans/cad-backend-replacement/issues/013-layered-case-construction.md#answer)
  adopts direct layered construction. H8 records user scope: exact cases/PCBs,
  mesh keycaps/vendor assets, no required general Boolean engine or final OCCT/C++
  fallback. [008](../plans/cad-backend-replacement/issues/008-step-export-independent-accuracy.md)
  requires independent analytic/reference evidence, not just same-kernel roundtrips.
- **Alternatives/consequences:** the general Boolean predecessor and polyline-only
  `i_overlay` versus line/arc `cavalier_contours` are documented in 013, not invented
  alternatives. Named cone-wall/sealed-void refusals remain product limits; the
  closure rescope explicitly does not demand a general STEP validator (H8).
- **Coverage/disposition:** 0001/0002/0012 govern valid source outlines; none records
  the CAD route or exact/mesh boundary. Write one CAD construction/output ADR,
  merging “accepted inputs, not rendered meshes” rather than creating a duplicate.
  **Question:** should the current exact/mesh output matrix and named refusals be
  the replacement ADR's explicit scope, with future general-3D work a new decision?

## G6 Vendor STEP source retention and bounded preview repair

**Subsequent decision (2026-10-19):** [ADR-0017](../adr/0017-forgiving-vendor-previews-with-preserved-source.md)
replaces certification as a prerequisite for preview admission with forgiving repaired
previews, preserved originals and explicit unknown collision accuracy. The historical
findings below describe the earlier policy, not the newly accepted direction.

**Decision statement to recover:** retain immutable original vendor bytes,
source-face/occurrence identity, units and placements; preview repair is a bounded
derived representation, not permission to drop geometry or declare exact solids.

- **What:** [ImportedStep](../../cad/monstertruck/src/step/mod.rs) owns original bytes,
  original source, occurrence transforms, source-face tags, precision and preview
  data. `StepImportOptions` keeps selected repair budget and deflection separate.
  `validate_invariants` is a cheap consistency check, not proof of every repair
  route's movement. [007 current work](../plans/cad-backend-replacement/issues/007-strict-step-import.md#current-work--2026-10-19)
  explicitly leaves counters/owner routes and packages open.
- **Named tests:** [step_import.rs](../../cad/monstertruck/tests/step_import.rs),
  `independent_tetrahedron_retains_exact_geometry_original_bytes_and_mm_mesh`,
  `absent_units_mean_mm_and_metre_coordinates_convert_once`,
  `dropped_face_or_unreferenced_solid_cannot_succeed_with_partial_mesh`, and
  `shared_exact_solid_keeps_both_rotated_and_translated_occurrences`.
- **Why recovered:** the [CAD destination and user scope](../plans/cad-backend-replacement/map.md#destination)
  explicitly separates exact generated output from vendor preview/alignment/collision
  and authorizes default `max(2 × authenticated declared precision, 0.02 mm)` or an
  explicit per-file preview budget, retaining declared precision, selected budget
  and actual movement separately. This is a recorded policy, not inferred from tests.
- **Limits/alternatives:** no primary rationale for the exact numerical default was
  recovered beyond its recorded authorization. [007 closure rules](../plans/cad-backend-replacement/issues/007-strict-step-import.md#closure-rules-shared-by-every-package)
  require final routes to terminate as identity, bounded repair or refusal, but
  identify incomplete source-selection/master/surface coverage. Do not claim that
  final strict policy is fully landed. Unknown analysis is not automatically a
  permissible unproved geometric repair under that future strict policy.
- **Coverage/disposition:** missing. Write delivered source/preview ownership now;
  label unfinished strict enforcement and its acceptance separately. **Questions:**
  what original evidence selected the 0.02 mm floor and doubling rule? Should final
  universal repair certification remain future policy rather than accepted-current
  implementation in the ADR?

## G7 Collision conclusions require owned qualified evidence

**Decision statement to recover:** a raw triangle-surface result is not a
source-geometry clearance/containment certificate. Supported conclusions require
owned final-mesh evidence, source/query/contact budgets and current operand binding;
unsupported or insufficient accuracy stays Unknown.

- **What:** [PreparedAnalysisMesh](../../cad/monstertruck/src/qualification.rs) seals
  emission/catalogues/settings and source proof behind private fields; callers
  cannot construct it from their own report. `verify_analysis_pair` consumes these
  owners. [Raw collision adapter](../../cad/wasm/src/model/collision.rs) keeps surface
  observations/witness provenance and rejects degenerate input. Complete execution
  belongs to Runtime, not the Case panel ([architecture](../architecture.md), H9).
- **Named tests:** [qualification.rs](../../cad/monstertruck/src/qualification.rs),
  `cap_preflight_refuses_triangle_budget_before_exact_work`,
  `cap_preflight_refuses_exponent_budget_before_exact_work`, and
  `real_preparations_bind_shared_activation_plate_vendor_and_mixed_pairs`;
  [runtime_collision_browser_tests.rs](../../web/crates/runtime/src/runtime_collision_browser_tests.rs),
  `complete_check_shares_one_vendor_refinement_across_repeated_placements` and
  `obsolete_false_view_entry_cannot_retire_an_active_complete_check`.
- **Why recovered:** [014 user collision decision](../plans/cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md#comments)
  chooses supported tolerance bands by default, refinement on demand and explicit
  0.01 mm tighter requests. Its acceptance text distinguishes unsigned surface
  distance from solid containment and forbids fabricated clear results.
  The original [collision research](cad/vendor-collision-validation-2026-10-17.md#recommendation)
  explains the source/mesh/query/containment distinction and records subsequent user
  adoption; its old v8 blockers are snapshot evidence, not current qualification.
  [Final current section](../plans/cad-backend-replacement/issues/014-vendor-mesh-collision-validation.md#current--2026-10-18-rescope)
  accepts qualified classes while preserving unsupported/insufficient cases as Unknown.
- **Consequences:** selecting 0.01 mm meshing is not a certificate of 0.01 mm source
  accuracy; qualified separating slabs do not establish general solid containment
  (014 and qualification tests). The complete-check controlled worker tests validate
  identity/lifecycle, not the actual CAD provider's mathematics
  ([167](../plans/typed-core-edits/issues/167-guide-complete-collision-test-fixtures.md#acceptance-criteria)).
- **Coverage/disposition:** missing and high-risk to undo. Write one evidence-bound
  collision ADR with G6 references; keep finite proof recipes/resource counters in
  technical contracts rather than separate ADRs. **Question:** which currently
  qualified classes should the ADR name, and which belong only in a versioned
  qualification reference so implementation growth does not require ADR churn?

## G8 Multi-tab saves and conservative asset reclamation

**Decision statement to recover:** reject stale saves atomically against observed
content identity, preserve pending edits for recovery, and reclaim assets only when
browser coordination and all readable projects establish a safe startup.

- **What:** [StorageToken](../../web/crates/host/src/host/storage.rs) hashes normalized
  stored JSON; `persist_project` compares expected content within the same transaction
  that writes project/assets. Equal revision numbers do not authorize replacement.
  [Storage lifetime tests](../../web/crates/host/src/host/storage_lifetime_tests.rs)
  exercise deferred cleanup while another editor may need Undo assets.
- **Named tests:** `stale_tab_save_preserves_the_other_tabs_document`,
  `startup_reclaims_only_unused_assets_after_editors_close`,
  `startup_keeps_assets_when_a_legacy_window_is_present`, and
  `unreadable_project_aborts_cleanup_and_can_be_recovered_or_deleted` in that file.
- **Rationale status:** H6 explicitly records the user-facing safety behavior;
  test comments explain retaining Undo assets. The benefit of content tokens over
  revision-only conflict checks is demonstrated behavior. A historical selection
  between this protocol, exclusive single-writer admission, merge or overwrite was
  **not recovered**. Do not list those as rejected options.
- **Coverage/consequences:** G1 covers Session recovery, not cross-tab concurrency
  or garbage collection. ADR-0004 covers format migration, not permission to erase
  unreadable assets. Cleanup can defer rather than maximize immediate disk recovery
  (H6 and tests).
- **Disposition:** likely ADR-worthy because data loss is costly and conservative
  retention is surprising. Confirm intent; group save safety and asset retention
  under browser-local durability, with G1 referenced rather than repeated.
  **Question:** was optimistic conflict refusal deliberately selected over a
  single-writer model, and must cleanup always defer if any project is unreadable
  or editor coordination is unavailable?

## G9 Portable project archives are a Core trust boundary

**Decision statement to recover:** the portable project carries JSON and
content-addressed assets through a validated, bounded Core archive service, not an
arbitrary browser ZIP extraction path.

- **What:** [archive.rs](../../core/src/archive.rs), `dispatch`, `unpack`,
  `verify_references`, `project_entry_limit` and `read_bounded`, validates paths,
  duplicate entries, SHA-256 references and declared/actual byte budgets before
  returning project asset buffers. Current project limits are 128 MiB compressed,
  8 MiB project JSON, 64 MiB per asset entry, 256 MiB total and 256 entries.
  `PackFiles` generic artifact exports are a distinct path; project numeric limits
  are not universal ZIP-output limits.
- **Named tests:** `unpack_verifies_missing_assets_and_hashes_before_any_output`,
  `limits_apply_to_pack_and_unpack_including_total_and_compressed_bytes`,
  `actual_read_limits_stop_at_the_first_excess_byte`, and
  `generic_exports_preserve_paths_and_have_no_project_numeric_limits` in
  [archive.rs](../../core/src/archive.rs).
- **Rationale status:** H1 §6 assigns archive/domain validation to Rust and original
  asset/file effects to Host; H6 records the import cap. The historical rationale
  for this exact portable envelope and individual limits was **not recovered**.
  Integrity and bounded allocation benefits follow from code, not a recovered user
  choice among archive formats or cap values.
- **Coverage/consequences:** ADR-0004 already covers ordered migrations at archive/
  storage boundaries. New material would be portable asset ownership and boundary
  admission, not another schema-version ADR. Missing/mismatched assets reject the
  result rather than delivering partially trusted output (named test).
- **Disposition:** hold for a focused rationale question; maintain ordinary numeric
  limits in code/reference docs unless their deliberate trade-off is confirmed.
  **Question:** is a self-contained content-addressed archive a long-term portability
  decision, and what compatibility/workload evidence selected these limits?

## G10 Real native Runtime tests do not replace mounted WASM checks

**Decision statement to recover:** native panel tests drive real Session/Core and
memory persistence through the Runtime surface with shared hold/fail gate semantics;
mounted WASM tests remain necessary for presentation/browser behavior.

- **What:** despite its retained filename,
  [runtime_test_stub.rs](../../web/crates/runtime/src/runtime_test_stub.rs) contains
  real `Session`/`CoreEngine`, a read-only event log and memory saves. `run_core` and
  `run_save` consume [GateDriver](../../web/crates/runtime/src/gate_driver.rs).
  [Browser Runtime](../../web/crates/runtime/src/runtime.rs) has in-process test
  adapters; [check.py](../../scripts/check.py), `browser`, separately schedules
  mounted workspace/provider/page checks.
- **Named tests:** [edit_ticket.rs](../../web/crates/runtime/src/edit_ticket.rs),
  `native_runtime_settles_a_ticket_through_the_real_session_and_core`,
  `held_core_and_save_keep_a_ticket_pending_until_released` and
  `core_and_save_failures_settle_the_ticket_without_a_resolver`;
  [runtime.rs](../../web/crates/runtime/src/runtime.rs),
  `gated_core_reply_keeps_the_edit_pending_until_released` is the browser-side
  controlled adapter check, not proof of DOM interactions.
- **Why recovered:** H5's explicit Answer selects Option C, shared driver/gate
  vocabulary, deletes hand-settling/parallel Session loops and keeps renderer/CAD/
  worker effects unavailable in the native edit adapter. Documented alternatives
  are compiling the full browser Runtime natively or using only a separate native
  adapter. [Architecture](../architecture.md) explicitly retains mounted checks.
- **Coverage/disposition:** missing. This is a genuine testing architecture choice,
  not a requirement to give every test invariant an ADR. A small ADR is justified
  by the risk of “faster tests” reintroducing fake resolution or removing browser
  coverage. **Question:** should the same-surface/shared-gates plus mounted-browser
  split remain a mandatory architecture constraint for future workspace tests?

## G11 Static local-first Rust policy with worker execution

**Decision statement to recover:** keep maintained application policy in Rust in a
static local-first browser host; generated bootstrap/upstream runtimes are not a
second authored application. Run heavy provider work outside the page, and preserve
offline clients across deployment updates.

- **What:** [Architecture](../architecture.md) identifies static browser deployment,
  Core/CAD workers and Rust offline policy. [service_worker.rs](../../web/src/service_worker.rs),
  `install_policy`/`activate_policy`, caches the release manifest, waits for old
  clients before activation, then deletes only older caches in its scope prefix.
  [offline.rs](../../web/crates/host/src/offline.rs), `scope_cache_prefix`, separates
  root/subpath cache names.
- **Named tests:** `path_hex_encoding_separates_root_and_subpath_cache_names` and
  `validates_complete_normalized_manifest` in
  [offline.rs](../../web/crates/host/src/offline.rs);
  `test_offline_worker_waits_for_old_clients_before_activation` and
  `test_build_assembles_requested_routes_from_dirty_worktree` in
  [test-build-web.py](../../scripts/test-build-web.py). These latter checks inspect
  source/package assembly; they do not prove real offline update behavior.
- **Why recovered:** H1 §§1/3/7 explicitly contrasts UI-only Rust, server execution,
  on-page engines and native hosts against local/offline/browser ownership, while
  allowing generated bindings and upstream runtime code. H7 and the install-policy
  comment explain keeping old clients' lazy offline assets usable. H2 retires the
  React comparison/rollback machinery, not local-first operation.
- **Coverage/consequences:** ADR-0003 covers Rust footprint generation only. Do not
  import H1's obsolete Cadrum allowance or proposed milestone browser-support matrix.
  Delayed activation trades immediate update adoption for live-client continuity
  (H7/current comment). Root/subpath isolation is an operational consequence, not a
  separate ADR per cache helper.
- **Disposition:** one platform/policy ADR after confirming current support claims.
  **Question:** is static browser/local-first still a product constraint excluding
  required server execution, and what browser support matrix is actually promised
  now rather than merely exercised by Chromium test commands?

## G12 Mounted source ownership versus embedded host circuitry

**Decision statement to recover:** a physical mounted daughterboard retains its
source-owned circuitry; an embedded circuit copy receives independent host IDs/nets
and explicit electrical joins. Visual presence is not fabrication qualification.

- **What:** [onboarding guide](../hardware/component-onboarding.md#onboarding-workflow)
  names those distinct ownership modes and separates imported, previewable and
  fabrication-qualified claims. [Mounted-module tests](../../core/tests/mounted_modules.rs)
  and [electrical circuit tests](../../core/tests/module_circuit_electrical.rs)
  exercise independent copies, preserved provenance and explicit host signal joins.
- **Named tests:** `two_embedded_circuits_own_their_definitions_and_internal_nets_and_undo_atomically`,
  `project_module_profiles_are_editable_and_undoable_without_rewriting_provenance`,
  and `removing_unknown_assembly_gate_requires_reviewed_geometry` in
  [mounted_modules.rs](../../core/tests/mounted_modules.rs);
  `embedded_circuit_requires_explicit_host_signal_and_matching_power_rail_joins` in
  [module_circuit_electrical.rs](../../core/tests/module_circuit_electrical.rs).
- **Rationale status:** H11 records the convention, and tests establish consequences,
  but a specific historical comparison explaining why these two modes were selected
  was **not recovered**. Avoid inferring hardware intent from names or a preview.
- **Coverage/consequences:** ADR-0010 covers mirrored board instances, module and
  embedded-circuit reflection, including `reuse-board` versus `new-board`. It does
  not fully explain source-owned mounted copper versus host-owned copied circuitry.
  ADR-0003/0009 already cover generator execution and parameter admission. Missing
  hardware facts must not be converted into unconditional manufacturing readiness
  (guide and gate tests).
- **Disposition:** likely costly/surprising enough for one future hardware-ownership
  ADR, but first interview the owner. Do not write an ADR per catalogue component.
  **Question:** were mounted/embedded modes consciously chosen as the durable
  library contract, and which source edits must be forbidden versus expressed as
  explicit new variants? Separately, is board-design versus physical-instance
  identity a standalone decision beyond the existing mirrored-half coverage?

## Existing ADR coverage map

All rows are accepted records in the [current index](../adr/README.md); this map
describes their text, not proof that every accepted requirement is implemented.

| Existing ADR | Covered decision | Gap boundary / disposition |
| --- | --- | --- |
| [0001 fixed outlines](../adr/0001-fixed-edited-outlines.md) | Manual outline copies stay fixed; located blockers gate dependent output, not saving/editing | No new “manual geometry ownership” ADR; G5 references it |
| [0002 linked refinements](../adr/0002-linked-outline-refinements.md) | Linked recalculation, source removal, explicit Freeze, persistence ownership | Do not split per outline action |
| [0003 Rust generators](../adr/0003-rust-footprint-generators-in-core.md) | Core-native Rust generator execution; remove JS/preview-worker duplication; manual upstream ports/licensing | Already covered; universal footprint redesign remains proposal, not a new accepted gap |
| [0004 format versions](../adr/0004-document-format-version-and-id-rename.md) | Ordered boundary migrations, identifier rename and older-build refusal; later format amendments | G9 concerns archive ownership/integrity, not another migration ADR |
| [0005 execution-time intent](../adr/0005-resolve-queued-edits-at-execution.md) | Queue resolution, fields versus one-shots, preview-only direct event, silent retirement, strict gesture/export/remap routes | Already covers these choices; G1/G3/G4 add durability/lifetime, not competing intent semantics |
| [0006 replacement exception](../adr/0006-replace-document-for-import-and-recovery.md) | Typed Core edit direction and named replacement exceptions; explicitly not globally enforced | Do not call it a completed prohibition or duplicate it as “Core owns every edit” |
| [0007 matrix placement](../adr/0007-matrix-gestures-edit-named-parameters.md) | Named parameters, Core axes and companion precedence, geometry-preserving conversion | Application of ownership; no per-gesture ADR |
| [0008 linked halves](../adr/0008-linked-halves-stay-linked.md) | Reflected defaults, explicit per-item exceptions, source-derived second half | Covered; current algorithms need no separate rationale invention |
| [0009 generator admission](../adr/0009-generator-parameters-checked-in-core.md) | Open/replacement warnings versus refusing newly written invalid parameters | Trust-boundary asymmetry already documented |
| [0010 Core-maintained reflection](../adr/0010-mirrored-half-is-a-core-maintained-reflection.md) | Hybrid storage/reconciliation, ownership frames, explicit exceptions, topology-atomic mirror creation and amendments | Covers much hardware-instance/mirroring territory; G12 must not duplicate it |
| [0011 part-axes offset](../adr/0011-part-offset-is-its-own-constraint-kind.md) | New persisted tag rather than silently ignored property; old Offset meaning retained | Compatibility rationale already recovered |
| [0012 bounded outline validation](../adr/0012-bounded-outline-validation.md) | Exact fabrication-grid contacts and per-persisted-path budget | No new ADR for each parser/geometry resource cap |

### Removed ADR: separate living rules from obsolete migration scaffolding

H1 is evidence, **not an ADR to undelete**. The [index note](../adr/README.md#notes)
records removal and number reassignment; new records must not reuse 0003.

| H1 material | Treatment |
| --- | --- |
| Engine/Session/UI authority; save acknowledgment/retry; scope identity; separate preview/export ownership | Reconcile into G1–G4 using current source |
| Rust-authored policy, static local-first host, off-page worker execution | Reconcile into G11; exclude obsolete kernel pins and unproven platform promises |
| Rust generator migration | Already replaced by current ADR-0003 |
| React reference frontends, separate trial origins/namespaces, P0–P3 milestones, cutover rollback/acceptance machinery | Historical only; H2 deliberately removes these |
| One web package rather than workspace crates | Superseded by the documented crate split/H10; do not reinstate |
| Old shared-scene/private-renderer integration proposals, cache-delta implementation particulars, exact Dioxus patch-selection proposal | Not established as current decisions by this inventory; require separate current-source tracing if needed |

## Exclusions and merges

- **Workspace dependency layering:** [architecture](../architecture.md) and current
  [Layout](../../web/crates/layout/Cargo.toml), [Runtime](../../web/crates/runtime/Cargo.toml)
  and [Host](../../web/crates/host/Cargo.toml) manifests establish current edges;
  H10 explains extraction. Manageable workspace size and rebuild consequences are
  documented, but no source recovered a choice to make the exact edge list permanent.
  Keep the detailed graph in architecture; put only durable ownership constraints
  in G2. **Ordinary architecture detail, no standalone ADR recommended.**
- **PendingEdits/Signal helper lifecycle:** already an implementation of 0005,
  [module-deepening spec](../plans/module-deepening/spec.md) and G3. Latest observation
  per key is not a reason for an ADR per helper. [Architecture](../architecture.md)
  records draft preservation, rebinding and unmount responsibilities.
- **Collision Runtime ownership versus Case display:** merge with G7/G2 rather
  than documenting the fixture, retry helper and every cancellation batch as new
  decisions (H9; [167](../plans/typed-core-edits/issues/167-guide-complete-collision-test-fixtures.md)).
- **Independent manufacturing reference checks:** part of G5/G7, not an ADR for each
  volume fixture or external tool invocation. The bounded [008](../plans/cad-backend-replacement/issues/008-step-export-independent-accuracy.md)
  evidence explicitly does not establish a general STEP validator.
- **Universal generator templates/custom authoring:** [investigation](../investigations/universal-footprint-generation.md)
  explicitly labels the redesign proposed and records the requirement for all
  built-ins to use one parametric engine. That requirement is not evidence that
  such an engine landed; the [docs index](../README.md#investigations) also treats it
  as an investigation. **Proposed design/not established as landed**; do not promote
  its implementation status while filling historical ADR gaps.
- **CAD final universal repair terminal policy:** retain as future/partially
  implemented policy within G6, not a claim of universal current enforcement;
  [007 current work](../plans/cad-backend-replacement/issues/007-strict-step-import.md#current-work--2026-10-19)
  names unfinished work. Ticket role labels alone cannot resolve completion.
- **Build-speed knobs, CSS tokens, exact crate file sizes, retained helper names,
  test launch counts:** operational/current implementation details unless a specific
  lasting trade-off is independently recovered. The [check-speed map](../plans/check-speed/map.md)
  is evidence for its optimization effort, not authorization to weaken G10's coverage.
- **Not exhaustively traced:** firmware export policy, user script execution/security,
  all logical-versus-physical topology rules, catalogue qualification completeness,
  renderer scene/camera internals and every storage recovery UI path. No “no gaps”
  conclusion is made for these areas.

## Suggested first writing batch and exact questions

Start with **three records**, not a mass ADR-generation pass:

1. **Durable publication and recovery (G1):** strongest original rationale and exact
   failure/retry tests. Ask: “Do we reaffirm save-before-publication, blocked dependent
   work after save failure, retry of the retained snapshot only, and explicit choice
   after an unknown executor outcome?”
2. **Core/Session/browser/presentation authority (G2), with the essential G3 identity
   rule:** recover the living ownership contract, not migration instructions. Ask:
   “Are these owners and captured-lineage requirements enduring constraints, while
   browser-handle extraction and exact workspace edges remain implementation details?”
3. **Exact generated CAD and mesh-only output scope (G5):** newer explicit user
   decisions with a clear trade-off and current adoption. Ask: “Is layered exact
   case/PCB output, mesh keycaps/vendor previews and the named unsupported refusals
   the accepted scope to preserve, without a general Boolean/STEP validator promise?”

Next take G4 and G7, then G6 with its unfinished-policy boundary. G10/G11 have
recoverable rationale but are less urgent than save/geometry correctness. G8/G9/G12
need the targeted rationale questions above before an accepted ADR is drafted.
These are recommendations, not newly accepted choices.

## Research checks

- Read the full report back for unsupported intent claims; kept inference/unknown
  rationale distinct from explicit decisions, and removed obsolete trial rules from
  suggested current contracts.
- Verified local Markdown targets/anchors and cited test symbol names against the
  current files: 118 local links/anchors, 45 distinct named tests, 16 commit
  references and the removed ADR's Git object all resolve.
- Checked the new report's whitespace/diff and that HEAD remained the examined
  revision. Only this report was intentionally written; concurrent changes were
  left intact.
- **Not run:** builds, tests, browser checks, mutation checks, benchmarks or external
  geometry tools. Existing passing results in linked tickets are historical reports,
  not fresh verification. No ADR, index, implementation or commit was created.
