# Reset gasket placement source pin

The React 5173 browser session `case-unlink-react-public-20261003` used the preserved original5b archive (SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`) and its accepted Gasket mount. In the Case Inspector, `Edit gasket supports` opens the `Gaskets` section. The section shows the existing `Reset gasket placement` control and the hint: “Reset releases manually positioned supports. Closure positions stay fixed.” The current Gasket support tree shows 17 pairs.

Pinned React source: `app/src/ui/MechanicalAssemblyPanel.tsx::GasketSettings`, where the button calls `update({ gasketLayout: { ...layout, supports: [] } })`. The shared `update` helper preserves the accepted gasket settings while applying its existing gasket-compatible configuration defaults. The reset must keep closure positions unchanged. It is separate from Issue14's selected-support unlink and Issue07's pointer-drag gestures.

Screenshot: `react-reset.png` (SHA-256 `15d7f15dc6c06962dca53316f4776751edd2b393f9923c6295059fc2b680fb44`). The click was not used during reference capture, so the fixture's saved/unlinked state remained available for Issue14 evidence.

This source pin bounds this packet. Dioxus implementation is in the isolated worktree `case-contextual-plate-inspector-20261003` on base `dd7697ed99864040697c905b3d0e28209301d11b`:

- `web/src/presentation/mechanical_settings.rs` adds the `ResetGasketPlacement` intent and exposes the exact action/hint in the existing Gasket settings surface only when the matching Gasket configuration is present. The control is disabled by the existing non-editable owner state.
- `web/src/presentation/mechanical_settings_controller.rs` applies the request through the existing accepted-scope controller, rejects a changed/non-Gasket owner, clears only `gasketLayout.supports`, and skips unrelated process normalization for this patch.

Formatting and `git diff --check` pass. The assigned page-only strict Clippy compile and the integrated Dioxus journey are pending coordinator target/build availability. No new routine UI test or broad suite is added. RF disposition: no new refactoring finding; preserve RF-001/RF-006.
