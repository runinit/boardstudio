# Graphite Batch 2 — direct Layout geometry

Mode: Operate. The user approved Batch 1 in chat on 2026-10-12 and authorized moving on to Batch 2 of the [migration plan](../../plans/ui-design/graphite-migration.md). The [approved Layout comp](layout-approved.png), semantic `--wb-*` palette, single header and docked Objects/Inspector remain the visual authority. No alternative design was generated.

## Implementation

Selected matrix, row, column and key geometry exposes a small canvas-attached strip with direct Stagger, Offset, Splay and Origin actions. Activation uses the existing selection conversion and tool controller. Accepted measurements appear beside the selection as native shared number fields: matrix origin, row/column/key offsets, column stagger, splay angle or splay origin. Units and edit operations are identical to the Inspector.

Canvas and Inspector drafts are independent. Both use the same request sequence, owner identity, accepted projection, typed operation resolver and PendingEdits settlement. Canvas drafts live with the controller, avoiding settlement writes to disposed component signals. The controller observes live canvas controls as well as a live Inspector; closing a dock does not retire a still-visible canvas editor.

The selection move handle starts the existing canvas drag route. Enter/Space focuses the exact canvas position field. Existing stagger, independent offset, splay and origin handles retain pointer capture, preview, snapping, keyboard nudges, cancellation, scope admission and one-change commit. Stagger inheritance and independent row/column offsets continue to come from Core. The splay angle has a scene-derived angular guide. SVG handles and HTML measurements retain a constant screen size across zoom, viewport and dock changes through a cleaned-up ResizeObserver; no polling was added. Guides use a non-scaling stroke.

The component uses pinned Dioxus Components through BoardToolbar/BoardToolbarButton and existing BoardNumberField. Geometry handles remain authored SVG because the primitive library has no keyboard-layout gesture primitive. Presentation code does not implement transform edit math or mutate documents directly.

## Verification and comparison

Evidence and commands are in `.scratch/graphite-batch2/`. Strict lint/rustfmt and WASM page typechecking passed. Native Layout tests passed: 77, with one pre-existing ignored test. The new mounted browser test proves canvas commits preserve an unfinished Inspector draft and share a single undo step.

Real-app checks cover exact numeric edits and undo, empty-draft Escape, pointer-preview Escape, pointer commit and undo, independent offset activation, splay/origin activation, exact splay and undo, keyboard access to move fields, and real move-handle pointer cancellation, accepted movement and one Undo. The batched comparison covers 1600px, 1359px, 1024px and 390px. The first visual pass exposed stale sizing after viewport changes and an overlapping splay handle; the correction uses post-layout surface metrics and moves the angle handle away from the measurement editor. Final confirmation passed: the measurements remain 268px wide at every tested viewport, stay within the canvas viewport, and do not overlap the selected keys. See [desktop](batch-2-desktop.png), [narrow canvas](batch-2-mobile.png) and [splay](batch-2-splay.png).

The broad Layout browser suite encountered the unchanged alignment popover's focus-restoration assertion (`layout_align.rs`, expected active element `m1-layout-align-trigger`, received an empty ID). Its panic poisoned later mounted tests. The original run and one serial retry were stopped rather than churning through secondary failures. This broad gate is not reported as passing. Focused transform verification passed: 22 controller/Inspector tests and 21 transform-menu/gesture tests, including cancellation, one-step undo/redo, Shift-latched independent offset, rotated/splayed pointer mapping and snapping.

## Boundaries and process cleanup

- These measurements expose existing transform values. Keycap width/height continue to use the existing Key size Inspector; this batch does not add a new resize gesture or change size semantics.
- Standalone parts retain existing component dragging and position controls; the new measurement strip is scoped to the matrix transform projection. Batch 3 and later workspace migrations have not started.
- No production/provider rebuild was needed. Save/recovery and accepted-document ownership remain on the existing routes; no preview is promoted to a save by presentation code.
- Superseded runners and verification browsers were stopped by their recorded process groups. The stale incremental Tailwind watcher (PID 1257150) was stopped and the pinned compiler regenerated CSS explicitly. The single Dioxus preview remains at [port 8182](http://127.0.0.1:8182/). Saved review notes and their recovery endpoint were preserved; no review decision or receipt was manufactured.
- Browser console also reports an existing malformed Layers chevron in `ui-shared/src/canvas_layers.rs` and existing missing-script/resource responses. These were not introduced or silently repaired by this batch. No uncaught page error occurred in the interaction check.
- The native Impeccable assembled-page capture cannot execute the worker-backed application. Actual app screenshots and interaction checks remain the evidence; automated phase gates are still open, as recorded in [Batch 1](batch-1.md).

## Broad-suite resolution — continuation

The unchanged Align focus assertion was reduced to two tests: matrix setup focus
followed by Align. The matrix fixture left its outside-stage button focused and
its DOM mounted; the popover correctly declined to steal outside focus. Removing
the matrix test root at completion makes the unchanged assertions pass. The
reduced pair passes (2/2), and the full Layout browser group passes (174/174).
Evidence is recorded in [the continuation packet](batches-3-6.md). Production
popover behavior and focus assertions were not weakened.
