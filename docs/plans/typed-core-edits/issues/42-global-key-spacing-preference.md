# 42: Persist a global key-spacing preference

Status: wontfix
Type: build
Blocked by: none
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Decision and source evidence

[Human decision 1](../audit/shipped-boards-review.md#human-decisions-2026-10-10) selects the repository default for new assemblies, an optional global pitch such as 19.00 mm, and a separate 1.0 mm keycap edge gap. Current creation uses 19.05 mm in [matrix_setup_operation.rs](../../../../web/crates/catalogue/src/matrix_setup_operation.rs) and [matrix_placement_controller.rs](../../../../web/crates/layout/src/objects/matrix_placement_controller.rs). The existing preference surface is [project_menu.rs](../../../../web/crates/ui-shared/src/project_menu.rs), with persistence/warning behavior in `web/crates/ui-shared/src/panels.rs`.

## One-hour slice and file ownership

One web-preferences implementer owns a small typed spacing-preference value/codec in `web/crates/ui-model/src/`, its context initialization in `web/src/presentation.rs`, and the user control/persistence in `web/crates/ui-shared/src/{project_menu,panels}.rs`. Also allow the narrow Workspace settings mount in `web/crates/library/src/library.rs` (currently around line 695) and its shared-component import in `web/crates/library/src/lib.rs`; use the existing settings component and avoid unrelated library refactoring. Follow existing browser preference conventions. Provide finite positive X/Y pitch with 19.05 × 19.05 mm defaults and support 19.00. Keep matrix edge gap separate at its existing 1.0 × 1.0 mm default; a new global edge-gap control is outside this slice. This ticket stores/edits the preference only; ticket 43 applies it on creation. Do not rewrite any project, measured input or existing matrix.

## Acceptance and regression

- [ ] A clean session shows repository defaults. A valid edited value persists/reloads; absent, corrupt, non-finite or non-positive pitch falls back safely. Invalid input is shown and does not replace the last valid value.
- [ ] Mounted preference tests start red on the missing control/round trip, then observe saved values after remount through both the project-menu surface and the existing Library Workspace settings mount. Both surfaces share the same preference and validation behavior. Storage-unavailable behavior uses the existing session-only warning; changing this preference does not mutate the accepted document or Undo history.
- [ ] The pitch control is labelled in millimetres and does not masquerade as keycap edge gap. No setting claims physical switch fit.

## Verification

`repo typecheck browser` for the preference/control/persistence behavior; add `lint test` if native shared code is touched. Fresh Sol verifier reproduces defaults, 19.00 round trip, invalid/storage-unavailable paths and mutation-checks the persistence site. Read DESIGN.md before implementation; no API-visibility widening or unrelated preference refactor is in scope. Board recipes keep explicit measured overrides; this is not a respace-all feature.

## Comments

- 2026-10-14: User deferred built-in board work under [138](138-retire-shipped-demos.md). This ticket's board-specific work is out of scope for now; re-triage only if the user brings built-in boards back into scope. Existing findings remain historical evidence, not implementation instructions.
