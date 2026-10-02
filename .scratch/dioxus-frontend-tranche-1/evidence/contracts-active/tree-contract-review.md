# T1-10 tree contract review

Reviewed 2026-10-02 at migration worktree HEAD 5e5d3370; author proposal /tmp/frontend-run/tree-contract.md. React useWorkbenchTree, useWorkbenchSelection, WorkbenchTree and matrixGeometry match pinned 5a472a9426e6e38993361da402cd4ec730feb369. Read ticket 10 and boundaries to 11/12, existing acceptance/authority/constraints. No tree source edits, build or public tree regression reproduction. Findings are source-grounded contract risks unless explicitly stated otherwise.

## Decision

Approve the private hierarchy/projection/rendering work with the corrections below. Reject the original complete adapter contract as sufficient proof: it omits companions in group selections, omits SessionEpoch from scope, incorrectly implies empty selection can clear Session's anchor, and leaves the actual shared callback/lifetime unspecified. These are bounded private integration issues; no unspecified public facade/API approval is requested. Root owns all shared integration. Keep the full ticket open until that integration and public evidence pass.

## Exact selection projection

Use the existing accepted snapshot's scene.matrix_scenes cells and document metadata. Do not recreate matrix geometry or decide a canonical-looking ID proves a Part exists. Primary key member IDs come from enabled projected cells, then must exist in ProjectDoc.parts and the active Board.part_ids. Sparse matrix cells default enabled per core; disabled/missing parts can still have semantic tree key context and Empty slot presentation, but never real selected IDs.

Material correction: matrix/row/column selection is NOT primary-key-only. React useWorkbenchSelection.ts:94–99 uses matrix.partIds; Workbench.tsx:364–380 maps both primary members and /assembly IDs to coordinates; core/src/matrix.rs:749–834 includes generated assembly companions in matrix.part_ids. Preserve actual matrix membership order and intersect with current-board live parts and enabled coordinates. Row/column selects primary members AND live companions mapped to that coordinate. Key selection selects only its live primary ID; selecting a cell-component row selects only its live companion ID. Counts remain primary live KEY counts, not all selected components. Never pass semantic key/matrix/row/column IDs into SelectParts.

For component rows retain semantic identity (matrix, row, column, assembly) even if no Part was generated. To produce a real companion ID use projected member ID plus /assembly.id and verify exact membership/live Part, or the documented source canonical fallback only as a lookup candidate. A disabled cell can be navigated as semantic empty context; its stale live-looking record must not become a selected key/member. Standalone means source part outside matrix.part_ids membership, not merely outside the primary projected member set; otherwise companions are duplicated as standalone components. Guard against duplicate rows where an actual companion appears under a cell and in the board list.

Visible matrix association follows Workbench.tsx:330–335: explicit boardId wins; otherwise matrix.partIds must intersect selected board membership, with empty-matrix/single-board exception. Keep current canonical board filtering; physical instance must not create a second copy of document member IDs.

Board selection clears real part selection and sets board/none context as private presentation; it does not navigate unless the board picker changes. Empty key/component context is a legitimate private context. The minimum Inspector/selected-context presentation must expose that actual context, not show stale numeric controls for a previously selected part or invent placeholder domain IDs.

## Hierarchy/disclosure/keyboard corrections

Preserve useWorkbenchTree.ts:121–133,166–253,256–323 rather than only flat matrix rows. Ordinary unowned matrices live under Layout; layouts wrap their matrix as half:{layout.id} with the layout name and Linked/Independent detail. Left half/Right half grouping appears when mirrorLink axis exists; Components grouping and ownership use layout.partIds. Linked status applies to both link target and source, not only layouts with their own mirrorLink. Disclosures use stable independent board/layout/half/group/key IDs and reference default-open inversions for half groups/components. Geometry counts and labels are separate from selected real-ID sets.

Row/column grouping defaults to column and uses optional key boardstudio:v2:tree-grouping; storage errors preserve in-memory state (Workbench.tsx:167–168,1279). Root/panel author must supply a real Group objects / Tree grouping control when integrating; ticket 07's earlier omission was only because no tree existed yet, not authorization to omit grouping from 10. Keep both orientations and independent disclosure state.

Reference WorkbenchTree uses a tree labeled CAD structure, separate disclosure buttons and selection treeitems with aria-level/selected/expanded. Enter/Space activates the appropriate button; disclosure alone must not change Session selection. Do not silently retain the old flat listbox's ArrowUp/Down selection-navigation semantics on rows whose reference arrows nudge components. Reference useWorkbenchTree.ts:223,253 and Workbench.tsx:689–711 attach real-part arrow nudging; empty slots have no such edit handler. Root must either wire this through existing scoped edit/history actions or retain it as an explicit outstanding parity join; do not claim keyboard parity from pointer-only selection. Full creation/inspector/outline authoring remains excluded. Use actual public tests for the keyboard actions included, including focus after disclosure changes and long/scrolling trees.

## Concrete private callable seam

An acceptable minimal shape (names may be aligned with an equivalent root-owned implementation):

```rust
// New types in private page objects module; parent visibility only.
struct ScopedTreeContext { scope: boardstudio_application::Scope, context: TreeContext }
struct TreeSelectRequest { scope: boardstudio_application::Scope, context: TreeContext }
// Objects component props:
selected_context: Signal<Option<ScopedTreeContext>>,
on_select: EventHandler<TreeSelectRequest>,
on_navigate: EventHandler<(Scope, String, Option<String>)>,
```

TreeContext uses the proposed Board/Matrix/Row/Column/Key/Component identity variants. Author can provide a pure resolver from fresh ReadModel plus context to real part IDs; parent-only visibility is a new private helper, not widened existing public API. Root owns the callback bodies, shared selected-context signal, canvas synchronization and optional scoped keyboard-edit callback. Objects must route board/instance pickers through on_navigate rather than directly submitting Navigate behind the coordinator.

Root stores the holder at the mounted Editor/App owner shared by Objects, canvas and Inspector, outside conditional Inspector content. The holder contains only semantic context/validity metadata, never a writable document or a duplicate authoritative selected-ID vector. At every callback, compare captured request scope to Runtime.scope(), then resolve membership against the CURRENT accepted snapshot. Reject stale contexts/missing targets before SelectParts. A coordinate/key target resolves against the current scene, never a cached Part clone. Tree selection updates the same signal read by canvas and Inspector; canvas primary/component/empty-ghost hits route through the same adapter. The existing ghost rects have no selectable callback (`presentation.rs:830–842`), so merely changing Objects does not satisfy empty-slot synchronization.

## Scope, anchor and pointer lifetime

Use existing Scope including session_epoch, document_id, board_id and instance_id (`application/src/session.rs:Scope`, Runtime.scope). A string-only document/board tuple misses same-ID reopen. Context becomes invalid when scope changes and when current document edits remove/disable the target; revalidate same-scope targets on use. Root cleanup must happen before notifying/rendering children for the new scope, not only after paint in an effect. Recheck after any synchronous Session submit that can change accepted scope.

Session SelectParts Replace([]) clears selected_part_ids but DOES NOT clear selection_anchor_id (`session.rs:622–624`). Navigate and reconcile_selection_and_board also do not clear it (`634–674,1474–1508`). Do not claim the field was cleared. Existing public events are sufficient for observable scope safety if the private adapter keeps an anchor-validity scope stamp, invalidates it on transition and routes ALL tree/canvas selection through the adapter. A stored legacy Session anchor is unusable unless the stamp matches current Scope and anchor resolves to an eligible current-board live member. For the first Shift action after invalidation, submit Replace of the eligible clicked target instead of Range; this updates Session's actual anchor through its existing API. Ordinary same-scope Range behavior remains ticket 12's separately reproduced rectangular correction. Do not duplicate a second authoritative real-ID anchor or let the existing direct canvas Range path bypass validation. If a requirement is added that the underlying Session field itself must be None, that is a separate Session behavior repair, not achieved by a UI guard.

Selected real IDs must be pruned/cleared through existing SelectParts before the new-board Inspector renders. Validate navigation target before discarding context; rejected navigation must not erase a valid current state. A successfully accepted project reopen invalidates scope even when its document ID is unchanged.

Current Drag has no scope stamp (`presentation.rs:26–35`); handlers reuse old positions/view transforms, and cleanup effect only watches active_workspace (`659–672`). Root adds scope to the private drag/callback capture and rejects old move/up/cancel/pan callbacks, clears its pending drag, releases DOM pointer capture even for pre-threshold drags/pan, and sends GestureCancel when an active old gesture exists. Cancel old scope BEFORE Navigate/project exposure; an old cleanup callback must never cancel a new pointer with a reused pointer ID. Do not send GestureCancel blindly to a new scope. A weak/lifetime-owned registration from Editor to the shared transition owner is acceptable; clean it on unmount. Session Navigate's existing gesture invalidation does not alone clear the page's Rc<RefCell<Option<Drag>>> or pointer capture.

## Required joins and RF

Author may implement the pure hierarchy and renderer now using the cleared mapping. Root must freeze the exact callback/type names and shared transition implementation before integrating; no assumption that a new public facade exists. Verify live/empty/disabled keys, companions, standalone components, ordinary/linked layouts, row/column grouping, same-ID reopen, board/instance switch, stale click/move/up, and current-scope selection removal. Verify selection/disclosure revision/history neutrality separately from explicit nudge edits and one-step Undo. Preserve five Layers/Footprints and panel behavior. No claim of rectangular Shift parity or outline activation until tickets 12/11 respectively.

RF: extend existing RF-001 shared presentation/Runtime integration hotspot with the concrete Drag lifetime and semantic-context split; existing Session real-ID selection lacks a scope-tagged anchor. These are source-grounded design risks pending a correct public/native reproduction, not new reproduced defects. Current baseline scope safety is required now; later refactor may consolidate interaction scope ownership. No broad framework or domain rewrite is approved.
