# 16: Case, mechanical settings and project/board names land through resolution

Status: resolved
Type: build
Blocked by: 06
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Move the Case workspace's edits and the three rename actions onto resolvers and the
edit ticket. Two of the renames are in the inventory's top ten risks: the setup
guide copies the document as it was when the panel rendered, and the library rename
replaces the whole document with no idle check. After this ticket each rename is a
field edit that sets only the name on the accepted document.

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

| # | Action | Submit | Kind | Settlement to delete |
|---|---|---|---|---|
| 54 | Case body edits and mount drag commit | `web/crates/case/src/case_controller.rs:470` | field edit, `SetCase` | `PendingBodyEdit` |
| 55 | Mechanical settings | `web/crates/case/src/mechanical_settings_mount.rs:180` | field edit, `ReplaceDocument` | `expected_matches` (`mechanical_settings_controller.rs:893`) |
| 8 | Setup-guide project rename | `web/src/presentation.rs:8243` | field edit, `ReplaceDocument` | none |
| 26 | Board rename | `web/crates/layout/src/board_inspector.rs:141` | field edit, `ReplaceDocument` | none |
| 58 | Project rename (library menu) | `web/crates/library/src/library.rs:990` | field edit, `ReplaceDocument` | none |

## Background you need

- **Mechanical settings carve-out.** `expected_matches` does not only compare typed
  values: it compares the closure evidence Core derives from the `case-closure/*`
  parts (`mechanical_settings_controller.rs:~893-925`), with the message "The saved
  document does not contain the requested mechanical settings and closure
  clearances". Find out what condition that catches.
  - If it only re-checks that the typed settings landed, delete it like the other
    content checks.
  - If it reports a real condition of the design (Core could not satisfy the
    requested clearances), it must not stay a landing check. Make it a resolver
    precondition (retire with that reason) or a status the panel shows from the
    accepted document independently of settlement.
  - If choosing between those needs a product decision, stop, add a note under
    `## Comments` here, and ask.
- #54's mount drag: if it uses the gesture path, leave the gesture alone; only the
  final `SetCase` commit through `Event::Edit` becomes an intent.
- Library rename and the board rename already have real-Session browser tests
  (`project_name_test_support`, now on ticket 01's adapter); extend those. The
  board-name input also appears in `outline_lifecycle_browser_tests`.
- The case panel uses a weaker Std-landed (saved and token changed); delete it.

## Acceptance criteria

- [x] Rapid test: board rename then an unrelated Layout edit queued behind it; both survive.
- [x] Rapid test: library rename while a Parts edit is queued keeps the Parts edit.
- [x] Two case body field edits committed back-to-back both survive; Undo removes them in order.
- [x] The mechanical closure check is either deleted or moved to a resolver precondition or independent status, with the reasoning in the Outcome.
- [x] `PendingBodyEdit` and `expected_matches` (as a landing check) are gone; no action sends `Event::Edit` commits directly.

## Verification

```sh
python3 scripts/check.py typecheck test
python3 scripts/check-wasm-tests.py
wasm-pack test --headless --chrome web/crates/case --locked --lib
wasm-pack test --headless --chrome web/crates/library --locked --lib
wasm-pack test --headless --chrome web/crates/layout --locked --lib
```

## Out of scope

- A case plan module or typed mechanical edits (review candidates 4 and 2).


## Outcome

Implemented in `3e4f898b9`, `1813508fe` and `171f9f8f9`. Case bodies, mechanical settings, setup-guide project rename, library
project rename and board rename now submit accepted-snapshot resolvers through
`EditTicket`. Scope and target checks retire ineligible work. Field commits remain
editable during Applying/Saving, retain separate tickets, and preserve unrelated
accepted changes. One-shot controls use action-specific pending feedback. Case
mount previews and gesture ownership remain unchanged.

`PendingBodyEdit`, mechanical `ExpectedCommit`/`ExpectedSettings`/`ClosureEvidence`
and `expected_matches` are deleted. The old mechanical check compared the requested
settings and generated closure-part rows/membership with the landed document. It
was an equality-based landing check, not a test that the physical design could
satisfy clearance constraints. Removing it needs no product decision. Mechanical
planning now projects the proposed accepted-based document with Core's pure
mechanical functions and the accepted scene's contours before submission; Core
findings remain independent accepted-design feedback. Catalogue loading remains
asynchronous, with requests dispatched in commit order after loading completes.

Regression evidence: library rename first overwrote the earlier Parts definition
name; queued board rename was ignored during Applying; the second authored body
field stayed at its old value; the mechanical controller rejected its second
field. Each failed for that expected reason before the migration, then passed
against real Session/Core with accepted values and ordered Undo verified. A mounted
pending-action test also failed when unrelated buttons were disabled, then passed
with control-specific guards. Numeric and text Escape/blur regressions were corrected without preventing
another edited draft from committing; the text test confirms same-field commits
before acknowledgment. A further regression first showed an older landing replacing
active text (`M5` became `M4`); accepted-value synchronization now preserves dirty
numeric and text drafts, and the mounted test passes.

Review: Standards — no documented breaches or required changes, including the
follow-up. Spec — the remaining mounted mechanical global busy guard was removed,
and pending action guards were narrowed. Project rename resolvers explicitly check
session/document identity. No remaining clear spec violations on rereview. The
same-field duplicate guard is cleared by input; it does not prevent newer commits.

Verification: application 29 passed; runtime 103 passed; Case 56 passed; library 12
passed; Layout 93 passed; presentation Component Inspector 20 and Layout remainder
6 passed. `check.py typecheck repo`, `check-wasm-tests.py` and `git diff --check`
passed. After rebasing onto agent A's Parts repair (`7e520799d`), typecheck and repo
checks pass with no temporary Parts overrides; those overrides were never staged.

Blocked environment/baseline gates: `check.py test` reaches the CAD STEP oracle's
workspace-discovery error in this nested worktree and fails there. This checkout
has no `check.py lint`
step; targeted WASM Clippy with `-D warnings` stops at the pre-existing Application
`Resolution` large-enum-variant warning. These blockers were not changed by this
ticket. The affected browser subsets of `check.py browser` were run as listed above.

Follow-up: ticket 17 remains gated on all cluster tickets 08–16 being resolved.
Explicit adoption actions remain collection replacements; other opening and mount
actions were narrowed in the review follow-up below. Typed Core mechanical commands
remain outside this ticket.

### Astra/high review follow-up (2026-10-07)

`0cbab30a7` addresses the queue/draft findings from the requested Astra/high review.
Authored gasket inset/width/depth now change only their accepted field. Opening
and mount add/remove/vertex actions resolve narrow intents against accepted
collections; explicit adoption actions retain replacement semantics. Captured
preceding-removal metadata retires queued positional fields whose opening or
vertex index shifts, under the existing single structural-action guard. Library
Escape/blur submits no canceled rename. Library, setup-guide name and authored
body numeric fields preserve newer typing when older tickets settle. A private
setup-name hook exposes the production behavior to its mounted regression.

Regression tests first reproduced the stale gasket/vector overwrite, shifted
opening target, canceled Library rename and older-landing draft erasure, then
passed with accepted values and ordered Undo checked. Failed-removal guard behavior
was source-traced by Astra; the runtime regression covers a successful removal.
`5d3bce3f` removes two production unwraps reported by Standards and formats the new
mount regression. Astra/high Standards and Spec rereviews report no remaining
findings after that correction.

Case browser 60, Library browser 13, page browser 42, native Application 29 and
Runtime 102 passed. Page typecheck, repo/docs, WASM ownership and whitespace checks
passed. The focused opening browser rerun after the checked-binding correction passed
(3 tests).
The CAD workspace-discovery and pre-existing Clippy gates above remain unchanged.
No ticket-15 code, strict export/remap path, dependency or public API was changed.
