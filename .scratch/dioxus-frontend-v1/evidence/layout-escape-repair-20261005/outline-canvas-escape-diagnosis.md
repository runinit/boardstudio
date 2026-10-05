# Fixed-outline canvas Escape diagnosis and prepared patches

## Cause and expected behavior

The perimeter inspector's `onkeydown` at `outline_lifecycle.rs:2167` handles Escape from its Done-button subtree. The focused point handle is a separate SVG canvas sibling, rendered by `OutlinePointCanvasOverlay` at `outline_lifecycle.rs:3307`; the inspector handler cannot receive its key event. The circle handler at `outline_lifecycle.rs:3365` only consumes Escape while a point drag exists. With no drag, Escape falls through to the arrow/delete dispatch and returns, leaving perimeter editing active.

The public reference Escape from the fixed-outline canvas point closes point editing but preserves the selected `OutlineVersion` (“Edited outline 1”) and returns to the ordinary Board outline controls with BODY focus. It does not select the Board Inspector context. The focused point circle unmount naturally returns focus to BODY. Preserve the existing active-drag Escape cancellation as the first branch.

## Prepared narrow fix and regression

The production patch keeps the active-drag cancellation path unchanged. For idle Escape it prevents default and propagation, validates that the selected scoped context still matches the Outline board and projection scope, then sets `projection.editing_points` to false. It does not change selected context or move focus programmatically.

The regression mounts the existing fixed-outline fixture and actual `OutlinePointCanvasOverlay` inside an SVG host. It opens “Edit perimeter points”, focuses the production `circle[aria-label='Outline point 2']`, dispatches bubbling Escape, then checks that Outline context remains selected, the point editor and canvas handles unmount, standard outline controls remain, BODY is focused, and the accepted document is unchanged. This exercises the production point-circle keyboard callback; it does not simulate success through a Runtime shim.

Prepared evidence-only patches are `outline-canvas-escape-test.patch` and `outline-canvas-escape-production.patch` in this directory. Their base hashes are respectively `80687d0b8451e84890f629bc106fba637faf71c7e5584e306df0add338feac3b` and `575ca1eb54a74d3ce8caa1c30bc48b38ba76d9918cb9d74ad49f751136dc4a48`. Patch hashes are in the receipt. Both passed `git apply --check`. No live source/test file was changed and no compiler was run; expected RED/GREEN remains pending source/compiler release.

## Public evidence and fixture barriers

The exact public checkpoints are in `.scratch/dioxus-frontend-v1/evidence/integrated-layout-c1d3-20261005/journey-checkpoints.json`, final Escape and settled checkpoints. The existing mounted fixture provides a ready Runtime, accepted fixed outline `fixed-outline-v1`, four points, selected Outline context, and m1.css. Only a synthetic SVG host/ref and default `CanvasInteractionArbiter` are needed to mount the production canvas overlay. No CAD, renderer, or external-asset barrier is involved for this focus/key route.
