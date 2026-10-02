# RF handoff — first Keymap composition/outline slice

Reviewed scope: `presentation/keymap/panel.rs` editor-tab composition and `presentation/keymap/canvas.rs` rendering of the accepted board contour. Reference trace and implementation contract: `/tmp/frontend-parity-reset-20261002/keymap/AUDIT.md` and `/tmp/frontend-parity-reset-20261002/keymap/first-parity-contract.md`.

The source comparison supports existing RF-001: `Editor` remains the shared composition and lifecycle root, while the new feature-owned workspace slot and existing Keymap panel can own Keymap-only visible composition. This slice adds no new architectural/refactoring finding and does not demonstrate that the shared hotspot is resolved. Preserve RF-001 as the existing finding; do not create a duplicate RF. Root-owned `POST-PORT-REFACTOR.md` and `refactor-findings.json` reconciliation remains with the integration owner.

Author verification limits at this handoff: source is in the isolated feature worktree, has not yet received the root's accepted contour/bounds mount update or fresh browser build. The old frozen public candidate produced the retained red baseline (no `Keymap editors` group and no Keymap outline polygon). Fresh build/public green, dirty-field blur parity, browser focus/AT, and root/subpath acceptance remain unverified and required before acceptance.
