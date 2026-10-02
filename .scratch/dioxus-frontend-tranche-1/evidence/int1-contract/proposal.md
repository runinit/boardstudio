# INT.1 private workspace composition contract

- Packet: T1-01 / INT.1; behavior-preserving private presentation extraction.
- Source baseline: `f3bb02a3d0eb15cf01822ed29d9b1632048df9c2` on `codex/frontend-int1-20261002` in `/home/chris/.local/share/boardstudio/worktrees/frontend-int1-20261002`.
- Integration target: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, branch `codex/rust-v1-ui-parity-20261001`.
- Spec: `.scratch/dioxus-frontend-tranche-1/issues/01-private-workspace-composition.md`; constraints and authority/acceptance docs read from this baseline.

## Observable result

The Library, Objects and Inspect UI continue to render in the same places and issue the same existing actions after their implementation moves into private page presentation modules. The Dioxus page remains the caller and composition owner. No behavior change, provider/session rewrite, public API/member visibility widening, generic component framework, or DOM/label change is included.

## Source-checked inputs and actions

- `web/src/presentation.rs::App` creates exactly one `Rc<Runtime>` with `use_hook(Runtime::new)`, installs one root change subscription that increments the shared `Signal<u64>`, provides that Runtime and version signal as Dioxus contexts, and closes/unsubscribes the runtime on root drop.
- `Runtime::model()` returns a clone of the authoritative `Session::read_model`; `Runtime::submit` sends existing `Event`s to that same session. No second session, document store or history authority is introduced.
- `Library` reads `Rc<Runtime>` and the root `Signal<u64>` contexts. It reads lifecycle and accepted identity, lists through the existing `runtime.store.list_documents()`, and invokes existing `open_fixture`, `open_saved`, and `import_file` actions. It calls the existing page `close_project_menu` helper.
- `Objects` reads those same Runtime/version contexts. It projects parts on `model.active_board_id`, selected IDs, definitions and physical instances from the accepted snapshot/read model. Existing actions are `Event::Navigate` for board/instance and `Event::SelectParts` with `SelectionMode::Replace`; keyboard navigation preserves focus by focusing the existing `m1-object-{index}` DOM target.
- `Inspector` reads those same Runtime/version contexts. It projects the selected part from the accepted snapshot plus selected IDs. Existing X/Y inputs submit `Event::Edit` using `EditCommand::MoveParts`, existing revision/transaction identity, and preview/commit phases. Escape restores the preview, Enter/Apply commits, component teardown restores an in-flight preview. Numeric edit state/helpers can move with this component because their only uses are inside Inspector.
- Shell call sites remain in place: `Library` is used in the project menu and landing page; `Objects` remains in the editor's existing object-panel slot; `Inspector` remains in the existing Layout-only inspector slot. The caller-owned compact-open signals and surrounding IDs/classes remain in `Editor`.

## Smallest module shape

Register one private `mod private;` under `web/src/presentation.rs` and add:

- `web/src/presentation/private/mod.rs` — private child declarations and parent-visible re-exports only.
- `web/src/presentation/private/library.rs` — extracted `Library` implementation, calling the existing ancestor `close_project_menu` helper.
- `web/src/presentation/private/objects.rs` — extracted `Objects` implementation.
- `web/src/presentation/private/inspector.rs` — extracted `Inspector`, `NumericEdit`, `KeyboardHandler`, `submit_position`, and `commit_numeric` implementation-local state/helpers.

The page parent imports only the three component functions. Existing Dioxus components are named from `presentation.rs`; extraction requires only `pub(super)` visibility at the parent-facing child boundary. No explicit typed props or additional context type are needed: these components already consume `Rc<Runtime>` and `Signal<u64>` from the root context. `WorkspaceState`, `ThemeState`, and `LayerVisibility` stay with existing page composition because these three extracted components do not consume them. No `main.rs` registration is needed because `presentation.rs` owns its child module tree.

Owned source files: `web/src/presentation.rs` and the three new private feature files plus `private/mod.rs`. `web/src/runtime.rs` is unchanged. No other worker edits these shared presentation files during this packet.

## Existing characterization to reuse

This is an extraction, not a behavior change or defect repair. Do not add tests that call private functions or duplicate their implementation. Preserve and rely on the existing accepted F1/F3a public behavior characterization, including project open/import, shell/theme/navigation, five Layout layers and the shared Footprints toggle, object selection, drag/Undo, numeric position preview/commit/cancel, camera behavior and keycap display. Existing public React traces include `app/e2e/library-workflow.spec.ts`, `app/e2e/workbench.spec.ts`, `app/e2e/mirrored-layouts.spec.ts`, `app/e2e/vik-module-review.spec.ts`, and `app/src/ui/createCanvasInteractions.test.ts`; use the already recorded F1/F3a/Dioxus acceptance evidence where it matches this source revision. This packet adds no new acceptance claim from an unrun browser check. Root reports the baseline public F3a browser regression passed this turn at `http://127.0.0.1:34643/`, using session `frontend-int1-before` (keycaps, five layers, Footprints, camera/history, cap edit Undo, desktop/compact, themes and focus); artifacts are at `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-tranche-1/evidence/int1-before/`. Browser server PID 1190604 / session 89812 is coordinator-owned. Reuse that evidence; do not take over the server.

## Verification planned after contract approval

- Existing affected checks, with no browser/build ownership in this packet: `cargo fmt --manifest-path web/Cargo.toml --all -- --check`, strict native Clippy on the affected supported targets, and the supported WASM compile/check for the `page` feature using the current lockfile/toolchain.
- Confirm the three existing shell call sites and all relevant context/action wiring by source review; inspect the diff for unchanged markup and moved numeric edit lifecycle.
- Do not use the shared `web/target` during compilation unless this worktree is the sole compiler, as instructed by the coordinator. No `dx` build or browser run unless separately assigned.
- No new refactoring takeaway observed from the contract/source inspection; finalize this after implementation/review in `/tmp/frontend-run/int1-handoff.md` and the required RF record if the integrated handoff calls for it.

## Review request

Astra High: review this concrete private seam and the actual same-page-crate reachability, module visibility, and single Runtime/Session authority before source edits. Identify any contract change required before implementation. Implementation remains blocked on this contract review.
