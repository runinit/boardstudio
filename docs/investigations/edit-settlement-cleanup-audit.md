# Edit-settlement cleanup audit

Source refreshed on 2026-10-07 after ticket 15 merged (`d13f0dd1`), during
[ticket 17](../plans/edit-settlement/issues/17-cleanup.md). The ticket Outcome records
final checks and reviews; this report classifies retained source responsibilities.

## One settlement path

All product commits use accepted-state resolvers and `EditTicket`; no product
`Event::Edit` commit remains. Direct web calls are preview paths or deliberate
Session/Core test fixtures. In particular:

- The old position Inspector's preview helper receives Preview; Apply/Enter use
  the resolver in `layout_component_edits.rs`.
- Transform toolbar preview calls pass Preview. Gesture-end commits use
  `commit_transform_drag`, which submits an accepted-state resolver.
- Outline perimeter preview dispatch is explicitly separated from
  `action_resolver` commits.
- Parts create/import/models/generators/profiles/module attach/assembly commits
  were migrated by ticket 15; none is a remaining direct-event exception.

Session's captured gesture route, export capture/identity checks and protected
remap keep their strict semantics. Ticket 18 will decide whether to restrict the
public direct-event API; source migration does not itself enforce that restriction.

## Retained state is not a second settlement implementation

Layout's ticket-backed action records now have descriptive `*Submission` names:
board, alignment, mirrored pair, existing half, placement, keycap size, matrix
setup/edit/preset/delete/transform and outline actions. Apply-to-key retains a
`KeyEditOwner`. These records carry owner identity, tickets or post-landing UI
state; they do not infer success from revision/content equality.

The remaining `Pending*` declarations have different responsibilities and stay:
Core/CAD worker request maps; `PendingLayoutFit` for navigation;
`PendingNewKeyboard` for project creation UI; `PendingProtectedRemap` for strict
reviewed wiring; and `PendingPart`, which holds the part being positioned before
commit, not an edit settlement. Preview/domain pending states are also retained.

No `pending_settlement_gate`, `StdLanded` or `Std-landed` helper remains.
`matrix_transform_lifecycle` still supplies live selection-retention and membership
logic; its Session tests exercise that behavior. `settle_edit_on_submit` remains
used by the placement probe tests. Neither module is dead merely because its name
contains settlement-related vocabulary. Generic failure wording is defined by
`edit_ticket.rs`; other copies are test assertions of that contract.

## Test adapter cleanup

The four `firmware_export_test_*` Runtime fields and fabricated scope/model/epoch,
submit, export-current and delivery branches are deleted. Firmware tests open a
real electrical document through the shared in-process adapter. Core resolves the
wiring, a fixture scripts only the generation-provider reply, and Core's archive
code builds the real ZIP. Request-selective and archive gates/failures live on the
adapter. A generic artifact sink captures final deliveries after Runtime's normal
identity checks. Tests replace owners through real Open and replace executors
through a real Session Core failure/restart, including stale-output suppression at
each provider await and concurrent export reporting.

The archive-import reply override also moved from Runtime into the Core archive
adapter. Saved-project tests gate observation of an actual store result; generic
adapter observers follow each async opening task independently. They do not
fabricate accepted state. Runtime no longer stores feature-specific import/open
completion channels or load gates.

Case-gesture and Keycaps preview executor injection remain leaf provider ports
for strict preview tests. They do not intercept Session submission, accepted state
or edit settlement; changing those provider contracts is outside this cleanup.
The shared `in_process_adapters` and held-effect queue remain the intended test
ports introduced by ticket 01. Neutral board/opened-session fixtures now live in
`in_process_support`, rather than firmware support.

## Decision evidence

The [replacement resolver inventory](../plans/edit-settlement/replace-document-resolvers.md)
is refreshed after ticket 15, with one row per resolver rather than per helper or
textual `ReplaceDocument` hit. Generator Apply and upload are separate resolvers;
model transforms/removal/upload share one. Assembly matrix, module-profile and
module-attach actions use existing typed operations and are excluded.

Architecture now names the intent/EditTicket path. The backlog records queued
coordinates as resolved and retains the distinct stale-preview issue. Tickets 18
and 19 remain human decisions; no API restriction or new Core operation is imposed.
