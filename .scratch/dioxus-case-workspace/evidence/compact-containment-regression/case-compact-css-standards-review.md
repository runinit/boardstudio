# Compact Case CSS — Standards review

Reviewed integration `web/assets/m1.css` SHA `e5e310b22e74c10f0edc3ff15d90a7417c5cb7c082bfa1c59d9ba129a64d07d6` against `9ef5bc5cad09ab3064711b45fc348b4cc09a74d4`, plus existing panel toggles and inert rules.

**P2 — Both compact panels can overlap while remaining focusable** (CSS lines439–443; presentation.rs1920–1922). The new rules put Objects and Inspector in the same grid cell, aligned to opposite edges with equal z-index. Below1150px the inherited widths are220px and300px; at390px they overlap130px. Both visibility buttons toggle independently, so opening both leaves the later-painted Inspector over Objects controls. panels.rs only hides/inerts a compact-closed panel; covered Objects controls remain in keyboard order. This introduces obscured focus and controls, contrary to CONSTRAINTS.md UI accessibility/focus visibility and responsive action reachability. Use a Case-only single-active-panel policy or a bounded layout that prevents overlap while preserving existing visibility semantics elsewhere. Cover both-open keyboard focus and panel transitions in the public regression.

The viewport containment change otherwise follows existing grid/flex/token conventions:100svh constrains Case, the viewport row has minmax(0,1fr), compact navigation and footer retain separate flex space, and panels receive bounded scroll surfaces. Selectors require Case and max-width760px; macro styles and other workspaces are unchanged in this diff. No host API change.

No browser or compiler checks performed. Reported prior heights and temporary diagnostic CSS are diagnosis evidence, not acceptance of this source. Full responsive/focus/nav/footer verification remains open.

## Corrected delta — P2 closed

Re-reviewed CSS SHA `ed3fcce8677e261167f247acf5304205f309cca62b593444d934bd089a0da009`. The both-open Case selector creates two minmax(0,1fr) tracks, gives each panel its track width, and places Inspector in column2. Workspace spans both tracks underneath. This removes panel-to-panel overlap while preserving existing independent visibility signals and the one-open preferred widths; closed-panel inert/display behavior remains unchanged. No new material source finding. Narrow layouts now rely on existing bounded panel scrolling for long content; root must verify actual control reachability and visible keyboard focus at supported widths/zoom, navigation/footer and closed/one-open/both-open transitions in the built page. Temporary probe is diagnostic only. No source edits, Cargo or browser verification performed.
