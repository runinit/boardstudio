# T1-07 desktop panels author handoff

Author branch: `codex/frontend-panels-20261002`, based on `598b2c026941130f9b95ff4fee57353f8eefcb0d`, owned implementation commit `ba15d0f2`, then normal merge of integration tip `cb8203fcf243eee36352f1d30149c7e861401f97` (`3af05c2c`). The checkout is clean. The owned commit contains only `web/src/presentation/panels.rs` and `web/src/presentation/panels/policy.rs`.

## Owned module

`panels.rs` exports private `PanelSide`, `PanelMode`, `PanelSettings`, `use_panel_settings(side)`, `ObjectsPanel`, and `InspectorPanel`. The Editor must invoke the settings hook for each side before its early return and pass the returned `Signal<PanelSettings>` to wrappers, as specified in `/tmp/frontend-run/panels-contract.md` and reviewed in `.scratch/dioxus-frontend-tranche-1/evidence/contracts-active/panels-contract-review.md`. Durable preference settings live above conditional Inspector mount. Each wrapper keeps transient reveal/menu/hover/focus state local and wraps children without touching Runtime/session state.

The module independently decodes and clamps mode/width, persists only `{mode,width}` under the reference keys, implements exact reference accessible strings/group semantics, 280ms reveal behavior, pointer/focus checks, Escape/focus restoration, outside pointerdown cleanup, media query reactivity across 760px, and timer/listener cleanup. The compact rails are not rendered; the outer compact shell uses the existing compact open signal with `inert` and `aria-hidden`. The requested root-owned parent/CSS proposal is `/tmp/frontend-run/panels-shared.patch`; root should integrate/tune it in `presentation.rs` and global CSS.

## Verification performed

- `rustfmt --edition 2024 --check web/src/presentation/panels.rs web/src/presentation/panels/policy.rs` — passed.
- `rustc --edition 2024 --test /tmp/frontend-run/panel-policy-harness.rs -o /tmp/frontend-run/panel-policy-tests && /tmp/frontend-run/panel-policy-tests` — 4 passed, 0 failed. Standalone harness emitted only dead-code warnings because module consumers are omitted.
- Temporary isolated Dioxus crate check on wasm target passed with Dioxus 0.7.10: `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target cargo check --manifest-path /tmp/frontend-run/panel-compile2/web/Cargo.toml --locked --target wasm32-unknown-unknown --bin boardstudio-web`. The temporary mount copied the owned module and added `mod panels;` only; it emitted unused/dead-code warnings because the parent hook/wrappers were not yet integrated, but no compile errors. Root integration must rerun the affected checks against the full crate.
- I did not run dx/provider build, browser comparisons, canvas/domain neutrality checks, or public-root build; those remain root-owned gates.

## Integration note

When root adds `mod panels;` to `presentation.rs`, the `#[cfg(test)]` policy tests become part of that private module tree and are discoverable by the applicable crate test target. The temporary `rustc` harness is independently reproducible in the current state. Parent-owned wrapper calls need to match the props above. Preserve the existing compact button IDs/`aria-controls`, apply mode/width CSS to the parent grid (not only child CSS vars), keep the rail outside the inert region, and ensure absent Inspector has deliberate grid/overlay disposition. No new refactoring takeaway was observed; existing RF-001/002 cover the shared composition seam.
- Strict isolated WASM Clippy on the updated temporary mount passed: `CARGO_TARGET_DIR=/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target cargo clippy --manifest-path /tmp/frontend-run/panel-compile2/web/Cargo.toml --locked --target wasm32-unknown-unknown --bin boardstudio-web -- -D warnings -D clippy::all -A dead_code`. `-A dead_code` is limited to this temporary mount because parent integration is intentionally absent there; dead-code validation belongs to the full integrated build/Clippy run.

Follow-up fix commit `95534a87` resolves the redundant-local and type-complexity Clippy findings by removing a redundant capture binding and naming the listener types. The shared compiler target has been released.
