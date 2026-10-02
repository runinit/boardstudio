# Layout layers, keycaps and footprint view restored

The requested F3a correction is implemented on `codex/rust-v1-ui-parity-20261001`.
Executable source: `f44a3d1b5c1f42786ae0b416db10c0d6e5bebcb3`.
Build: `frontend-layout-f44a3d1b-20261001`.
Editable demo: [root](http://127.0.0.1:34643/) and
[subpath](http://127.0.0.1:34643/boardstudio/). Choose REVIUNG41 from the project
library if the browser has no saved project. Owned server PID1052674.

## What changed

- Layout draws rounded keycap outlines and inset faces by default, using part
  overrides, definition sizes and matrix pitch/edge-gap fallback. Empty projected
  cells use the matrix definition, matching the pinned React lookup.
- The floating Layers control independently switches Keys, Components, Keycaps,
  Footprints and Board. The toolbar Footprints control shares the same state.
- Footprints show real pad/drill shapes and the retained recipe's graphics/text,
  including part overrides, rotation and back-side reflection. Dioxus owns the
  SVG and controls; the unchanged generator remains a documented service.
- Cap hit areas, selected appearance and existing edits/Undo remain functional.
  Visibility and workspace changes do not create document/history writes or
  move the camera. Compact controls have 44px targets and Close/Escape focus return.

## Verification

The original browser reproduction failed with no Layers, no Footprints and zero
keycap outlines. Final public browser regression passes all five switches,
shared Footprints state, unchanged revision/camera, selected cap edits/Undo,
workspace retention, both themes, compact bounds and keyboard dismissal/focus.
REVIUNG41 matches React exactly: 41 caps, 193 copper pads, 238 drills, 309 graphics.
An imported 27×18mm cap and the unobscured edge outside its switch courtyard pass;
back-side parts retain their reflection. Both Sofle boards show 29 caps and 70 parts,
with independently scoped geometry. Root and subpath render generated footprints
while offline; saved Layout restoration passes.

[Final raw browser records](browser-f44a3d1b/steps.json),
[override/offline/multi-board records](additional-f44a3d1b/steps.json),
[scoped offline reopening](subpath-f44a3d1b/browser-steps.json),
[root offline graphics](root-offline-f44a3d1b.json).
[Reusable driver](browser-qa.py) and [additional driver](additional-qa.py) retain
the tested public workflow. The copied override fixture retains its assets.
Initial driver failures were incorrect pad-count/selector assumptions, an edit
wait that observed the previous Saved state, and a click inside an overlapping
neighbor. The corrected driver waits for actual geometry and tests the unobscured
cap edge; none required changing application behavior.

Formatting, WASM check, strict Clippy, 12 native web tests and 2 new projection tests
pass. Repository tests 7/7 and repository reachability checks pass. Exact commands
are in [checks](checks/commands.json). The maintained build passes all 7 commands
and verifies 930 source hashes; provider bytes are reused only
after verification against the maintained M1 build. Both offline inventories
include the retained generator service and dependencies. [Provenance](build-provenance.json).

[Standards](standards-final.md) and [Spec](spec-review.md) reviews have no remaining
source findings. [Fresh visual review](finish-review.md): ship for F3a.
[Design handoff](design-documentation.md) is reflected in DESIGN.md; detector
findings are advisory. Axe has zero violations in desktop dark, desktop light,
compact, and the final source check. SVG text contrast is incomplete; actual
assistive-technology interaction remains unavailable and is carried forward.

## Evidence reuse and remaining work

The reviewed light/dark desktop/compact captures are at 7268628d under
[browser-7268628d](browser-7268628d/desktop-dark.png). The only later executable
change selects the matrix definition for a projected cell without a member;
these fixture captures contain members and are visually unchanged. Final source
was rebuilt and its public behavior/offline paths rechecked without another
visual-polish round. The original React captures and missing-feature review
remain beside this handoff.

This closes the requested missing-function correction, not full F3 or frontend
v1. Remaining work includes the complete Layout tree/inspectors, authoring tools,
constraints/snapping/outline editing, 3D assembly, and the other F2–F9 workspaces.
No whole TSX file is claimed fully ported from this slice. No public Rust API,
saved format, backend recipe, production entrypoint or React retirement changed.
The original checkout remains at 5a472a94 with its pre-existing untracked scratch
content and both stash identities intact. Prior M1 acceptance limits remain open.
