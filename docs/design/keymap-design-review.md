# Keymap design review

The Keymap workspace extends the existing Keyboard Lab workbench. It preserves the selected light/dark theme, role colors, typography, inspector disclosures, modest corners and selection treatment. Layers precede the selected-key behavior editor; Keys, Macros and Encoders share the inspector. Keycaps has its own workspace. DYA and ZMK supplied interaction references without copied implementation.

Reviewed the incumbent [PRODUCT.md](../../PRODUCT.md), [DESIGN.md](../../DESIGN.md), `.impeccable/design.json`, the [surface brief](keymap-workspace.md), and the implemented Keymap panel and stylesheet. No global design system or token changes were needed or made.

Rendered evidence uses a real Corne project with a selected mod-tap key:

- Desktop: [desktop capture](evidence/keymap/desktop.png), 1280 × 900; layout selection, Base layer, editor navigation and selected-key inspector are visible.
- Narrow: [narrow capture](evidence/keymap/mobile.png), 390 × 844; the inspector uses the existing drawer and bounded scrolling. Browser measurements reported no horizontal overflow and 44 px input/select heights.
- Design detector: `/tmp/keymap-design-detect.log`; command exited successfully with no output. This is a detector result, not a complete accessibility audit.

Reviewer disposition: **ship the bounded selected-key/layers extension on desktop and the narrow inspector**. Macros, encoders and Keycaps were inspected in source; their rendered states were not certified by this visual review. Offscreen behavior parameters, alternate themes, all interaction states and broader application accessibility remain outside this evidence.

Existing system documentation was preserved. No unreviewed surface, incidental spacing or pre-existing drift was promoted into a global rule.
