# Proposal: composition proof first; do not duplicate workspace migrations

Status: composition-first scope and documentation-only ticket-tree creation/validation/commit approved by the user on 2026-11-15; implementation is not authorized.

## Approved direction

Keep the original parent001 and its unresolved ownership goals visible, but do not migrate all12 implementation tickets as a newly confirmed build programme. The approved first phase is a bounded **production-composition proof epic**, using existing ticket002 without inventing a replacement identity. Reconcile domain-specific moves with the workbench adapters before scheduling them; defer remaining pure ownership relocation unless the user still wants that architecture outcome independently.

This is an approved sequencing/scope decision, not a claim that003-013 are done, dropped, obsolete or waived. The approved legacy graph still governs that legacy effort until a replan is approved. No original status, prerequisite, receipt or file changes here. There is no evidence from this research that source inspection alone justifies closing any original ticket.

## What the evidence changes

- The actual composition gap is concrete: production WorkbenchApp and existing Editor test hosts differ in outer subscription/providers and unmount predicate. See [report§1](research-editor-workspace-ownership.md#1-how-does-production-composition-work).
- Workspace state/render modules and operation-specific admission already exist. The residual extraction is principally controller/source/action assembly still in Editor—not reimplementing those feature controllers. See [report§2](research-editor-workspace-ownership.md#2-where-does-workspace-orchestration-live).
- Save-before-publication, one board-scoped PCB plan, post-Landed physical setup, placement owner checks and deferred finding currentness already have source implementations. Their preservation is required; recreating them as new features is not. See [report§3](research-editor-workspace-ownership.md#3-how-do-cross-workspace-workflows-preserve-ownership).
- Focused tests are reusable, but cannot be relabelled full production composition, actual pointer capture or worker/GPU qualification. See [report§4](research-editor-workspace-ownership.md#4-how-is-this-behavior-tested).

## Approved first epic: prove real production composition and lifecycle boundaries

**Outcome:** a maintainer can exercise the actual App composition with a supplied test Runtime and distinguish hidden Editor, Session Close, Editor-only removal and App removal without a second application implementation.

**Parent identity:** legacy001 remains the overarching specification and is not closed by this epic. This is a phased subset, not a faithful migration declaring the full original extraction finished. No new epic number or tracker identity is assigned by this proposal.

### Requirements and source

- R1: one private Runtime-injection seam reuses production subscription, reconciliation and providers; normal startup retains its real Runtime constructor. Source: [002, Scope/acceptance](../editor-workspace-extraction/issues/002-production-composition-test-seam.md); [001, Application composition seam](../editor-workspace-extraction/issues/001-editor-workspace-ownership.md#application-composition-seam).
- R2: a scope-correct mounted fixture drives a real edit, filtered reply/save gates, bounded waits and accepted/saved or view-state assertions without cross-scope/dropped-Signal warnings. Source: [002, Acceptance criteria](../editor-workspace-extraction/issues/002-production-composition-test-seam.md#acceptance-criteria).
- R3: workspace cycle and Export Back/Escape/header return follow actual production controls; view switching alone does not edit the document. Source: [002, Acceptance criteria](../editor-workspace-extraction/issues/002-production-composition-test-seam.md#acceptance-criteria).
- R4: pending leave/return and reused-ID project replacement cannot revive obsolete controls or retarget work; lower-seam late-reply evidence is explicitly named. Source: [002, Acceptance criteria](../editor-workspace-extraction/issues/002-production-composition-test-seam.md#acceptance-criteria); [001, Fixture and evidence requirements](../editor-workspace-extraction/issues/001-editor-workspace-ownership.md#fixture-and-evidence-requirements).
- R5: Library hiding preserves Editor, while actual Editor-only/App removal is separately exercised with held work released afterward and observable cleanup/no stale DOM, navigation, focus or dropped-Signal access. Source: [002, Acceptance criteria](../editor-workspace-extraction/issues/002-production-composition-test-seam.md#acceptance-criteria).
- R6: keep useful feature/native tests, isolate browser preferences/global DOM, keep fixture controls out of normal startup and record baseline/command/diagnostic/receipt scope. Source: [002, Verification](../editor-workspace-extraction/issues/002-production-composition-test-seam.md#verification); [map, Shared contract](../editor-workspace-extraction/map.md#shared-implementation-and-verification-contract).

### The actual next ticket

**002: Establish the real App composition test seam** — retain its existing identity and scope. It has no implementation prerequisite. Its ready label is unchanged, not a new instruction to start.

Build order inside002:

1. Record the unchanged-source baseline; expose the private production seam and prove scope-correct mount/effect driving.
2. Add the minimum real-composition characterization: one edit/gate, workspace cycle, production board reconciliation, Export return and reused-ID replacement. Reuse sufficient lower-seam oracles rather than repeating every field.
3. Exercise hidden-versus-removed owners and late allowed work. If safe driving/unmount cannot be shown, stop and report the blocking condition before ownership extraction.

These are internal steps of the existing ticket, not newly assigned leaf identities. If implementation sizing requires a formal split, propose child identities and each one's check before creating them; do not silently repurpose003.

### Done when

All R1-R6 and existing002 acceptance pass. The fixed proof map supplies genuine RED or named proof-gap mutation evidence, then fix→affected checks (including lint for WASM-only Rust)→boundary mutations→one fresh Spec+Standards review→002's integrated gate. Use its documented quick changed-scope checks, page-only WASM compilation, page-bin mounted suite and relevant Runtime tests; fix exact commands/test filters in the proof map before implementation. This research runs none of those implementation checks.

No historical mockup, handwritten DOM, synthetic generation or root-DOM removal counts as real composition/reconciliation/teardown. A controlled lower seam stays explicitly lower seam. This epic does not close001 or satisfy013's all-workspace ownership gate.

**Tracer decision approved (2026-11-15):**002's real edit→held save→workspace transition→accepted publication→genuine owner removal is the first end-to-end path. No fabricated HITL setup, framework or architecture spike. Additional refactor sweep remains declined; cleanup stays bounded to the ticket or original013 when that scope is actually completed.

## Disposition of all original identities

All original headers remain ready/build. The approved scheduling direction below is not a set of status edits or completed transfers; other plan owners still need to agree any transfer.

| Legacy identity | Current evidence / proposed scheduling disposition |
| --- | --- |
|001 parent specification|Preserve exact identity and full unresolved acceptance; obtain the scope decision below. Never infer closure from a child count.|
|002 production composition seam|Retain as the sole immediate build frontier, with all original gates.|
|003 Keymap workspace ownership|Partial modules exist; root controller/input assembly remains. Coordinate remaining moves with workbench011, preserving its M2 classification/approval gate and captured-owner draft rules. Do not schedule duplicate Keymap migrations.|
|004 PCB wiring and plan ownership|One retained board-plan owner already exists. Coordinate residual source/action/surface assembly with workbench010; preserve shared plan availability and instance normalization, not a new resolver/query.|
|005 Parts browsing/attachment/routes|Source state/rendering and attachment/placement guards exist. Coordinate residual Parts wiring with workbench014 and Layout005; retain actual accepted-route/stale-destination proofs.|
|006 Case settings/instance navigation|Substantial tree/display admission already module-owned; mechanical/settings and shared setup assembly remain. Coordinate with workbench013; do not absorb separately gated Preview015 or duplicate physical setup.|
|007 complete setup-guide workflow|Preferences/stages already modular; orchestration/drawer assembly remains. Defer pure relocation pending the ownership-goal decision; route/lifecycle regressions belong to their demonstrated owning slice, not a new guide framework.|
|008 Keycaps assessment/finding navigation|Projection/state/currentness mechanisms exist; root controller/action assembly remains. Coordinate with workbench012 and keep accepted default-relocation/classification gates. Do not duplicate deferred-fit owner logic.|
|009 Layout selection/Inspector|Coordinate residual mounts/admission/input assembly with Layout workbench003/004/005; avoid freezing obsolete dock presentation into a second migration.|
|010 Layout creation/placement|Placement machinery and post-Landed checks exist; root host/port assembly remains. Coordinate with Layout005/Parts014; no new placement controller or arbiter.|
|011 shared camera/surface|Pan/camera helpers already own part of lifecycle. Defer remaining pure wiring relocation pending scope decision; do not change formulas or add a camera store. Production cleanup evidence is distinct from workflow-level viewpoint/recovery acceptance.|
|012 Layout canvas/gestures|Inline transaction surface remains; broad extraction is not demonstrated necessary to fix a current functional failure. Defer that architectural move pending scope decision. Do not resurrect the separately removed pointer plan. Keep trusted-pointer/save/Undo obligations visible if this scope is retained.|
|013 integrated ownership/obsolete plumbing|Retain unresolved. Its original all-six-workspace gate cannot close on002 or the smaller proof epic. Run it only if/when the parent ownership outcome is retained and delivered; do not rebrand it as a generic refactor sweep.|

Workbench references are qualified identities in a different plan, not rewritten dependencies. No blanket prerequisite is imposed on its authorized M1 UI work. Any accepted transfer needs the owning thread's agreement and explicit requirement coverage; it is not effected here. The original Keymap-first graph is not silently rewritten.

## Authority conflicts to settle, not hide

The workbench's accepted new panel model supersedes old dock/pin/width expectations for migrated surfaces, but its later adapter phases and form classifications remain human-gated. Existing captured-owner commit rules and lifetime safety still apply. The older extraction cannot require keeping retired UI, and the newer UI cannot refresh expired admission to retain drafts. These are explicit cross-plan contracts, not inferred implementation completion. See [planning-context digest](digests/planning-context.md) and [workbench refactoring decisions](../toolbar-led-workbench/refactoring.md).

At the research snapshot, workflow verification owned six broader release/storage/renderer/keyboard/artifact/notice journeys. That plan has since been removed in the shared working tree by another writer; it is not an active prerequisite and is not revived here. Those broader journeys are not imported into this epic.

Working-tree planning metadata checked2026-11-15 shows both `layout-pointer-transaction/` files deleted by another writer and workflow-verification documents modified. Those changes are outside this task; neither is restored or overwritten. Snapshot history still describes their former context. The CAD replacement plan is canceled per current repository instructions; none of its tickets is revived here.

## Approved scope — 2026-11-15

**Decision:** the user approved composition-first scheduling of existing 002, coordination of overlapping domain work with existing workbench owners, and leaving remaining pure extraction unscheduled until 002's evidence and an explicit decision to retain the full architecture goal. Parent 001 remains open; 003–013 are neither done nor canceled. No transfer to another plan has been executed.

**Alternative not selected for immediate scheduling:** explicitly retain full001 extraction as an independent outcome. Then keep003-013, narrow each to the residual assembly shown by the research, reconcile current UI/lifetime contracts, and approve a shared ownership/dependency plan before migration. Evidence does not justify labeling those tickets done.

The original approval chose scope only. The user subsequently approved creating, validating and committing the revised documentation-only tree on 2026-11-15. The [adopted initiative](../initiative-editor-workspace-extraction/initiative-editor-workspace-extraction.md) contains one composition-proof epic and only original 002 as its implementation entry. This does not authorize starting code, changing another plan, selecting a shared active initiative or pushing. Frozen originals stay untouched.
