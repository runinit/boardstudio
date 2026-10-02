# INT.1 preimplementation contract review

Reviewed in the exact worker `/home/chris/.local/share/boardstudio/worktrees/frontend-int1-20261002`, clean at `f3bb02a3d0eb15cf01822ed29d9b1632048df9c2`. Read the proposed contract, T1-01, relevant CONSTRAINTS/model-policy requirements, and the affected page source/call sites. No repository edits, builds, browser checks or agents were performed by this reviewer.

**Decision: approve implementation of the corrected direct-child module shape below. The submitted `presentation/private/` re-export shape requires this narrow correction before edits.** This is contract approval only; integrated source and affected checks remain required before INT.1 acceptance.

## Required placement correction

Declare `mod library; mod objects; mod inspector;` privately in `presentation.rs`. Use `web/src/presentation/library.rs`, `objects.rs`, and `inspector.rs`. Each extracted component entrypoint is `pub(super)` and is brought into the parent with an ordinary private `use` (or referenced through its private module). Keep all Inspector-only state/helpers private inside `inspector.rs`. Do not create the additional `private/mod.rs` umbrella or its re-exports.

In the submitted nested shape, `pub(super)` in `presentation::private::library` grants access only through the `presentation::private` scope. A `pub(super)` re-export from that intermediate scope attempts to expose the restricted item to `presentation`; it does not make the original item more visible. The direct-child shape removes both the unnecessary layer and this reachability problem. It retains the same effective presentation-subtree accessibility as the current presentation-local component functions. No library export, Runtime member, public type, or new crate-level API is needed or approved.

`main.rs` already declares `presentation` and `runtime` in the page binary. Descendants of `presentation` can use the existing page Runtime and existing ancestor helpers. In particular, Library may call `super::close_project_menu` without widening that helper. The `boardstudio_web` library boundary is not crossed by this extraction. Keep `main.rs`, `runtime.rs`, library exports and manifests unchanged.

## State and behavior contract approved

- `App` keeps the only `use_hook(Runtime::new)`, authoritative Session, root `Signal<u64>` provider, subscription and root unsubscribe/Close teardown. Extracted components continue consuming the same existing Runtime and signal contexts. They do not create a new runtime, subscription, provider or writable document/read-model authority.
- Move the existing Library implementation and its hooks/actions unchanged. Preserve its two current mounts (project menu and landing page), existing list query/reactivity, fixture/saved/import actions, close-menu behavior and file-input reset. This packet does not silently repair the separately tracked library race or demo-copy behavior.
- Move Objects unchanged, preserving accepted-board projection, real selected IDs, board/physical-instance navigation, Replace selection action, listbox semantics, stable part keys and `m1-object-{index}` focus targets.
- Move Inspector together with `NumericEdit`, `KeyboardHandler`, `submit_position` and `commit_numeric`; the source inspection confirms those helpers are only used by Inspector. Keep the component's own signals/hooks and their order, preview transaction/revision/start coordinates, target-change effect, Escape restore, Enter/Apply commit, invalid-number behavior and pending-preview teardown exactly as they stand. Keep the selected target lookup and existing transaction IDs/actions unchanged.
- Preserve existing component identities/call-site positions and conditional mounts. In particular, Inspector remains under the existing Layout-only branch; compact-open signals and surrounding panel wrappers remain in Editor. Add no wrapper DOM, component keys, props that alter remounting, or replacement context layer. Preserve all markup, labels, IDs, classes, event closures and action wiring apart from necessary imports/module paths.

This is the smallest coherent INT.1 prefactor. Reusing existing contexts is sufficient here; introducing new read-model DTOs, callbacks or a general component framework would widen the change unnecessarily.

## Acceptance checks after extraction

The worker must compile the actual WASM page target to prove module and Dioxus macro reachability. Native checks alone cannot prove these modules compile because `main.rs` gates presentation on `target_arch = "wasm32"` plus the `page` feature. Formatting and the established affected strict checks also remain required under coordinator-owned build scheduling.

Integrated review should compare moved bodies, hooks and call sites directly, then use the coordinator's exact candidate public evidence for shell/layers/Footprints/selection/drag/Undo. The reported F3a baseline remains reusable for the traces it actually covers; it does not become evidence for unperformed traces. Because Inspector draft code moves, acceptance must have affected public numeric preview→Escape, preview→Enter/Apply→Undo, target change, and Layout teardown coverage from current or demonstrably applicable evidence. The shared check gates and existing AT/resource limitations remain unchanged.

No new refactoring finding beyond existing RF-001/RF-002 was observed. The avoidable intermediate module reinforces the existing same-crate-placement mitigation; it does not justify a new architecture effort.

## Reviewed SHA-256 snapshots

| File | SHA-256 |
| --- | --- |
| `/tmp/frontend-run/int1-contract.md` | `8dcaaa11dbd69683ff65fada1b6c254140dc69a2b22fa60a50d0d25cb48b7b40` |
| `/home/chris/.local/share/boardstudio/worktrees/frontend-int1-20261002/web/src/presentation.rs` | `c62ef7152cc62b3603fd30ee9ef5c576da1e4c14e44df7689b0bb2a639e5e46c` |
| `/home/chris/.local/share/boardstudio/worktrees/frontend-int1-20261002/web/src/main.rs` | `a4ed7efb4f593c9a50ee802511a7aae27896df734804496f234644eb13494ed0` |
| `/home/chris/.local/share/boardstudio/worktrees/frontend-int1-20261002/web/src/runtime.rs` | `a31deaa5a6aa124f716ac22f6f3ea6604e26fc0ce52d04d7277b8cf7be9ae34f` |
| `/home/chris/.local/share/boardstudio/worktrees/frontend-int1-20261002/.scratch/dioxus-frontend-tranche-1/issues/01-private-workspace-composition.md` | `9d19e763a31635ba8be9319c40b47a64513a316517314fc26258f8115a317da6` |
