# F3.3 Layout background grid dots

**Reference:** React `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/Workbench.tsx` around the Layout canvas grid definition. **Dioxus source base:** `81681302c1b4e771d65ca1e8fff467389b164e6e`.

React always paints the canvas background dot pattern, even when grid snapping is Off. In normal Layout it uses `PITCH_MM / 2` (9.525 mm for the pinned 19.05 mm pitch) and a 0.12 mm radius. While the outline point editor is active, it uses the current outline snap step with a 0.1 mm minimum, rounds that step up to at least eight screen pixels, and scales the radius to 0.7 screen pixels. The pattern and its bounds are drawn before the Y-flipped geometry group, with `--wb-grid-large` as the theme-aware color.

Dioxus now projects the same normal/editing spacing and radius from its existing accepted matrix pitch, root-owned snap preference, outline edit projection and measured canvas width. The background remains visible with snap Off, and the rectangle shares the SVG view bounds without changing camera, document, or history state. No new controls, snap policy, or geometry algorithm were introduced.

**RF handoff:** No new refactoring takeaway observed. Preserve the existing RF-001 single-Editor ownership boundary and RF-005 geometry-policy boundary. This is a source-confirmed visual parity repair only; it has not been publicly qualified in this receipt.

**Qualification:** Source implementation and formatting/diff checks only. The assigned workflow explicitly deferred browser and build work; this receipt does not claim a paired public-browser result or complete F3.3/F3 acceptance.
