# Accessibility component audit (old maintained release)

This is component evidence for source `050e9282e076628b411db99e9def21677616a0f4`, build `m1-release-20261001-050e9282`; it predates the reviewed export-registration cleanup fix and is not final M1 acceptance. Browser: Chromium via agent-browser session `m1axe050e-7dd31abc1fc4`; axe-core 4.12.1 embedded in agent-browser. Captured root/subpath library, editor, and case-error states at 390×640; root editor also at 1280×720. Raw result JSON, paired screenshots, and focus style probes are retained beside this file. The existing React reference app ran from Vite at `http://127.0.0.1:5174/` at the same 390×640 viewport.

## Raw results

| Candidate state | Passes | Violations | Incomplete |
|---|---:|---:|---:|
| M1 root library, 1280×720 | 30 | 0 | 0 |
| M1 root editor, 1280×720 | 37 | 0 | 1 (`color-contrast`, 10 SVG text nodes) |
| M1 root editor, 390×640 | 37 | 0 | 1 (`color-contrast`, 10 SVG text nodes) |
| M1 subpath library, 390×640 | 30 | 0 | 0 |
| M1 subpath editor, 390×640 | 37 | 0 | 1 (`color-contrast`, 10 SVG text nodes) |
| M1 subpath case-generation error, 390×640 | 39 | 0 | 1 (`color-contrast`, 10 SVG text nodes) |
| Reference library, 390×640 | 31 | 2 | 0 |
| Reference editor, 390×640 | 34 | 1 | 2 (SVG contrast and prohibited ARIA attributes) |

Reference library violations: `heading-order` on “Your keyboards” (`h3` after the page menu hierarchy), and `label` on `.wb-project-file-input`. Reference editor violation: `page-has-heading-one` on `<html>`. Its incomplete ARIA checks point at `.wb-matrix-ghost` SVG groups; raw details remain in the corresponding JSON. These reference findings are recorded for comparison, not treated as M1 failures.

## Manual contrast and focus follow-up

M1's `color-contrast` incomplete nodes are SVG scene labels (`text.m1-part-label`) for board objects. Computed foreground is `rgb(24, 35, 49)` (`#182331`) on white object polygons (`#ffffff`), giving an sRGB contrast ratio of approximately 15.87:1. This exceeds WCAG AA text contrast. The labels remain visually small at the 390px viewport; that is a legibility/zoom concern separate from the computed contrast result. The generated-case canvas has an accessible name, but this old release rejects the CAD result before producing geometry, so there is no rendered 3D scene to visually inspect. Canvas pixel contrast is not measured by axe.

M1 first-tab focus on the “REVIUNG41 copy” button shows a visible 2px solid `rgb(56, 88, 214)` outline with 3px offset. The reference app's “Project” button shows the same 2px color outline with 2px offset. The paired short-viewport captures are `screenshot-subpath-editor-390x640.png` and `screenshot-reference-editor-390x640.png`; the M1 error state is `screenshot-subpath-case-error-390x640.png`.

## Screen reader

`orca` is unavailable. `speech-dispatcher`, `spd-say`, and `espeak` exist, but they are not an interactive screen reader. No spoken screen-reader pass is claimed. Repeat raw axe and visual/canvas checks against the corrected source release; use a genuine assistive technology run when one is available.
