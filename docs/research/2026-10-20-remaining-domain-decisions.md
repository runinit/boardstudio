# Remaining domain decisions: finite conversation inventory

**2026-10-20 — research only; no decisions accepted.** HEAD at inspection was
`a9d0652982c0c4a8d7449b8a7221c4386567dac5`. The checkout contains extensive concurrent
tracked/untracked work, including authoritative ADRs 0013–0033 and the glossary.
This is **not an atomic source snapshot**. Only this report is an authored addition.

**Subsequent camera decision (2026-10-20):**
[ADR-0014](../adr/0014-document-authority-and-captured-lifetimes.md#amendment-preview-recovery-respects-automatic-preview-2026-10-20)
now requires successful same-project/view recovery to restore the last usable
viewpoint when available, with a fitted overview as fallback. Ordinary workspace
departure/return within the same editor session also restores that matching scoped
viewpoint. The user chose session-only retention, without a persistence requirement
for project/browser reopening. The camera questions below are settled; this is not
implementation or GPU-recovery evidence.

**Subsequent input decision (2026-10-20):**
[ADR-0032's amendment](../adr/0032-recreated-inputs-start-unassigned.md#amendment-temporary-disable-retains-bindings-2026-10-20)
settles temporary disable/re-enable: resume the same input's prior bindings even
across save/reopen, without making it active while disabled. This does not promise
retention of all Part-owned settings/wiring or establish runtime acceptance.

## 1. Inventory before more interviews

**Conversation queue completed (2026-10-20):** the user chose the new matrix's
Objects entry as keyboard-focus destination after saved creation succeeds and the
form closes. This ordinary UX rule is recorded in the
[workflow spec](../plans/application-workflow-verification/issues/001-verify-six-application-workflows.md),
not a new ADR. With the camera, temporary-input and notice choices above/below
recorded, the four researched groups are settled. Transfer to bounded implementation
and verification; this does not establish that any pending behavior is delivered.

**Subsequent notice decision (2026-10-20):**
[ADR-0033's amendment](../adr/0033-include-license-information-in-exported-packages.md#amendment-modification-notes-are-optional-2026-10-20)
makes user-authored modification notes optional, retaining supplied notes and known
applicable notices without inventing missing history. Its later nested-package
amendment makes the inner KiCad ZIP independently forwardable with its own root
`license.md`, in addition to the outer handoff's clearly scoped notice. Those
questions below are settled. Unknown terms also no longer leave admission unchosen:
the user chose export with an explicit licence-unknown notice, without mandatory
acknowledgement or blocking solely for unavailable terms. Known terms and other
integrity checks remain. These are policy decisions, not implementation evidence.

Read [root instructions](../../AGENTS.md), [domain instructions](../agents/domain.md),
[glossary](../../GLOSSARY.md), [ADR index](../adr/README.md) and relevant current
records through 0033, the installed domain-modeling skill and both format guides,
the [workflow map](../plans/application-workflow-verification/map.md) and its
[48-story parent specification](../plans/application-workflow-verification/issues/001-verify-six-application-workflows.md).
Reconciled the four prior reports:
[workflow evidence](2026-10-20-application-workflow-evidence.md),
[compatibility/identity/provenance](2026-10-20-compatibility-identity-provenance-deep-dive.md),
[blind spots](2026-10-20-adr-blind-spots-and-sustainability.md) and
[coverage/candidates](2026-10-20-adr-coverage-and-candidates.md).
Their older candidate lists are not today's decision queue.

| Boundary | Current disposition; do not ask again |
| --- | --- |
| Renderer recovery mode | **Settled policy / implementation and evidence work:** [0014 amendment](../adr/0014-document-authority-and-captured-lifetimes.md#amendment-preview-recovery-respects-automatic-preview-2026-10-20): explicit error, forms/2D usable, current accepted design/current owner, no project reload/edit replay. Existing affected-view automatic preview ON permits one automatic recovery attempt; OFF waits for Retry; failed recovery requires explicit Retry, without a rerender-driven loop. Camera alone remains open. |
| Generator changes | **Settled:** [0003 amendment](../adr/0003-rust-footprint-generators-in-core.md#amendment-existing-projects-use-current-generators-2026-10-20) rejects frozen old geometry/per-project opt-in upgrades. Current compiled generators/defaults apply; schema, parameter, identity/terminal and protected-handoff integrity still apply. |
| Deleted/shrunk inputs returning | **Settled / implementation debt:** [0032](../adr/0032-recreated-inputs-start-unassigned.md) requires unassigned replacements, including save/reopen and reused IDs. Compacted survivors and actual Undo are distinct. Temporary disable is expressly unresolved. |
| Package notices | **Settled / implementation debt:** [0033](../adr/0033-include-license-information-in-exported-packages.md) requires root `license.md`, known applicable notices/source references, honest unknowns, bounded project import and information retention for re-export. Neither blanket relicensing nor a blanket unknown-term export ban is selected. |
| Firmware and handoff | **Settled:** [0030](../adr/0030-export-firmware-as-an-editable-repository-template.md) means editable repository template, external compilation, disclosed unsupported optional peripherals if the base keyboard remains valid. [0031](../adr/0031-preserve-pcb-handoff-protection-across-undo.md) protection follows successful packaging/current ownership/**saved** protection before delivery, survives Undo and later download failure/cancellation. |
| Hardware topology | **Settled / delivery debt:** [0024–0028](../adr/README.md) cover shared routing with actual assembly/jumpers, exclusive removed-half design/accessory deletion with Undo, either side central, incomplete designs saveable, explicit reversible conversion without merging routing. |
| Vendor geometry/collision | **Settled / existing work:** [0017](../adr/0017-forgiving-vendor-previews-with-preserved-source.md)/[0018](../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md) admit useful repaired/partial previews and approximate observations with explicit limitations; unsupported accuracy and whole-model clearance remain uncertain. |
| Scripting | **Excluded:** [0029](../adr/0029-constrain-geometry-scripts-and-apply-results-atomically.md#scope-correction-2026-10-20) deprecates the feature premise. Retained code safety is not intended workflow; removal/compatibility is later scoped work. |
| Release/GPU/hardware confidence | **Operational evidence, not an invented policy vacancy:** source-bound A/B artifacts and authorized disposable origin, genuine supported context loss, target toolchain, downstream routing/DRC and qualified bench/fit resources remain pending under the [spec prerequisites](../plans/application-workflow-verification/issues/001-verify-six-application-workflows.md#operational-and-human-prerequisites). No handshake or all-hardware qualification policy follows from missing runs. |

**Deduplicated queue: four grouped areas, supported by four evidence sets below.**
Camera belongs with 0014, temporary-input identity with 0032, and notice boundaries
with 0033; focus is ordinary UX/spec clarification. No standalone new ADR is
currently justified. Offer one only if an explicit answer establishes meaningful
reversal cost, surprising behavior and a genuine trade-off.

Existing [Editor 011](../plans/editor-workspace-extraction/issues/011-shared-camera-and-surface-coordination.md)
owns behavior-preserving camera/surface extraction, not a persistence redesign.
[Workflow 001](../plans/application-workflow-verification/map.md) is a ready-for-agent
**spec**, with approved S-A real application/mounted, S-B source-bound page and S-C
actual-package seams; its children are unpublished. Reuse existing Editor work,
[CAD browser harness 016](../plans/cad-deepening/issues/016-rust-browser-harness.md),
[deploy gate 06](../plans/cleanup/issues/06-gate-pages-deploy-on-checks.md) and
[forgiving-preview successors](../plans/cad-backend-replacement/spec.md), not duplicate
tickets. Publication is not delivery. Stale historical links are not this domain loop.

## 2. Priority 1 — camera after preview recovery

### Evidence set C: distinct state and lifetime boundaries

| Transition | Inspected behavior, not a promised outcome |
| --- | --- |
| Regeneration within the same live 3D viewer | [SharedViewer projections](../../web/crates/case/src/shared_viewer.rs#L826-L1129) send `keepCamera: true`; `project_parts_preview` does likewise. [Renderer `accept_scene`](../../renderer/src/wasm.rs#L1015-L1047) updates bounds but retains its live camera when requested. Explicit Fit resets target, distance, yaw and pitch. |
| Workspace navigation / 2D→3D remount | SharedViewer owns its Host locally and disposes it on drop ([lifetime](../../web/crates/case/src/shared_viewer.rs#L1622-L1748)). [Host `attach`](../../web/crates/host/src/renderer_host.rs#L78-L124) always fits a newly constructed renderer. Retaining a mode flag at Editor scope does not retain the former renderer's viewpoint. Production routing must establish whether a particular transition actually remounts. |
| Renderer failure / recovery | Existing fail-stop guards reject camera calls; new reconstruction would start with the mount/fit path absent additional camera transfer. This predicts a reset, not implementation of the newly accepted recovery amendment. |
| Project reopen / fresh browser Runtime | [Session camera](../../application/src/session.rs#L107-L142) is 2D `center: Vec2` and `zoom`; [SetCamera](../../application/src/session.rs#L1096-L1113) changes the read model, not a saved document. Successful [Open publication](../../application/src/session.rs#L1947-L2005) clears selection/instance, not camera. A [fresh Runtime](../../web/crates/runtime/src/runtime.rs#L386-L499) creates a fresh Session before restoring the saved project. Same-Runtime reopen is therefore not browser-session persistence. |

The requested name `sharedCameraBasis` is **not a current Rust symbol found in this
checkout**. Its relevant current counterparts are `CameraBasis` (bounds/surface for
finding navigation), Session's 2D camera and Editor's workspace-specific fit basis
([types](../../web/crates/keycaps/src/keycaps_fit.rs#L60-L64),
[Editor](../../web/src/presentation.rs#L4640-L4726)). These are not a serialized 3D
viewpoint. [ProjectDoc](../../core/src/model.rs#L903-L970) has no dedicated camera
record; the renderer separately owns target/yaw/pitch/distance and GPU objects
([fields](../../renderer/src/wasm.rs#L375-L399)). Do not promise restoration of every
GPU field or add preferences merely to answer a user-visible scope question.

**Test boundaries:** inspected `layout_fit_basis_keeps_react_padding_and_source_surface_aspect`
in [keycaps_fit](../../web/crates/keycaps/src/keycaps_fit.rs#L1290-L1340) asserts 2D
bounds/aspect math. `context_loss_settles_callers_stops_rendering_and_unmount_releases_everything`
in [Host](../../web/crates/host/src/renderer_host.rs#L1083-L1153) uses a scripted
renderer and dispatched DOM event, checking refusal/cleanup, not restored camera.
`parts_renderer_initialization_failure_keeps_form_and_returns_to_2d` in
[Parts](../../web/crates/parts/src/parts/preview.rs#L1265-L1354) injects initialization
failure and seeded geometry; it checks forms/2D and unchanged accepted document,
not successful retry or real GPU loss. None was run here.

**First scenario/question for the parent:** “You orbit and zoom a Case preview,
then its renderer fails. Recovery succeeds in that same project/view using the
current accepted design. Should your last viewpoint return, or is a fitted overview
acceptable?” Present only this scenario first; recovery timing is already decided.

If the answer asks for retention, clarify ordinary workspace departure/remount **next**,
then reopen/new-session scope only if needed. These are separate transitions, not
one implicit ‘persist camera everywhere’ promise. Record a narrow 0014 amendment
if the trade-off warrants it; update workflow 001's camera oracle and coordinate
Editor 011 without changing its extraction scope silently.

## 3. Priority 2 — temporary disable is not deletion

### Evidence set I: what remains and what disappears

The real [selected-key actions](../../web/src/presentation/layout_key_selection_actions.rs#L335-L386)
distinguish Enable/Disable (`SetMatrixCellsEnabled`) from Delete
(`RemoveSelectedMatrixCells`). [edit_selected_cells](../../core/src/matrix/deletion.rs#L470-L544)
refuses deleted cells, changes `enabled`, retains other cell fields and calls
`set_matrix`; it does not set `deleted` for a temporary disable.

For an **ordinary unpaired matrix**, inspected
[set_matrix](../../core/src/matrix.rs#L714-L998) establishes:

- The cell's definition/variant/offset/rotation/assembly recipe remains. A disabled
  cell creates no Part or companions. Parts, Board membership, net pins, constraints
  and explicit outline-envelope membership are removed, not merely hidden.
- Typed layer bindings/sensors, legacy Board key bindings and document keycap maps
  are not pruned on this setter route. The [common edit cleanup](../../core/src/lib.rs#L620-L663)
  and `prune_removed_components` prune outline dependencies, not those binding maps.
- Re-enable regenerates coordinate IDs and recipe geometry. It does **not** recover
  removed Part-owned keycap size, generator overrides, locks or reference identity
  from a dormant Part archive; a new Part gets defaults. Deleted net-pin/constraint
  records are not automatically restored. Retained keycap-map values are different
  from `Part.keycap`.
- [Electrical discovery](../../core/src/electrical.rs#L206-L295) skips disabled cells
  and requires actual Board-owned Parts. [Firmware adaptation](../../web/crates/runtime/src/firmware_request_adapter.rs#L90-L171)
  emits current assignments/auxiliary inputs, not every retained binding-map entry.
  On re-enable, if planning succeeds, the reused ID can again retrieve its retained
  legacy binding; [typed source generation](../../core/src/keymap.rs#L472-L500) looks
  up the same current key IDs. A dormant binding is not an active firmware key.

This is **source-derived partial retention**, not a full ‘temporarily hide everything
and restore it’ implementation. Native mirrored reconciliation has additional
membership/exception cleanup; do not generalize this unpaired outcome to every
linked/Local/Omitted case ([prior bounded trace](2026-10-20-compatibility-identity-provenance-deep-dive.md#b4-suffixes-mirrors-legends-and-queued-ownership)).

**Inspected tests:** [Core](../../core/tests/core.rs#L2649-L2820)
`matrix_shrink_discards_out_of_bounds_edits` asserts structural cleanup;
`matrix_cells_preserve_survivors_and_companions` asserts disabled absence and
surviving Part identity during growth, not disable→enable binding restoration.
[core_disable_then_undo_restores_scoped_key_selection](../../web/crates/layout/src/matrix_transform_lifecycle.rs#L340-L469)
tests actual Undo and selection retention, not re-enable or saved bindings.
Missing oracle: bind typed/legacy values → disable → serialize/fresh Open → enable
→ inspect emitted order/bindings, separately checking Part-owned data and a paired
case. No new test execution is claimed.

**One question:** “A key bound to A is temporarily disabled, saved/reopened, then
enabled without Undo. Should it resume A as the same input, or start unassigned as
a replacement?” Both are still open under 0032. Ask about appearance/wiring retention
separately only if ‘temporary’ is meant to preserve those too; current behavior does
not justify that wider promise. Amend 0032, and add a glossary term for a temporarily
disabled input only if resolved. Do not reopen deleted/shrink-regrown replacements,
compacted survivors or Undo.

## 4. Priority 3 — notices: lineage, distribution scope, unknown terms

### Evidence set N: available facts versus product choices

[PartDefinition/PartGenerator](../../core/src/model.rs#L135-L214) retain source text,
profiles and parameters, not a universal custom-author/change-history record.
[Asset](../../core/src/model.rs#L644-L654) has optional licence/source strings;
[HardwareSource](../../core/src/hardware.rs#L4-L52) can retain repository, revision,
path, licence and optional hash. The
[marbastlib manifest](../../catalogue/parts/import-manifest.json#L4-L15) already
supplies a pin, path, hash and licence, while runtime
[ImportedPart](../../web/crates/catalogue/src/catalogue.rs#L76-L79) decodes only its
definition. Missing propagation of known facts is implementation debt, not a user
decision about whether to include them.

Likewise, [generator NOTICE](../../footprints/NOTICE.md),
[KiSwitch](../../ergogen/library/manifest/kiswitch.json) and
[KiCad manifests](../../ergogen/library/manifest/kicad.json) contain auditable
distribution facts. Generator terms are not automatically model terms. Infused-Kim's
[README](../../ergogen/library/vendor/infused-kim/README.md#L158-L172) and
[LICENSE](../../ergogen/library/vendor/infused-kim/LICENSE#L1) literally disagree;
preserve that conflict rather than adjudicating it. External build instructions or
template dependency pins are not licence grants.

[prepare_archive](../../web/crates/runtime/src/portable_archive.rs#L10-L96) clones
existing asset metadata but creates bundled asset records with no licence.
[Core admission](../../core/src/archive.rs#L44-L121) and
[entry allowlist](../../core/src/archive.rs#L216-L240) still lack notice support and
retention. [build_handoff](../../web/crates/runtime/src/pcb_handoff.rs#L65-L218)
puts board/models into an inner ZIP and that ZIP plus assembly/wiring files into
the outer package; [model loading](../../web/crates/runtime/src/pcb_handoff.rs#L220-L275)
copies hash-checked bytes. This structure proves neither that users forward the
inner archive alone nor that an outer notice suffices for that workflow.

**Inspected tests:** `pack_round_trip_keeps_local_bytes_and_optionally_adds_opaque_model_bytes`
in [portable_archive](../../web/crates/runtime/src/portable_archive.rs#L374-L503)
checks metadata/byte precedence, with opaque payloads, not notices.
`unpack_verifies_missing_assets_and_hashes_before_any_output` in
[archive](../../core/src/archive.rs#L594-L616) supplies integrity coverage to retain.
`native_board_export_preserves_untouched_source_spans_and_kicad_10_parses_and_plots_both_sides`
in [KiCad integration](../../core/tests/kicad_integration.rs#L774-L830) preserves a
synthetic Vendor note, not custom legal attribution. No mixed package was generated.
The prior reports' dated U1/header observations remain bounded historical evidence.

**Sequential prompts, not a batch:**

1. A user modifies a source-backed footprint. Beyond preserving known/supplied
   information, must BoardStudio collect a short user-supplied modification note,
   or is identifying unavailable customization history honestly sufficient? This
   is the remaining capture promise, not permission to invent authorship.
2. Only if needed for the delivery story: is forwarding the inner KiCad ZIP alone
   an intended supported handoff, or is the outer package the required delivery?
   For any independently distributed package, root-notice inclusion is already
   settled; subset coverage/outer-summary layout is ordinary presentation work.
3. One locally supplied model has no supplied distribution terms. Should the
   affected package proceed with explicit unknown information, require an explicit
   acknowledgement, or wait for supplied information/removal of that content?
   This is product admission, not a legal verdict or a currently accepted blanket ban.

Prefer a 0033 amendment for durable capture/admission boundaries, ordinary spec
examples for notice layout. Known-notice propagation and bounded import/re-export
can proceed without a universal provenance registry or reconstructing old history.

## 5. Priority 4 — one actual post-action focus destination

### Evidence set F: preserve established behavior first

[MatrixSetup](../../web/crates/layout/src/objects/matrix_setup.rs#L164-L205) captures
the opener, focuses Rows on mount, and on Cancel restores the connected opener or
guide heading. Both named
[matrix_cancel focus tests](../../web/crates/layout/src/objects/matrix_setup_focus_tests.rs#L88-L160)
assert this using programmatic focus/click, not trusted keyboard traversal.
[Shared-control tests](../../web/crates/ui-shared/src/controls.rs#L675-L831) cover
associated number errors and popover return-to-trigger versus preserving outside
focus. These are not unresolved universal focus policies.

The actual [setup settlement](../../web/crates/layout/src/objects/matrix_setup_controller.rs#L525-L581)
closes the form and selects the saved matrix; failure retains an error. It names
no success focus destination. The native
[current_saved_completion_selects_the_created_matrix_exactly_once](../../web/tests/support/matrix_setup_presentation.rs#L228-L277)
asserts saved selection through controlled ports, not browser focus. Selection is
not focus. The earlier BODY observation came from an older source-unbound package,
not a fresh current-source regression ([dated limits](2026-10-20-application-workflow-evidence.md#executed-partial-journey-x3)).

**One narrow question:** “After Create matrix succeeds and its form disappears,
should keyboard focus go to the new matrix's Objects entry, its Inspector, or the
Layout canvas?” Confirm a concrete reachable destination in current mounted
composition before making it a test oracle. Cancel is already defined; do not
re-decide it. Other vanished-target/recovery destinations need interviews only if
characterization exposes an actual ambiguity. Record ordinary UX/spec acceptance,
not one ADR per focus rule.

## 6. First conversation and stopping rule

Start with **Priority 1's same-view failure/recovery viewpoint question**. Capture
the explicit answer before advancing; it can narrowly amend 0014 and the workflow
spec, without a new camera framework. Then resolve temporary-disable identity,
the genuinely required notice subcases, and the single focus destination.

Stop when each relevant scenario has either an explicit user outcome or an explicit
deferral that prevents false acceptance. Transfer known-policy gaps to bounded
implementation/verification work through approved S-A/S-B/S-C seams and existing
owners. Missing A/B artifacts, genuine GPU access, firmware tools, bench hardware or
legal interpretation are finite resource/evidence gates, not prompts for more ADRs.
Reopen research only for a specific contradicted fact or newly encountered boundary.

**Verification/limits:** CodeGraph preceded source inspection; omitted source/test
ranges were read directly. References identify inspected symbols/test bodies, never
fresh passing results. Local links/anchors and named references were checked. No
builds, tests, browser runs, dependencies, commits, worktrees, child agents, source
copies/manifests or changes to ADR/glossary/spec/tickets were made. Concurrent work
was preserved; refresh relevant files before implementation.
