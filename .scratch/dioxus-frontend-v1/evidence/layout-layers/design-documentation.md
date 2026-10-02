# F3a design documentation handoff

## Scope and evidence

Documenter pass for the scoped F3a Layout restoration at `7268628d`. The supplied final review says the slice is ready to ship; the detector findings are advisories. The existing world in `PRODUCT.md` and `DESIGN.md` remains the source of visual identity. The scoped issue and the new F3a architecture section agree that this work restores Layout layers and footprint rendering while full F3 and F7 work remains open.

Checked the existing design document against `web/assets/m1.css`, `web/src/presentation.rs`, `web/src/presentation/footprint_graphics.rs`, the scoped F3a issue, and `docs/architecture.md`. The supplied browser evidence covers 41 caps, 193 pads, 238 drills, 309 footprint graphics, independent layer behavior, shared Footprints state, cap edit/Undo, overrides, back mirroring, hit testing, offline loading, and Sofle boards. The final review also records zero axe violations in three states, with SVG contrast incomplete and actual assistive-technology testing unavailable.

## Narrow design-truth update

`DESIGN.md` already describes the keycap overlay, geometry colors, and a floating Layers control. Extend only its “Drawing and footprint workspace” paragraph so it reflects the actual F3a control behavior:

> The Layout canvas draws the rounded keycap envelope and inset top face by default, using resolved key dimensions and the existing geometry and selection roles. Its floating Layers control groups independent visibility switches for Keys, Components, Keycaps, Footprints, and Board; all start visible except Footprints. The toolbar Footprints switch shares that view-only preference. Layer preferences remain presentation state across workspace changes and do not create document revisions or history entries. The control overlays the canvas without changing its bounds or camera; in compact canvas widths the list uses 44px targets and a Close action, while Escape closes it and restores focus to the trigger.

Keep the existing workbench world, theme tokens, and geometry roles intact. Do not expand this paragraph into general 3D assembly behavior: F3 and F7 remain open, and the architecture document keeps Parts/PCB/3D work in its separately scoped follow-ups. No frontmatter or `.impeccable/design.json` change is warranted; the one-off floating control is not evidence for a reusable token or component rule.

## Five-line system summary

- Palette: light and dark neutral paper/panels around a slate canvas; action blue, violet secondary emphasis, teal board geometry, blue keys, and violet pads.
- Type ramp: bundled Source Sans 3; 23px inspector title, 13px section/body, 12px fields/supporting text; Source Code Pro for measurements and metadata.
- Named rules: Geometry Role Rule keeps canvas, board, key, pad, and selection roles distinct.
- Named rules: Stationary Controls Rule anchors scope and snap controls while geometry moves; Disclosure Rule reveals optional inspector settings in place.
- Named rules: Structural Shadow Rule reserves offset shadows for transient overlays and floating controls; the existing paired-foreground and selection rules remain unchanged.

No repository design files changed. Handoff written to `/tmp/layout-design-documentation.md`.

Defects or drift not canonized or repaired: the supplied detector flags task-specific shadow and radius values on the floating Layers control; these remain local implementation details rather than new system tokens. Contrast evidence is incomplete for SVG and actual assistive-technology review was unavailable, so this pass makes no general accessibility or contrast claim.

Execution: a fresh generic sub-agent applied the Impeccable documenter instructions; no role-specific documenter tool was used.
