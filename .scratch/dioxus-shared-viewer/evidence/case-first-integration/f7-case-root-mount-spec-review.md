# F7 Case root mount Spec re-review

**Decision: root mount source clears; no remaining material findings in this reviewed slice.** Actual dirty source reviewed against the root mount contract and Case-first issue02.

Color alias read/set/reset and Reset availability now use the actual consumed `case_display.rs` helpers. `has_color` checks every expanded alias, so Models-only and upper-gasket-only saved preferences remain resettable. Picker values use the reference first-alias policy, set/reset covers all aliases, and unrelated colors/visibility are preserved. The previous P2 is cleared.

Inspected root's regression logs: extracted old raw-key behavior failed the two intended PCB/gasket assertions (`None` versus alias color); latest corrected native run passed 12 library + 12 page tests, including the partial-alias regression. These are reported root evidence, not checks executed by this review.

Shared authored body selection now derives viewer highlighting only when Scope and mesh membership match; PCB/non-authored selection clears the body context. Fresh emitted selection/display callbacks retain owner identity, Scope, accepted token, exact scene Rc and effective-instance gates. Authored-body forwarding verifies captured current-board document membership and never changes Session part selection. Theme resolution, persistence key/in-memory fallback, existing Case generation/forms and private host registration remain sound.

Reviewed SHA-256:
- presentation `096a9d31a8cddc6c430d71ee0cb4623762502aceaa9485b897276c2131be4c11`
- CasePanel `7e20747d6c3d4b131af47586eb9d6bf6559d1f3205270d98a9a7beba8ff41bb4`
- main `f16ce4c02ac6de23ff286feae1b26046d29f7cefb5a975acab71dd8a915994be`
- case_viewer `ed9f6b2e92659a55cb90f96de328a763c8d19d49d9428631e7af53c99bce8d88`
- case_bodies `f848ddaf68c1e745440bafd34206a40c386940eb83ce7b1a773cdaa8252ae68f`
- shared_viewer `22bdd16a07b7353a915e064da29c737e3126c9ba0533372a5ee79476ea0cf1a9`
- case_display `516cdb43a5450a56892007cd0cc4fa77bceba441ff89d68abdc6f96f3e29baa3`

Narrow slice remains contours/thickness, current CAD meshes/optional stack and viewer controls. Populated surfaces/holes/models, battery/reference overlays, handles/manipulation, mechanical Inspector forms, prepared/model capability integration and five-consumer parity remain open. No build/public-browser, error/cleanup/stale-event, accessibility, INT.2/BND.1 or F7.3 parent acceptance inferred. No source edits or Cargo performed.
