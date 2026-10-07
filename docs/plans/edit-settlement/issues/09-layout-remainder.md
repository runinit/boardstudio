# 09: Layout remainder: constraints, old position Inspector, nudges and geometry scripts

Status: resolved
Type: build
Blocked by: 06
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Move the Layout actions that ticket 06 did not cover onto resolvers and the edit
ticket. Constraint set/remove in the Layout Component Inspector, the old position
Inspector (still shown when the Component Inspector has no projection), the object
tree's keyboard nudge and the geometry-script New/Apply actions.

Keyboard nudges change in one visible way: each press becomes a delta intent ("move
by one step"), resolved against the accepted position when it runs. Holding an
arrow key now moves the part by every press instead of losing repeats. Each press
is still its own Undo step.

## Migration rules (same for every cluster ticket)

Ticket 06 sets the pattern; read its Outcome and copy its shape before starting:
`web/src/presentation/layout_component_edits.rs` (pure resolver builders, one
`EditTicket` per committed field, the settle helper) and
`web/crates/runtime/src/edit_ticket.rs` (`EditTicket::begin(port, label, feature,
resolver)`, `settlement(owner_is_live)`, `is_pending()`; usage example in its module
docs). Ticket 06 left two things for the clusters: it had no one-shot controls, so
the first cluster ticket with one establishes the `is_pending()` disabling; and its
settle helper passes `owner_is_live = true` because the Inspector unmounts with its
owner. Panels that outlive their owner (a selection change keeps the panel mounted)
must pass a real liveness answer. In short:

- Every action in this ticket submits intent through the edit ticket (ticket 04)
  with a resolver. This includes field-scoped operations: their resolver checks the
  target still exists and submits the command, or retires.
- A resolver is a pure function of the accepted snapshot plus values captured at
  submit. It never reads signals or Runtime state. Whole-document actions move their
  clone-and-mutate step into the resolver, so `ReplaceDocument` is built from the
  accepted document at execution.
- **Field edits** queue freely: remove the panel's "refuse while pending" guard and
  never disable the field for a pending edit. **One-shot actions** disable their
  control, with no message, while their ticket `is_pending()`. See the
  [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-field-edits-and-one-shot-actions-2026-10-06)
  and the terms in [CONTEXT.md](../../../../CONTEXT.md).
- The latest committed value wins. Retire only when the target is gone or no longer
  eligible, with a reason the user understands. If the value already equals the
  accepted value, resolve **Unchanged**.
- Delete the panel's `Pending*` settlement struct and its "did it land" heuristics
  (token/revision/durability checks, whole-document equality, revision = base+1,
  content checks and their "does not contain the requested value… retry" messages).
  Landed means landed; the field then shows the accepted value.
- Failure wording comes from the edit ticket. Keep only a feature noun.
- Keep admission checks at dispatch (selection lifetime, stale callbacks, owner
  identity) so stale UI is still ignored early.
- Previews stay on `Event::Edit` with the preview phase; only commits become
  intents.
- Tests assert accepted results (the accepted document, settlement, Undo and the
  field text the user sees), not command shapes or `Pending*` internals. Each ticket
  adds at least one rapid-entry test for its riskiest action: two commits queued
  behind a gated Core reply both survive, and Undo removes them in order.

## Call sites

From [inventory.md](../inventory.md) (line numbers at `3368825`, orientation only):

| # | Action | Submit | Kind |
|---|---|---|---|
| 4 | Component Inspector SetConstraint | `web/src/presentation.rs:2350` | field edit, `SetConstraint` |
| 5 | Component Inspector RemoveConstraint | `web/src/presentation.rs:2366` | one-shot, `RemoveConstraint` |
| 6 | Old position Inspector: commit on Enter/Apply | `web/src/presentation/inspector.rs:171` | field edit, `MoveParts` |
| 7 | Object-tree keyboard nudge | `web/src/presentation.rs:4776` | delta intent, `MoveParts` |
| 56 | Geometry scripts: New | `web/crates/ui-shared/src/geometry_scripts.rs:122` → `:275` | one-shot, `ReplaceDocument` |
| 57 | Geometry scripts: Apply | `web/crates/ui-shared/src/geometry_scripts.rs:214` → `:275` | one-shot, `ReplaceDocument` |

## Background you need

- #6 also sends a live preview on every keystroke. The preview stays as it is
  (map decision: previews stay on `Event::Edit`); only its commit becomes an intent.
  Build the commit's resolver like ticket 06's `SetPosition`: accepted position
  plus the user's axis value, first-selected anchoring for groups.
- #7: the resolver captures the part IDs and the delta, reads each part's accepted
  position, and retires if a part is gone, locked or relationship-driven. Don't
  capture an absolute target.
- #4/#5: retire when the part or the constraint no longer exists.
- #56/#57: the script is applied to the accepted document inside the resolver.

## Acceptance criteria

- [ ] Native or mounted test: three nudges queued behind a gated Core reply move the part three steps; three Undos restore it.
- [ ] Mounted test for #6: X then Y committed with replies gated keeps both (old Inspector).
- [ ] Constraint set then remove queued back-to-back both land; removing a constraint on a deleted part retires.
- [ ] Geometry-script Apply queued behind an unrelated edit keeps that edit.
- [ ] None of these actions sends `Event::Edit` commits directly.

## Verification

```sh
python3 scripts/check.py typecheck
python3 scripts/check-wasm-tests.py
python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs
wasm-pack test --headless --chrome web/crates/ui-shared --locked --lib
```

## Out of scope

- Merging held-key nudges into one edit (map: out of scope).
- Stale previews drawn after a queued commit (recorded in `docs/backlog.md`).
- Matrix and transform nudges (ticket 11).

## Outcome

Commits: claim; "Land constraint, old position, nudge and geometry-script edits through
resolution".

- Constraint set/remove (call sites 4 and 5) already submit through resolvers since ticket 06;
  this ticket's tests now cover them queued back-to-back and retiring on a deleted part.
- Keyboard nudge (`nudge_resolver`): a delta intent that moves each part from its accepted
  position when it runs and retires when a part is gone, locked or relationship-driven. The
  tree handler keeps its admission checks and the selection/camera side effects.
- Old position Inspector: previews stay on `Event::Edit`; the Enter/Apply commit begins a
  ticket (`commit_position_resolver`) that keeps the preview's transaction id, retires if the
  part is gone and resolves Unchanged on an equal point.
- Geometry scripts: `new_script_resolver` and `apply_script_resolver` build the
  `ReplaceDocument` from the accepted document; New and Apply disable while their ticket is
  pending, a failed ticket shows its message inline, and admission now only requires the open
  project session (the token/revision equality is gone).
- New tests (real Session/Core, gated reply): `layout_remainder_tests.rs` — three nudges
  queued behind a gated reply move the part three steps and three Undos restore it; a nudge
  retires when its part was deleted; old-Inspector commits keep the latest value and Undo
  steps back; constraint set then remove both land; removing a constraint whose part was
  deleted retires. `geometry_scripts.rs` — Apply queued behind an unrelated edit keeps that
  edit; two queued New scripts get distinct names; Apply on a deleted script retires.

Checks: `python3 scripts/check.py typecheck`; `python3 scripts/check-wasm-tests.py`;
`python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs`
(20 executed, 0 failed) and `--files web/src/presentation/layout_remainder_tests.rs` (5/0);
`wasm-pack test --headless --chrome web/crates/ui-shared --locked --lib` — 9 passed.
Typecheck was run with agent A's broken WIP parts files swapped for their pre-WIP versions
locally (never committed); see the handoff.

Follow-ups: the project-name submit in `presentation.rs` still sends `Event::Edit` directly
(ticket 16); a failed script ticket's message stays visible until the next script action.


### Continuation review correction

The two-axis review found a spec gap in the old position Inspector: it captured a
whole point and moved only one part, and the initial test bypassed the mounted
Inspector. The correction commits only the edited axes against the accepted
position, translates the captured selection from its first part, and keeps edit
tickets so pending drafts and failure feedback follow settlement. A mounted,
gated X/Y test with reversed document/selection order failed first (the selected
anchor remained at `(40, 0)` instead of `(45, 7)`), then passed with group positions,
field text and two Undo steps verified. The direct resolver test now supplies only
its committed axis. Geometry-script admission was renamed for clarity.

Review: Standards — no documented breaches; optional duplication of symmetric
input handlers. Spec — the axis/group/mounted-test finding is corrected; no
remaining findings on re-review.

Checks: application tests 29 passed; runtime tests 103 passed; `check.py repo`
and `typecheck` passed; `check-wasm-tests.py` passed; Layout remainder browser
suite 6 passed; Component Inspector browser suite 20 passed; `git diff --check`
passed. Typecheck/browser checks still require the uncommitted pre-WIP Parts-file
workaround: unmodified `origin/dev` fails with nine errors in agent A's Parts WIP.
