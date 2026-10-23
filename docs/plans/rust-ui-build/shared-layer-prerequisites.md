# Shared layer prerequisites for 016 and 020

This continuation follows the landed [CONTROL prerequisite](shared-control-prerequisites.md).
It does not resolve either workspace ticket or change global acceptance. The coordinator owns
one completed Standards/Spec closure review and any later local feature landing; landing is held.

## Isolated source and preserved boundaries

- Clean feature/CONTROL base: `c38dc6bf4c52808663a7f5e78761bbfe99abb984`.
- Authorized source: `d0633057b7dc5698aa95cd5cf2cc68ee7e876eea`, parent
  `f11e6bd84456b94b2d7657f57a0d05b42184c4e7`, tree
  `b1713bc8a4c2bb02995e20ebfbd3c54a1a5da7c4`.
- Local cherry-pick: `7ac31403f`; exactly `canvas_layers.rs`, its example,
  `scripts/test-canvas-layers.py`, `web/assets/m1.css` and `web/tailwind.css`.
  No full 016 ancestry or workspace WIP was imported. The superseded four-file `69643366` slice
  omits required dock CSS and is not used.
- CONTROL Button/Input/Checkbox source, vendored renderer, manifests and lock remain unchanged.
  CanvasLayers visibility mutation, target mapping, public props and caller-owned callbacks remain
  unchanged. LayerPanel uses retained Collapsible content; each row uses Item and a controlled
  one-item ToggleGroup, with Tooltip on the eye control. Trigger and eye target minimums are 44px.
- Seven existing common-layer button selectors exclude copied data-slot controls. The only two
  tailwind changes add the Collapsible alternative beside the existing dock inline-list selector
  and exclude copied controls from the dock legacy sizing selector. Legacy selectors remain for
  other callers. The PartsPreviewLayers caller retains its own hidden set and toggle callback.

The browser harness now resolves build output and fixture output through `CARGO_TARGET_DIR`,
keeping the default repository target when unset, and uses a worktree-specific browser session.
This prevents stale/shared artifacts and collisions with the active 016 browser session. Product
source is identical to the isolated five-path source commit after its clean CONTROL cherry-pick.

## Evidence attribution and verification plan

The original lane's replacement completed-layer section is read-only at
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-721450bb/docs/plans/rust-ui-build/issues/016-layout-sweep.md`.
Its evidence is under that worktree's `.evidence/016/`:

- `shared-layers-open-inert-red.log`: expanded content remained inert because `inert="false"`
  is still a present HTML boolean attribute. Presence is now conditional on closed state.
- `shared-layers-actual-dock-red.log`: actual mounted dock list was absolute and eye height 40px,
  failing the inline/44px assertion before the two approved tailwind corrections.
- `shared-layers-actual-dock-matrix-green.log` and `shared-layers-actual-dock-clippy.log`:
  trusted keyboard/pointer, callbacks, owner visibility, independent targets, Escape/focus,
  hidden/inert Tab exclusion, retained child draft/node, disabled refusal, remount defaults and
  12 width/theme/appearance combinations passed on the five source files with dirty dependencies.
  These receipts establish the original RED/GREEN; they are not clean combined-pin proof.

Clean combined verification will use only
`/home/chris/.cache/boardstudio-target/shared-layer-prereq-t3-b5fe12f0` after the coordinator grants
one build slot. It will run the actual dock fixture/matrix, shared all-target WASM Clippy, native
shared tests and the mounted shared UI regression floor sequentially. Formatting, Python syntax,
documentation links and the bounded source/lock diff will also be checked. Existing CONTROL
receipts remain valid for unchanged source and will be distinguished from fresh layer checks.
No Cargo command has run for this continuation while the build slot is pending. New receipts go
under `.evidence/shared-layer-prereq/`; fixture screenshots retain the harness's `.evidence/016/`
location in this owned worktree. Cleanup waits for approved landing and an exclusive-owner audit.
