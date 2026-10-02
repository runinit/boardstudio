# F7.2 private Case mount and authoring contract

Status: source-checked design handoff for the author. This is private UI wiring only. It proposes no public API, model/schema change, CAD engine behavior, Cargo/build change, or parent graph edit. Canonical F7.2 starts after INT.1 only and has no acceptance joins. F7.3 starts after F7.1; its INT.2/BND.1 are acceptance joins, not F7.2 blockers. F7.4 keeps its INT.2 join. Do not widen any of those edges or close a parent by mounting this UI.

## Current seam and disjoint ownership

The page selects `CasePanel {}` from the `active_workspace == "Case"` branch in `web/src/presentation.rs:1503-1504`. That component lives in `web/src/cad_presentation.rs:20-115`; it currently renders mechanical settings, generation actions/status, and the Case canvas. Authored Case body list/editor UI does not exist in the current Dioxus component. The Case path currently has no `InspectorPanel` mount: `web/src/presentation.rs:1511-1524` mounts that panel only for Layout and Parts. Case workspace layout/mount integration therefore needs a coordinator-owned splice in the shared page composition, not an edit to `Runtime` or a competing shell implementation.

### Frozen private component interface

Use one author-owned private component in `web/src/cad_presentation/case_bodies.rs` with private typed inputs and edit requests. Keep form/list/draft state there; do not pass `Runtime` into it. Its interface is:

```rust
#[derive(Props, Clone, PartialEq)]
pub(crate) struct CaseBodiesProps {
    pub board: Option<CaseBoardSummary>,
    pub bodies: Vec<CaseBody>, // already filtered to board.id
    pub scope: Scope, // document/session/board/instance identity
    pub editor_instance_id: u64, // root-generated for this mounted Case editor
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub generated_stack: bool, // active Case projection matches board
    pub mismatch: Option<CaseMismatch>, // configured board id/name
    pub editable: bool, // rendering hint only; root rechecks live gate
    pub feedback: Option<CaseBodyEditFeedback>, // root-filtered by editor + full request key
    pub on_edit: EventHandler<CaseBodyRequest>,
    pub on_show_configured_board: EventHandler<String>, // target board id
}
#[derive(Clone, PartialEq)]
pub(crate) struct CaseBodyRequest {
    pub editor_instance_id: u64,
    pub request_id: u64, // monotonic within this editor mount
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub edit: CaseBodyEdit,
}
#[derive(Clone, PartialEq)]
pub(crate) struct CaseBodyEditFeedback {
    pub editor_instance_id: u64,
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub request_id: u64,
    pub state: CaseBodyEditState,
    pub message: Option<String>,
    pub created_body_id: Option<String>, // set only for successful AddBody
}
#[derive(Clone, PartialEq)]
pub(crate) enum CaseBodyEditState { Pending, Saved, Failed }
#[derive(Clone, PartialEq)]
pub(crate) struct CaseBoardSummary { pub id: String, pub name: String }
#[derive(Clone, PartialEq)]
pub(crate) struct CaseMismatch { pub board_id: String, pub board_name: String }
#[derive(Clone, PartialEq)]
pub(crate) enum CaseBodyEdit {
    AddBody,
    SetKind { body_id: String, kind: CaseKind },
    SetThickness { body_id: String, value: f64 },
    SetClearance { body_id: String, value: f64 },
    SetZ { body_id: String, value: f64 },
    SetWallHeight { body_id: String, value: f64 },
    SetWallThickness { body_id: String, value: f64 },
    AddMount { body_id: String },
    SetMountKind { body_id: String, mount_id: String, kind: MountKind },
    SetMountX { body_id: String, mount_id: String, value: f64 },
    SetMountY { body_id: String, mount_id: String, value: f64 },
    SetMountHoleDiameter { body_id: String, mount_id: String, value: f64 },
    SetMountBossDiameter { body_id: String, mount_id: String, value: f64 },
    SetMountHeight { body_id: String, mount_id: String, value: f64 },
    RemoveMount { body_id: String, mount_id: String },
    SetGasket { body_id: String, gasket: Option<Gasket> },
}
```

Use the existing enum/type spellings from `core::model` when implementing; do not add public types. Body selection is local component state keyed by `scope` and body id; `SetCase` is emitted only for edit requests. No body deletion is exposed. The component does not construct or replace a full `CaseBody`. Each request includes the exact captured scope, root-generated editor-mount identity, snapshot token and revision from props. Root callback compares those against the current mounted editor identity and live scope/snapshot/gate, fetches that body from the latest accepted canonical snapshot, applies only this field-level change, and submits one `SetCase` for the updated body. This preserves optional fields and sibling mounts and prevents stale full-body closures from erasing each other. Do not weaken the gate below. Existing Case canvas, renderer, mechanical panel, operation lifecycle and shared panel behavior remain with their owners.

Mount the component in Case mode beside the existing Case preview, in the right Inspector slot matching React's Case inspector. Do not put the form inside the canvas element or make the whole Case panel the editor; keep preview/viewer and form independently usable at desktop and compact widths. The current Dioxus slot is not mounted for Case, so the parent shell owner must add that Case-specific inspector mount. Existing `.m1-case-panel` rules in `web/assets/m1.css:272-274, 310` only style the current vertical panel and compact minimum height; they do not provide the React-like Case editor list/form. The author should name/class the editor structure and provide the focused style delta; coordinator owns integration into the shared stylesheet. Root must also suppress the mismatched-board “Add case settings” and Bottom thickness controls while showing mismatch/editor: `has_settings == false` currently conflates a configuration owned by another board with absence (`cad_presentation.rs:50-60, 96-104`), while `set_settings` replaces project mechanical config (`cad_presentation.rs:115-164`, `case_settings.rs:4-18`). Generated mode omits the authored form but continues to render the existing generated mechanical assembly/status UI and a saved-body count/note. No simultaneous generated-stack and authored-body editing.

## Data ownership and scope guard

Build `mismatch` only from the actual configured board on the current Case projection, not from `case_settings::initial_settings` (which synthesizes defaults when no matching config exists). The authoring source of truth is `snapshot.document.case_bodies` (`core/src/model.rs:990-991`), filtered strictly by `body.board_id == model.active_board_id`. The selected board is `Scope.board_id`; never show every board's bodies as a fallback when board context is absent. The pinned React path similarly selects canonical `document.caseBodies` in `app/src/ui/useCaseWorkspace.tsx:24-27`; `CaseInspectorPanel` filters list items by selected board in `app/src/ui/CaseInspectorPanel.tsx:45-47`.

Physical instance context is a separate dimension. Rust `Scope` carries session epoch, document id, board id and optional instance id (`application/src/session.rs:42-48, 474-481`). `captured_case_document` in `web/src/cad_jobs.rs:460-530` overlays the selected physical-instance mechanical configuration and transforms effective case inputs; it does not transfer authored body ownership to an instance. React also passes canonical `document` for authored `caseBodies`, while `caseDocument` is the selected physical projection for mechanical readiness/configuration (`app/src/ui/useCaseWorkspace.tsx:17-27, 28-36`). Thus F7.2 body writes stay in the canonical document and on the active board. Include instance id in a mounted/draft key or callback freshness token so an instance switch discards stale in-flight drafts where applicable, but do not create instance-owned CaseBody records, apply the projected document back to canonical data, select physical instances, or implement F7.7 handoff. Board/instance selection remains with its current owner.

Every mutation request carries the accepted `Scope`, snapshot token/revision and body ID. Immediately before submit, compare captured scope to live `runtime.scope()`, verify the accepted document/token is still captured, and verify the body still exists in the accepted canonical document and belongs to the current board. Drop/reset stale field drafts on document, session, board or applicable instance changes. `Event::Edit` enqueues non-strictly and may rebase `base_revision` before dispatch (`application/src/session.rs:551-554, 1088-1093`); that rebase is not a scope guard.

Use the exact single-flight gate already used by `CasePanel`: lifecycle `Ready`, no `display_preview`, no gesture, and `Durability::Saved { revision: accepted.document.revision }` (`web/src/cad_presentation.rs:64-70`). This is stricter than the pinned React CaseNumber controls, which remain editable while a save is in progress. It is a deliberate safety restriction because Session rebases queued whole-body replacements; it is not claimed as parity or waived. `editable` only drives disabled rendering. Recompute the predicate from live `runtime.model()` inside every root mutation callback. While any edit applies/saves, disable Add/body/mount/gasket/numeric mutation. This serializes full `SetCase` commits, preventing one in-flight replacement from erasing another. If the gate is false, do not queue or silently drop: keep the draft uncommitted and provide visible pending/busy feedback until it can be submitted or report the existing wait/finish/cancel status. No extra queue/API is authorized.

Give each numeric draft one submit state/token. Enter commits once; the subsequent blur observes that token and is a no-op. Repeated Enter/blur for an unchanged draft is also a no-op. Mark the field accepted only once the updated canonical body is reflected and the saved-revision gate returns. Root owns a non-reused `editor_instance_id` allocated when its Case-inspector mount wrapper mounts (use `runtime.operation().0` once in that wrapper’s mount hook, not on every render). The child request id increments within that mount and resets only with it. Root stores pending identity as the full tuple `(editor_instance_id, Scope, request_id, snapshot_token, revision)` and emits `feedback` only when every field matches the currently mounted editor/pending request; stale completion/acknowledgement from another scope or unmounted editor is discarded. `Pending` is shown while applying/saving; `Saved` only when the matching request’s body appears in the accepted snapshot and the saved-current gate returns; `Failed` on core rejection/save failure with the Session reason. A successful `AddBody` feedback includes the root-created `created_body_id`; the component selects exactly that body ID, never infers it from list order. On Add failure it does not select a nonexistent body. If rejected or persistence enters `Durability::Failed`/`RecoveryRequired`, the old accepted body remains authoritative; preserve the attempted draft visibly with its error (never auto-replay). After recovery or explicit correction, a fresh action rereads and patches the latest accepted body. Invalid values stay visible with field-level feedback and never submit.

## Existing edit path and exact authored behavior

Root handles `AddBody` only if the live selected board exists and the submitted request scope/token match; it constructs the exact default `CaseBody` for the live selected board, allocates a unique ID, then submits existing `Event::Edit` + `EditCommand { phase: Commit, base_revision, transaction_id, target_ids: [new_body.id], operation: SetCase { body: new_body } }`. For other typed changes, root verifies body/mount membership in the latest accepted document, fetches the latest accepted body, applies just that field-level change, then submits the same existing `SetCase` command with `target_ids: [body.id]`. `core/src/lib.rs:1186-1192` confirms `SetCase` upserts by ID; standard Session persistence/history supplies save/undo/redo. Preserve optional/unknown existing body fields. For nested mount edits, clone/update only the matching mount and preserve other mounts and coordinates. Removing a mount or gasket is represented by a normal `SetCase` replacement (`mounts` without that ID; gasket absent); do not add a body deletion operation—the current model operation has no remove-body variant and F7.2 does not require deleting whole bodies.

Match the pinned React behavior/defaults from `app/src/ui/useCaseWorkspace.tsx:42-78` and `app/src/ui/CaseInspectorPanel.tsx:35-88`:

- New body: root-generated collision-checked id; name `<selected board name> plate`; boardId selected board; `kind=plate`, thickness `3`, clearance `0.5`, material existing PLA id/name match or `pla`, `z=0`, `wallHeight=14`, `wallThickness=2`, empty mounts.
- Body tabs list only the selected board's bodies, show order/name/kind, and select one. Empty copy is “Add a plate, tray, or lid to begin the case stack.”; no-board copy is “Add a board before creating a case body.”
- Kinds are Plate, Tray, Lid. Thickness > 0; clearance >= 0; z finite; for non-plate bodies wall height and wall thickness > 0.
- Mount default: root-generated collision-checked id, position (0,0), hole, hole diameter 2.5, boss diameter 5 and height 5. Expose hole/boss kind, X/Y, hole diameter; boss additionally exposes boss diameter and height. Support add, update, remove.
- Gasket is optional; add default `{ inset: 2, width: 2, depth: 1.5 }`; expose inset >= 0, width/depth > 0; remove clears only the gasket channel.
- Preserve actionable invalid-value feedback. Numeric field draft commits on blur or Enter. Escape restores the accepted field value. React's `CaseNumber` currently resets on accepted-value change and commits blur/Enter (`app/src/ui/InspectorControls.tsx:36-62`) but does not implement Escape; the ticket explicitly requires Escape rollback, so implement and test that interaction instead of copying this gap.

React composes the mismatch panel and editor together when the mechanical configuration belongs to another board, with “Show configured board” (`app/src/ui/useCaseWorkspace.tsx:80-97`, especially 82-86). For the selected board, show the authored editor alongside that explanation/action; selecting it changes board through the existing navigation owner. Do not call `Event::Navigate` from the child. Root wires the requested target board id from `on_show_configured_board` to the existing guarded `navigate` closure (`web/src/presentation.rs:810-853`), invoking it with the current render Scope, configured board ID and `instance_id=None` so an incompatible physical instance is dropped. That closure validates captured scope/target membership and performs scoped drag cancellation plus selected-context/anchor cleanup before submitting `Event::Navigate`. Reuse this closure; do not add a Case-specific competing navigation path. Do not call `SetMechanical`, replace the configuration, or implement configure/disable controls here.

When the selected board has a matching active generated mechanical stack, match React's generated branch (`useCaseWorkspace.tsx:34-35, 84-97`): determine the active stack from the current Case mechanical projection (physical-instance projection when supplied, canonical configuration otherwise), while continuing to count/read authored bodies from the canonical document. Show existing generated assembly/generation UI and the note that selected-board authored bodies remain saved, hide/unmount the authored editor so local drafts are discarded, and never mutate/delete hidden body records. F7.4 owns mechanical editing and disable-stack behavior; F7.6 owns generation/readiness/export. F7.2 contributes only the presentation join, no generator or CAD work.

## Important current implementation gap

In `web/src/cad_presentation.rs:27-60, 93-109`, `has_settings` checks only whether the effective mechanical configuration matches the active board. There is no cross-board mismatch explanation/action, and the `!has_settings` branch offers “Add case settings”; `set_settings` then creates/replaces mechanical configuration (`case_settings.rs:4-18`, `cad_presentation.rs:115-164`). F7.2 must not turn that existing F7.4 configuration action into an authored-body control or silently overwrite another board's configuration. Render the source-backed mismatch explanation and navigation join for this ticket; leave configuration editing/enable-disable implementation to F7.4 and coordinate its mount when integrating.

The Rust `Runtime` exposes `operation() -> OperationId`, not a domain ID allocator; existing Rust code has no `makeId` counterpart to React `workbenchGeometry.ts:17`. Do not use a bare operation number as an entity ID. Root integration owns a private allocator: derive a prefixed candidate from a fresh `runtime.operation().0`, check against all accepted body IDs (or all mount IDs document-wide), and retry on collision. No dependency, Cargo, API or schema change is needed. The component requests `AddBody`/`AddMount` without IDs; root supplies unique IDs; Event operation IDs are allocated separately. The author allocates `request_id` monotonically within the provided editor mount and echoes the tuple in `CaseBodyRequest`; root filters feedback by the full tuple and includes `created_body_id` for a successful AddBody.

The Rust `CaseBody` already represents `kind`, thickness, clearance, optional z/walls/mounts/gasket/material/features/openings (`core/src/model.rs:717-744`), and core validation/readiness is present (`core/src/lib.rs:721-743`). This is UI projection/edit work over existing engine capability; no new CAD or model contract is indicated.

## Verification and refactoring record

Use the same pinned React Case fixtures/actions and public Dioxus route: add/select bodies on multiple boards; verify cross-board bodies are hidden; edit each field/default, nested mount and gasket; exercise invalid values, blur, Enter, Escape; verify save/reopen and Undo/Redo; switch board/session/instance with active drafts and prove they cannot commit across scope; test generated-match hidden-editor/retained-record state and mismatch explanation/navigation. Verify focus order and keyboard operation plus desktop/compact and light/dark. Keep exact fixture/actions and outputs in the ticket evidence; browser visual matching alone is insufficient. The Ready/saved gate needs explicit paired React/Dioxus acceptance for busy/save/recovery behavior: prove ordinary field edits are usable, an attempted commit during busy state preserves its draft and reports why it is blocked, and retry/recovery neither loses nor silently replays it. This stricter interaction restriction receives no acceptance waiver; if the paired evidence shows unacceptable friction, stop for a serialized fresh-field-patch coordinator decision instead of relaxing the gate.

**Refactoring takeaway: No new refactoring takeaway observed.** A private form component is the bounded ownership seam for this feature, not a rationale to restructure Case settings, Session, CAD or the shell. Existing RF findings may be linked if verification produces new source-backed evidence; otherwise leave the register untouched.

## Source map

- React Case authored fields/defaults and generated/mismatch branch: `app/src/ui/CaseInspectorPanel.tsx:35-89`, `app/src/ui/useCaseWorkspace.tsx:21-97`.
- React numeric draft behavior: `app/src/ui/InspectorControls.tsx:36-62`.
- React workbench composition and physical projection split: `app/src/ui/Workbench.tsx:93-95, 321-329, 420-428, 1386-1387`.
- Dioxus Case workspace mount/current panel: `web/src/presentation.rs:1503-1504`; Inspector slot condition `1511-1524`; current panel `web/src/cad_presentation.rs:20-164`.
- Private Case defaults/projection: `web/src/case_settings.rs:4-68`; `web/src/cad_jobs.rs:460-530, 800-830`.
- Canonical data and command: `core/src/model.rs:717-744, 990-991, 1235-1245, 1288-1310`; `core/src/lib.rs:1186-1192`.
- Session scope/navigation/edit queue/history authority: `application/src/session.rs:42-55, 474-481, 551-558, 634-658, 929-978, 1088-1130`.
- Ticket and dispatch bounds: `.scratch/dioxus-case-workspace/issues/01-authored-case-workspace.md`; `.scratch/dioxus-case-workspace/DISPATCH.md`; canonical `.scratch/dioxus-frontend-v1/tasks.json` F7.2/F7.3/F7.4/F7.6/F7.7/INT.1/INT.2/BND.1 entries.
