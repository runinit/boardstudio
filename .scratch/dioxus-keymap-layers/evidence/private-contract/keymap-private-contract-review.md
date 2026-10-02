# Independent review — F6K.1 private Keymap contract

**Final verdict: CLEAR — no remaining contract blockers.** This verdict applies only to the reviewed design contract; it is not implementation, browser-parity, F3.1 integration, or acceptance approval.

Reviewed contract: `/tmp/frontend-run/keymap-private-contract.md`

Final SHA-256: `c8277af420f45fb2ac94f9f5fb71f0a87cd6f3aaf29ab825a2e64325da3655ac`

Source checkout: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, inspected HEAD `f3905be87fa43f77cbf2df23f3ecacc5a649a4d1`. The cited React files had no diff from reference `5a472a9426e6e38993361da402cd4ec730feb369`. Reviewed the issue/dispatch, React projection/panel/layout/binding/settings/Workbench/selection sources, private Dioxus selection/object resolvers, Session selection branch and core keymap types/validation. This pass reread the revised contract and reused that source evidence. No compilation or repository edits.

## Corrections verified

- Persisted base semantics explicitly follow layer index zero regardless of ID/name. Virtual Base remains derived, with fixed `base`/`Base` identity; missing later-layer bindings display Transparent.
- Membership reproduces the pinned board-membership AND (switch-kind OR direct matrix member) predicate. The issue's companion wording is explicitly qualified: non-switch nested companions are excluded; switch-kind companions remain source-supported. This qualification must remain visible in downstream Spec review and evidence.
- Root-owned active-layer input now appears in the private projector and render seam. Rendering explicitly overlays accepted `scene.transforms[id].pose` on document pose, with document fallback.
- React's captured-selection-mode nuance is disclosed. Dioxus single-key Replace is explicit, with paired first-activation scenarios required after Layout matrix/row/component selection.
- Absent saved values versus equivalent display labels are distinguished without data normalization. Empty Replace explicitly preserves Session's existing anchor ID while clearing the private context/anchor stamp. Empty-select and layer callbacks receive Scope/generation guards.

## Boundaries and open implementation evidence

The existing private `objects::context_for_part` and `selection::submit_canvas_selection` provide the proposed semantic selection route without widening visibility. Scope/generation/current-membership validation, accepted-snapshot authority, selection/document neutrality, shared highlighting, root composition/CSS ownership, and private module ownership are coherent. Scope and generation can be captured in root callbacks; the child rendering seam need not gain another selection store or authority.

No source/spec qualification closes a parity gate. Unsupported scene-context resolution must still be reported; it must not trigger another selection path. The contract retains required paired browser, fixture, theme/compact/focus/accessibility, stale-scope, history/storage, build and independent review evidence, and keeps F3.1 and parent acceptance open. Root must carry the companion qualification and any observed React selection discrepancy into issue/review records.
