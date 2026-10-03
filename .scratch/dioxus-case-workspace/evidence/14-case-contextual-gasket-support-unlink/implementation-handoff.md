# Case gasket support unlink handoff

```yaml
packet_id: case-contextual-gasket-support-unlink-14
branch: codex/case-contextual-support-linking-20261003
base: 8a644d01
reference_commit: 5a472a9426e6e38993361da402cd4ec730feb369
reference_source:
  toolbar: app/src/ui/AssemblyScene.tsx::unlinkGasket and Edit gaskets toolbar
  inspector: app/src/ui/MechanicalAssemblyPanel.tsx::selectedSupport resize
reference_behavior: The viewer's Edit gaskets toolbar exposes `Unlink selected support` for the active support. The handler marks the selected support and its pairId-referenced partner unlinked. Resizing only mirrors to the paired row while both are linked. This is reflected-support pairing; upper/lower pads remain the same support geometry.
reset_behavior: Reset gasket placement is a separate Gasket settings Inspector action; it is not coupled to unlink.
browser_pin: New own agent-browser session `case-link-react-webgpu-20261003`, React5173, imported the preserved original5b fixture SHA-256 5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df, configured Gasket mount, and observed 17 pairs. The preview settled, but React disabled Edit gaskets in this session because the renderer did not enter its ready state, so no browser unlink click is claimed. Exact action/map semantics are pinned by the reference source above.
mount: Case shared viewer toolbar; current generated support selected through existing Case layer selection. MechanicalSettingsProps is passed from the editor's existing mount.
edit_owner: MechanicalSettingsPatch::SetGasketSupportUnlinked through the existing scoped request/controller and canonical/instance mapping.
checks_before_freeze: rustfmt --edition 2024 on changed Rust files; git diff --check (pass).
compile: cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings; pass, exit 0, using coordinator-assigned shared target `/home/chris/.local/share/boardstudio/worktrees/case-current-result-status-repair-20261003/web/target`.
review: consolidated Sol review on the integrated candidate; no separate source-review handoff.
rf_disposition: No new refactoring takeaway observed in this bounded packet; preserve RF-001 shared presentation composition and RF-006 physical-instance/canonical scope history.
parent_gates: F7.2/F7.4/F7.3/F7.5/F7.8 and canonical 62-parent graph remain open; Issue07 retains full gesture acceptance and consumes this action.
```
