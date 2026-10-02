# Workbench composition contract proposal

Status: design proposal only; implementation awaits independent Astra clearance.

## Baseline and authority

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/frontend-workbench-composition-20261002`
- Branch: `codex/frontend-workbench-composition-20261002`
- Exact base: `89b1de8a28fdf02db91d972c90a69235bfbbbffb`
- Baseline was clean. No repository files have been changed and no build/test was run.
- Authority: user's approved six-stream migration and composition task packet; `CONSTRAINTS.md`; `docs/adr/0003-rust-application-ownership.md`; current frontend roadmap and first-tranche acceptance rules.

## Problem

`web/src/presentation.rs` has existing private feature modules for Objects, panels, Keymap, Parts and Case, but `Editor` still directly chooses the contents of the Objects, toolbar/canvas and Inspector regions for all workspaces. Layout's canvas markup and pointer lifecycle remain especially integrated with `Editor`. A single new file containing all six streams would preserve the same contention and is not the proposed ownership boundary.

## Design decision

Use one minimal private dispatcher plus six flat, independently owned workspace modules. Do not create a new crate, plugin/registry/template framework, or hierarchy of subcrates. The dispatcher only selects a workspace module and forwards the corresponding already-prepared render input. Each workspace file owns its current Objects content, toolbar, canvas/content and Inspector content. Shared panel framing remains shared.

Proposed private files:

- `web/src/presentation/workspace_composition.rs`: module registration and a thin four-surface dispatcher. It owns no feature markup, hooks, state or behavior. Its routes call the matching feature-owned function.
- `web/src/presentation/layout_workspace.rs`
- `web/src/presentation/pcb_workspace.rs`
- `web/src/presentation/keymap_workspace.rs`
- `web/src/presentation/keycaps_workspace.rs`
- `web/src/presentation/case_workspace.rs`
- `web/src/presentation/parts_workspace.rs`

The six feature files are disjoint authoring surfaces. The shared dispatcher is stable for changes inside an existing workspace: adding Layout Inspector content edits `layout_workspace.rs`; adding Case Objects content edits `case_workspace.rs`. Only adding/removing a workspace changes dispatch.

### Private calls and inputs

The private dispatcher exposes only these internal call shapes:

```rust
fn objects(workspace: &str, inputs: WorkspaceObjectInputs) -> Element;
fn toolbar(workspace: &str, inputs: WorkspaceToolbarInputs) -> Element;
fn canvas(workspace: &str, inputs: WorkspaceCanvasInputs) -> Element;
fn inspector(workspace: &str, inputs: WorkspaceInspectorInputs) -> Element;
```

These are private implementation calls, not crate/public APIs. The dispatcher matches the current workspace string and forwards each input to a same-named function in that workspace module. The `Workspace*Inputs` above are routing enums with one variant per current workspace, not a shared writable model or feature state store; each variant carries that workspace's distinct typed render inputs. Alternatively, equivalent private function overloads may be used if that is simpler within Rust's type system. Feature input structs and render functions live in their owning file, never in the shared dispatcher.

Representative feature-owned signatures are:

```rust
layout_workspace::objects(LayoutObjects<'_>) -> Element;
layout_workspace::toolbar(LayoutToolbar<'_>) -> Element;
layout_workspace::canvas(LayoutCanvas<'_>) -> Element;
layout_workspace::inspector(LayoutInspector<'_>) -> Element;

case_workspace::objects(CaseObjects<'_>) -> Element;
case_workspace::toolbar(CaseToolbar<'_>) -> Element;
case_workspace::canvas(CaseCanvas<'_>) -> Element;
case_workspace::inspector(CaseInspector<'_>) -> Element;
```

The same four signatures apply to PCB, Keymap, Keycaps and Parts with their own `*Objects`, `*Toolbar`, `*Canvas` and `*Inspector` inputs. Define only inputs needed by the current subtree in that file. Prefer references to the accepted snapshot/document and current `Scope`, plus the existing feature actions/signals/callbacks prepared by `Editor`; do not clone `ProjectDoc`, add writable duplicated fields, bundle all six workflows into a shared giant props struct, or make generic common context carry feature-specific state. If an owned surface needs no content today, its feature function returns the exact current empty/placeholder output for that surface.

Keep `Editor` as the caller and lifecycle owner. It continues to create every hook/signal, subscribe/reconcile scope, prepare every current action and handler, retain pointer/canvas handles and cleanup registration, and submit all Runtime/Session events. The extraction moves RSX ownership and per-workspace surface composition only. In particular, Layout's existing pointer transaction, DOM listener/mount handlers, render-scope guard, final sample and cleanup remain created and owned in `Editor`; Layout's canvas function receives those existing handles/callbacks as render inputs and attaches them to the same SVG. It does not start a second interaction lifecycle.

A small common frame may continue to own `ObjectsPanel` and `InspectorPanel` wrappers, panel settings/visibility preferences, compact controls, the shared workspace content section, and layout geometry. Each dispatcher route supplies the wrapper's child content from its workspace-owned `objects` or `inspector` function. Toolbar/canvas routing is likewise delegated to the matching feature function. This keeps existing panel state/ARIA contracts centralized while making each stream's interior markup independently owned.

## Source-led surface map

| Workspace | Objects content today | Toolbar/canvas content today | Inspector content today |
| --- | --- | --- | --- |
| Layout | shared `Objects` with current selection/navigation/nudge handlers | Layout status/Footprints toolbar and main SVG canvas | selected-context summary and current position `Inspector` |
| PCB | shared `Objects` | current generic placeholder path | existing current generic Inspector path when present |
| Keymap | shared `Objects` | `KeymapCanvas` SVG or current unavailable status | `KeymapPanel` or unavailable status |
| Keycaps | shared `Objects` | current generic placeholder path | existing current generic Inspector path when present |
| Case | shared `Objects` | pending-instance status or `CasePanel` | pending-instance status or `CaseBodyInspector` |
| Parts | `PartsLibraryPanel` | current generic placeholder path | `PartsInspectorPanel` |

This is a composition inventory, not authorization to fill missing features. The current `Export` route remains in its existing root-owned branch and continues to mount `ExportPanel`; it is not silently dropped into the six-workspace router. The generic `PlaceholderWorkspace` fallback remains for any existing non-six value/current behavior. Do not turn PCB or Keycaps placeholders into new UI as part of this extraction.

## Exact production ownership

- Add the dispatcher and six private feature files listed above.
- Edit `web/src/presentation.rs` only to declare private modules, call the dispatcher at the existing four composition points, and retain all current root state/hook/action/lifecycle ownership.
- Reuse existing feature components and their props. Do not edit `Runtime`, `Core`, CAD, `web/src/m1.css`, public/generated contracts, saved formats, or other feature implementation files. If source constraints make a surface impossible to move without changing action or visibility contracts, stop and revise this proposal before widening scope.

Case-specific tree entries and moving mechanical settings controls to another menu remain later feature slices. This packet does not alter either.

## Mount and behavior invariants

Preserve current visible routes, labels, panel modes/sizes, focus behavior, accessibility relationships and transitions. Preserve the exact Case viewer key based on session epoch, board id and instance id; do not add keys that remount the viewer on toolbar/panel changes. Preserve current workspace switching behavior, Case instance reconciliation and scope guards. Keep export panel and placeholder fallback mounted through their current routes.

Preserve canvas lifecycle: same SVG receives existing mounted event and pointer/keyboard/wheel handlers; no extra mount target, listener owner, remount, or duplicate cleanup. Preserve selected-context, tree navigation/nudge, layer changes, Parts query/selection, compact Objects/Inspect controls, Escape/pan/drag/Undo, save retry/recovery, all existing Layout layers and Footprints behavior. Composition must not turn view-only inputs into writable copies.

## Public behavior seam and characterization

Use paired public browser journeys against the current TypeScript reference at `http://127.0.0.1:5175` and the Dioxus candidate at `http://127.0.0.1:34687`, with identical fixture, viewport/theme and user actions. Record the baseline before extraction and rerun the same journeys after. Do not test source shape or private component structure. Assert only visible/user-observable results and resulting document/history/selection behavior.

Characterize at least:

1. Open the same saved/demo document, visit all six workspace tabs, and verify visible surfaces plus shell state. For PCB and Keycaps, assert the currently visible placeholder behavior. Also verify Export still opens its current panel and unknown/fallback content retains its existing route.
2. In Layout, select an object through the tree and canvas; verify selection summary, Inspector controls, canvas selection and document/history effects as they behave today.
3. Verify Layout layers and Footprints, Keymap selection/layer/panel, Parts query/selection and Case physical-instance selection/viewer through public controls.
4. Verify compact Objects/Inspect toggles, keyboard/focus behavior and compact Case control reveal without viewer remount or scope leakage.

Treat this as characterization of current candidate invariants for a behavior-preserving extraction. Do not manufacture a failing source test to force a red phase. A baseline failure needs an exact public reproduction and must not be converted into an assumed approved behavior change; reference defects still require a separately approved corrected oracle.

The agreed primary seam is the paired public browser journey. Supporting evidence is the affected root-orchestrated native, strict WASM Clippy/build and established browser/build checks. Do not run Cargo/build in this author worktree; root owns integration verification. Preserve existing failures and unavailable gates in the result.

## Completion evidence

- Independent Astra clearance of this concrete ownership/input contract before source extraction; independent Standards and Spec review of the resulting diff.
- Same paired public journeys before/after, with fixture, source/build identity, viewport/theme, observable state and document/history/selection outcomes recorded.
- Root-run affected native/WASM/build checks, with failures/unavailable gates retained.
- Diff confirms only `presentation.rs`, dispatcher and six workspace-owned files; no public visibility changes, duplicate authority/state/algorithm, lost Export/fallback route, changed mount identity, or weakened checks.
- Update the post-port refactoring handoff with RF-001 only: existing source continues to support the known composition hotspot and this is a local mitigation, not evidence that broad coupling is resolved. Do not add a duplicate RF from this design.

## Risks and limits

- Layout's canvas markup is closely coupled to computed geometry, pointer state and handlers in `Editor`. Its `LayoutCanvas` typed input may be mechanically substantial. Keep the types feature-specific and read-only; if the boundary becomes an oversized forwarding object or needs lifecycle movement, leave that region in root and narrow the first extraction for review instead of inventing a shared store.
- Dioxus RSX/component macro constraints may affect moving subtrees. Retain current scoped values, callback types and mount key exactly; do not widen APIs or alter behavior to satisfy module boundaries.
- Conditional mounts may affect focus, listener cleanup or viewer lifetime despite identical markup. The browser journey must cover compact Case focus and workspace/scope transitions.
- This extraction completes no feature stream and authorizes no public API change, broad refactor, behavior change, integration merge, cutover, or edits to other worktrees.

## RF takeaway

RF-001 remains the sole evidence-backed takeaway: the root source routes all workspace surfaces through one `Editor`, while existing feature modules own much of the inner content. Disjoint private workspace files are a local mitigation. No new RF is justified by this contract alone.
