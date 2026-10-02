# Proposed F3.1 object tree and selection tickets

Draft for approval only. No repository files or task graph were changed. Each ticket description below is path-free and intended to transfer into the local ticket template after approval.

## Proposed breakdown

### 01: Publish the private Layout workspace seam

**Blocked by:** None (can start immediately).

**What it delivers:** Layout mounts through one reviewed private composition seam that passes the current session read model and explicit selection/navigation callbacks to feature modules. Existing Layout canvas, Layers, Footprints and current object-list behavior still work through that seam.

**Acceptance criteria:**

- [ ] Review the concrete callback/read-model contract and prove it is reachable from the page binary without widening public API visibility.
- [ ] Mount the Layout composition through the seam and demonstrate current object-list selection and canvas selection still use the same session and document history.
- [ ] Preserve the accepted F3a Layers, Footprints, selection/drag/Undo behavior in the integrated public UI.
- [ ] Record paired shell evidence and any refactoring observation.

### 02: Browse and select the active board’s Layout objects

**Blocked by:** 01.

**What it delivers:** A keyboard-accessible tree shows the active board’s Layout, matrices, configured row or column groups, keys, cell components and standalone components. Disclosure toggles are independent from object selection. Selecting a populated matrix scope, key or component updates the same visible canvas selection and inspector context as selecting it on the canvas.

**Acceptance criteria:**

- [ ] Project stable tree identities and reference-compatible labels/details for the active canonical board, using existing matrix membership and projection data.
- [ ] Support independent disclosure and selection, including the current row/column grouping behavior and expanded component children.
- [ ] Scope every selectable member to the active board and enabled/live matrix membership before sending selection to the session; empty slots may open key scope but never manufacture a selected document part ID.
- [ ] Verify tree-to-canvas and canvas-to-tree selection on a populated key, matrix group and standalone component, including a split-board fixture; disclosure and selection leave document revision/history unchanged.
- [ ] Verify accessible names, visible focus and keyboard operation at desktop and compact widths.

### 03: Navigate outline versions and bridge targets from the tree

**Blocked by:** 01.

**What it delivers:** The active board’s tree exposes its outline, Generated entry, saved fixed versions and applicable bridges. Choosing Generated or a fixed version enters outline context, clears part selection and activates that version through the existing outline edit path. Choosing a bridge enters outline context, clears part selection, records the selected bridge and fits the camera to its geometry. This is tree navigation and activation, not the full outline authoring surface.

**Acceptance criteria:**

- [ ] Show Generated/current status and fixed-version labels from the selected board’s outline state, with stable IDs and independent outline disclosure.
- [ ] Show only bridges belonging to the active board’s outline scene and preserve their association with the applicable matrix where the reference does so.
- [ ] Selecting Generated or a fixed version follows the reference transition into outline context, clears part scope/selected parts and invokes the existing outline activation edit; verify its document/history effect and Undo/redo behavior instead of asserting revision neutrality.
- [ ] Selecting a bridge follows the reference transition into outline context, clears part scope/selected parts, records that bridge as the current presentation context and fits the camera to its geometry; verify bridge selection and camera movement do not themselves edit the document.
- [ ] Verify stale/missing bridge or version entries remain readable and cannot act on another board’s target.
- [ ] Verify tree labels, focus and navigation in paired desktop/compact browser scenarios. Do not add a placeholder inspector; full outline authoring controls remain in F3.4.

### 04: Match canvas toggle, rectangular range and scope transitions

**Blocked by:** 02. The range/mode contract must be resolved against the existing session before implementation; no public API expansion is implied.

**What it delivers:** Canvas and tree selection stay synchronized for ordinary selection, Ctrl/Cmd toggle and Shift rectangular key ranges. The range starts at the current key anchor and contains only live members in the rectangle. Changing project or board cancels an active gesture and clears invalid scoped selection while preserving valid presentation state; tree keyboard behavior remains usable in compact layout.

**Acceptance criteria:**

- [ ] Match the React anchor contract: Shift selection uses the anchor matrix/row/column and includes live members in the row/column rectangle, not the linear interval between flattened IDs.
- [ ] Ctrl/Cmd selection toggles the intended live component/key member and updates both tree and canvas; modifiers never start a drag.
- [ ] Reject cross-board, disabled, empty and stale member IDs at the selection boundary; switching board/project cancels active gestures and removes invalid selection/anchor state.
- [ ] Verify stationary selection, selection after scope changes, revision/history neutrality, desktop/compact focus and paired React action traces.
- [ ] Leave panel resizing/drawer focus-restoration acceptance with the F2.3 owner; this ticket only verifies tree focus and selection behavior in the available public frame.

## Parent coverage

- Ticket 01 establishes the exact INT.1 private composition/callback seam and preserves already accepted F3a behavior.
- Ticket 02 covers F3.1 board/matrix/row/column/key/component tree, independent disclosure, tree/canvas synchronization and baseline board/membership correctness.
- Ticket 03 covers F3.1 outline/version/bridge groups and contextual navigation.
- Ticket 04 covers F3.1 Ctrl/Cmd toggle, anchored rectangular Shift selection, scope cleanup, gesture cancellation and compact keyboard/focus behavior.
- Across all tickets, tree/selection/visibility and panel-open actions are checked for no document revision/history mutation; outline-version activation is the explicit document-edit exception and carries its own history assertions. No ticket defers active-board or live-membership correctness to ticket 04.
- Do not gate tickets 02 or 03 on F2.1 project-library work or F2.3 panel implementation. F3.1 is start-gated only by INT.1 and has no later acceptance join; the full F3.7 integrated Layout parity slice explicitly joins F2.3. Keep that join at F3.7 rather than making panel work a prerequisite for the tree.

## Boundary questions and actual blockers

- Actual start blocker for every F3.1 consumer is ticket 01 / INT.1: the private read-model/callback slot and same-crate reachability need a reviewed concrete contract. Existing F3.1 task metadata lists only INT.1 in `start_after`/`depends_on`.
- React selection scope includes matrix/row/column/key/component. Dioxus `ReadModel` currently stores selected part IDs, one anchor part ID and selection mode, but no semantic matrix/row/column scope. React also represents outline selection separately from part selection. Resolve whether private presentation scope plus session-owned part IDs faithfully preserves that ownership split; do not infer a need for a new public contract.
- Empty key scope has no real part ID. The React projection may create a synthetic cell ID, then filters it out of selected part IDs. Dioxus `SelectParts` validates against actual document parts, so use separate semantic key scope and valid part selection rather than sending a fabricated ID.
- Current Dioxus range handling is a flat ordered slice of `range_part_ids`; React explicitly computes the rectangle from anchor and target row/column. Prove a rectangle-only ordering can safely drive the existing session event or identify the narrow private adapter requirement before choosing an implementation. Do not accept the current flat visible-ID list as parity.
- Outline/version/bridge projection and basic activation/context fit do not require full F3.4 outline authoring. F3.4 owns version controls, feature drawing/editing, refinement and complete outline-inspector workflows. F2.3 may remain a final integrated-frame acceptance join, but is not a tree-feature start dependency.
- Outline actions have different effects: React outline choice enters outline context and clears part selection; version choice invokes a document edit, while bridge choice records bridge context and fits the camera. F3.4 is not a prerequisite for this narrow path if existing outline-edit and camera interfaces suffice; it remains the owner of full version management and outline authoring. Do not claim an editor is already present or add a placeholder inspector.

## Refactoring register

No new RF finding is asserted from this read-only breakdown. Existing RF-001 (shared presentation/Runtime integration hotspot) and RF-006 (canonical/physical/sample scope confusion) are relevant evidence to retain if implementation reveals a new reusable refactoring takeaway.

## Source evidence

| Evidence | What it establishes |
| --- | --- |
| `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/issues/03-layout.md:1-15,157-170` | F3 scope, F3a preserved behavior, F3.1 start gate, and F3.7's explicit F2.3 integration join. |
| Same file `:17-68` | F3.1-relevant user stories: hierarchy, separate disclosure/selection, synchronized tree/canvas, scopes, modifiers, active-board membership and compact state. |
| Same file `:121-160` | Session owns accepted read model/selection/navigation; Dioxus owns disclosure/focus/drafts; no duplicate geometry or public visibility widening; existing test-policy decisions. |
| `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/tasks.json` (`F3.1`, `INT.1`, `F2.1`, `F2.3`, `F3.2`, `F3.3` rows) | Exact start blockers, F3.1 acceptance, INT.1 acceptance and separation from library/panel tasks. F3.1 currently has `depends_on: [INT.1]`, with empty `acceptance_after`. |
| `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/AGENT-ROUTING.md:27-39,52-64` | INT.1 contract review and existing proposed F3.1a/b/c decomposition. |
| `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/EXECUTION.md:15-36,47-53,61-67,81` | Slice start versus acceptance joins, ownership of shared presentation/runtime files, first-visible-tranche ordering, scope/cancellation adapter context, required evidence and crate-boundary constraint. |
| `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/CONSTRAINTS.md:360-427` | Preserve paired parity and existing gates; no destructive or unapproved API/contract changes; no weakened tests or suppressed failures. |
| React reference `app/src/ui/useWorkbenchTree.ts:121-154,166-255,256-324` at commit `5a472a9426e6e38993361da402cd4ec730feb369` | Stable ID patterns, active-board tree projection, outline/version/bridge rows, matrix grouping/key/component children, selected scopes, and mirrored-half/layout grouping behavior. |
| React reference `app/src/ui/useWorkbenchSelection.ts:29-113` at the same commit | Ctrl/Cmd toggle; anchor is matrix/row/column; Shift selects the live-member rectangle and filters to the current board; scope selection filters valid part IDs. |
| React reference `app/src/ui/WorkbenchTree.tsx:3-17,41-59` at the same commit | Tree entry contract, independent disclosure and selection controls, accessible roles/names/selection state and visibility actions. |
| React reference `app/src/ui/Workbench.tsx:481-490,669-679` at the same commit | Board switching clears selection/scope/anchor and cancels placement; version selection invokes outline activation, while bridge selection sets bridge context and fits camera to the bridge. |
| Continuation `application/src/session.rs:102-115,155-160,168-170,186-200,562-625` and `core/src/model.rs:1188-1220` | Read model has selected IDs/anchor ID/mode but no semantic selection scope; SelectParts validates real part IDs; Range slices a supplied linear order; current event/edit surfaces include edit and camera operations and the model exposes outline selection operation. |
| Continuation `web/src/presentation.rs:444-498,1035-1056` | Existing Dioxus object list and selection callbacks; canvas Shift currently passes a flat visible-ID sequence to the linear range event; no full outline editor is represented by this object list. |
| Continuation `web/src/runtime.rs:117-160` | Runtime exposes current scope/model and submits session events; scope changes have cancellation cleanup. |
| Continuation `.scratch/dioxus-frontend-v1/refactor-findings.json` | Existing RF-001 and RF-006 titles; no new RF is claimed here. |
