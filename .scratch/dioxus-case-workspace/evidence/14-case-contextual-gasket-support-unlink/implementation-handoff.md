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
browser_pin: Own agent-browser session `case-unlink-react-public-20261003`, React5173. Imported untouched original5b fixture SHA-256 5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df; configured mechanical stack to Gasket mount; waited until preview geometry was current and 17 pairs rendered; expanded Gaskets and selected `Gasket 1 · Left 70 × 3 mm`; entered the viewer's `Edit gaskets` context and clicked `Unlink selected support`. React's returned archive stores exactly one explicit anchor override (`left-keys-layout:0`, length 70, width 3, `unlinked=true`); the other reflected pairs remain generated. Undo removes the explicit override (the saved layout supports list becomes empty, representing the generated linked baseline); Redo restores the same override. Browser reload retained the Gasket configuration and selected support state; a new portable copy after reload is byte-identical to the post-Redo copy. Captured archives and screenshot are in this directory. This is React reference confirmation; the Dioxus integrated-candidate action remains pending coordinator's served-ready signal.
mount: Case shared viewer toolbar; current generated support selected through existing Case layer selection. MechanicalSettingsProps is passed from the editor's existing mount.
edit_owner: MechanicalSettingsPatch::SetGasketSupportUnlinked through the existing scoped request/controller and canonical/instance mapping.
checks_before_freeze: rustfmt --edition 2024 on changed Rust files; git diff --check (pass).
compile: cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings; pass, exit 0, using coordinator-assigned shared target `/home/chris/.local/share/boardstudio/worktrees/case-current-result-status-repair-20261003/web/target`.
review: consolidated Sol review on the integrated candidate; no separate source-review handoff.
rf_disposition: No new refactoring takeaway observed in this bounded packet; preserve RF-001 shared presentation composition and RF-006 physical-instance/canonical scope history.
react_receipts:
  after_unlink: react-configured-after-unlink.boardstudio (SHA-256 54f48300593db919bc37e827ac81e4e12588acc88a960c8400acd8c947f1d62c)
  after_undo: react-configured-after-undo.boardstudio (SHA-256 92b7899d7f8bd0cbaf364def57629ec91ce44c9ee9345f44176e756ae18e5053)
  after_redo: react-configured-after-redo.boardstudio (SHA-256 df38eec77d8cbd2953a8a01449b41819ec8c79638d190fd14dfc7f47e7f6de38)
  after_reload: react-configured-after-reload.boardstudio (same SHA-256 as after_redo)
  after_reload_screenshot: react-after-reload.png
candidate_receipt:
  integrated_source: a76fa2bdee3379be1d9d2c2133f9428a87a75fb0
  served_candidate: http://127.0.0.1:34748/boardstudio/
  provenance_sha256: 7e764747832a5ce20ba2c9091938eeeaee06283583a9071a89d8321c6dc88299
  profile: case-unlink-dioxus-34748-20261003
  reopened_profile: case-unlink-dioxus-reopen-34748-20261003
  result: Passed bounded UI action, Undo/Redo, portable save, and import into a fresh profile. The same original5b input was used. Gasket 1 · Left 70 × 3 mm was selected; toolbar action became disabled after applying and the Case viewer plus Inspector reported `This gasket is unlinked from its pair.` Undo reported `This support is linked to its mirrored pair. Matching upper and lower pads resize together.` Redo restored unlinked feedback. The saved archive contains the scoped left-instance gasket configuration and anchor `left-keys-layout:0`, anchor 0.9220773021477031, length 70, width 3, `unlinked=true`; importing that archive into a fresh browser profile retained the same UI feedback and 70 × 3 dimensions.
  archive: dioxus-candidate-a76fa2bd-after-redo.boardstudio (SHA-256 0668acee93bd2ab1340b80566bbba1d1b45f2c0b16b3cb3821056e07f60baedd)
  undo_screenshot: dioxus-candidate-a76fa2bd-after-undo.png
  redo_screenshot: dioxus-candidate-a76fa2bd-after-redo.png
  reopen_screenshot: dioxus-candidate-a76fa2bd-after-reopen.png
parent_gates: React reference and Dioxus integrated-candidate unlink/history/reopen journeys passed. Consolidated Sol review remains with coordinator; F7.2/F7.4/F7.3/F7.5/F7.8 and canonical 62-parent graph remain open; Issue07 retains full gesture acceptance and consumes this action.
```
