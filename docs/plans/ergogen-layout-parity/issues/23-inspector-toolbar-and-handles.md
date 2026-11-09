# 23: Deliver the accepted Inspector, toolbar and working canvas handles

Status: resolved
Type: build
Blocked by: [22](22-update-layout-consumers.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns only the UI confirmation for exception changes and its trusted-pointer tests; the Core behaviour is left to [21](21-exception-lifecycle.md).

## Outcome

Implement the accepted variant D interaction in the real Rust/Dioxus workspace, using
house controls and existing draft/pending-edit ownership. Do not merge the prototype's
HTML/JS, losing variants or switcher. Core remains the sole placement authority.

## First RED / GREEN slice

Fail a mounted key Shift field/tool interaction through the real Runtime/Core seam:
preview shows actual later-key movement, commit is one edit and Undo restores exactly.
Then add a trusted pointer handle drag to catch hover-target replacement, not just
synthetic PointerEvents which bypass the original bug.

## Subsequent acceptance

- Matrix/Row/Column/Key parameter tools and Carried/This-only editors use named fields.
  Separate Spread/Stagger handles; no Spread at column 0 or Padding at row 0. Show
  pitch defaults, explicit values, units, changed fields and accessible blocked reasons.
- Splay scope selects Splay/Column rotation without edits on switching; Origin fields,
  Pick/drag, Ctrl-locked swing, hold-still nudges, Reset/base snap and Alt no-snap follow
  Core rules. Follow directions constrain matrix drag; disable Rotate appropriately.
- Add/Pick/Follow/Detach controls and confirmed exception changes reach one atomic
  Core intent. Do not silently resize/Rejoin or commit a stale confirmation.
  Ownership note: the Core behaviour of confirmed exception changes is owned by [21](21-exception-lifecycle.md); this ticket owns only the UI confirmation.
- Passive Handles visibility is view-only. Keep the active tool's required origin;
  hover does not replace its SVG target and passive hints do not intercept input.
- Amber effective-pose preview, pre-drag ghosts, pointer capture, release outside the
  handle, cancel/lost capture, context changes and a subsequent drag all recover.
- Preserve newer drafts, queued-operation settlement and validation/refusal messages;
  no whole-document replacement or client transform formula for convenience.

## Checks

Sources: layout `matrix_transform_operation.rs`, `matrix_transform_lifecycle.rs`,
`objects/` Inspector/controllers/toolbar, page canvas and existing shared pending edits.
Use mounted `layout`/page tests and the real packaged-page mouse-drag smoke, rebuilding
providers/page as README requires. Test keyboard nudges and Ctrl/Alt at pointer-down.
Record which engine/input was used; physical touch remains unproved unless tested.

## Answer

Resolved on branch `docs/ergogen-layout-comparison` at `34b8045ec`. All slices
landed, plus five fix commits: `9cd0542e8` (key-selection Add anchored matrix
through Core intent), `ebcfd1897` (anchor-section canvas Pick), `3a3f3666`
(Follow-constrained matrix Move and Rotate drags), `23d22080c`
(trusted-pointer Matrix drag smoke with synthetic dispatch marker), `34b8045ec`
(each matrix tool mount draws only its own handle). Both independent Spec and
Standards re-reviews PASSED. Native baseline `47820fc85` (+13 names,
0 removals). Fresh gates green — quick: `check: all steps passed: repo,
tooling, lint, typecheck, test, browser`; full: `check: all steps passed: repo,
tooling, lint, build, test, browser`.

No release approval is implied.
