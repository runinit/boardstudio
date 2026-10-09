# Map: Typed Core edits

Label: wayfinder:map
Spec: [spec.md](spec.md) · Decision: [ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md)

## Destination

Product edits are typed Core intents that state what they change; Core owns their rules
and reports precise changed IDs. `ReplaceDocument` is used only for import and recovery
and for any bulk action a recorded decision permits.

## Notes

- Domain terms: [CONTEXT.md](../../../CONTEXT.md) (accepted document, pending edit,
  field edit, one-shot action).
- Skills: `tdd` for build tickets, `codebase-design` for operation and seam shape,
  `grilling` + `domain-modeling` for decisions, `code-review` before resolving.
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- For implementation handoff, use [handoff.md](handoff.md): complete matrix Core
  02/03 first, then the approved mechanical ticket 02.
- Starts after edit settlement's
  [ticket 20](../edit-settlement/issues/20-preview-only-direct-edit-event.md), which
  also edits Session's event types.

## Decisions so far

- `ReplaceDocument` is reserved for import and recovery as the direction, no
  enforcement date; archive open/import and recovery reopen, footprint import and
  routed-board reference upload are permitted by name:
  [ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md).
- The first slice is the smallest bounded demonstration, `SetWiringMode`, to settle
  the pattern before larger concepts copy it:
  [edit settlement ticket 19](../edit-settlement/issues/19-decide-first-typed-core-edits.md#answer).
- Ticket 01 is resolved and establishes the pattern for later typed edits.
- Mechanical settings and outline versions come next, from
  [module deepening](../module-deepening/map.md) (user, 2026-10-07):
  [02](issues/02-mechanical-settings-patch.md#answer) has a confirmed design (user,
  2026-10-08): atomic
  settings/clearance projection, bidirectional mechanical mirror pairs for reused
  or distinct PCBs, and per-feature exceptions. The catalogue adapter stays;
  existing projects link explicitly, Make local is explicit, and fit conflicts remain
  accepted with findings. Linked deletion removes both features, omissions are
  explicit, rejoining selects source geometry with a preview, and new construction
  sharing is pair-scoped. Its shared Core edits
  integrate after matrix-layout-consistency 02/03. [03](issues/03-decide-outline-version-intents.md)
  is resolved: the user confirmed one typed outline-edit interface **including linked
  refinements** on 2026-10-09. See its [Answer](issues/03-decide-outline-version-intents.md#answer)
  and the dependency-ordered outline tickets 11-15. 01 and the outline planner split have landed.
- After mechanical 02 and matrix integrated verification, [04](issues/04-refresh-included-keyboards.md)
  redoes all included keyboards using the settled model (user follow-up, 2026-10-08).

## Progress

<!-- one line per resolved build ticket -->

- 01 `SetWiringMode` lands through a typed Core intent with precise board IDs and ordered
  Undo: [Outcome](issues/01-set-wiring-mode.md#outcome).

## Ready for implementation

- [02: Mechanical settings and linked features](issues/02-mechanical-settings-patch.md)
  is `claimed` and remains open. Q1-Q13 and the complete interface, ownership/migration,
  coordinate conventions and build order are confirmed in its
  [Answer](issues/02-mechanical-settings-patch.md#answer). Shared Core work integrates
  after matrix-layout-consistency 02/03; the build and its verification remain open.

## Outline implementation frontier

Decision [03](issues/03-decide-outline-version-intents.md#answer) is resolved; its
build is not. The original proposal is retained as history, but deferring linked
refinements is superseded by the confirmed scope.

```text
03 approved
└── 11 ownership / identity / persistence
    └── 12 linked resolution / source cleanup
        └── 13 typed edits / atomic lifecycle
            └── 14 web adapter / linked presentation
                └── 15 integrated verification / reviews
```

- [11: Core ownership and persistence](issues/11-core-outline-ownership.md) is the
  unblocked outline implementation frontier.
- [12: Linked resolution](issues/12-linked-outline-resolution.md),
  [13: Typed edits](issues/13-core-outline-edits.md),
  [14: Web adapter](issues/14-outline-web-adapter.md) and
  [15: Integrated verification](issues/15-outline-integrated-verification.md)
  follow the dependency chain. No build is claimed complete by design approval.
- Mechanical child tickets 05-10 remain under parent 02's acceptance; outline
  tickets do not close or replace those gates.
- Work can proceed alongside mechanical 02 in separate worktrees. The parent
  coordinates leases for shared Core model/migration/dispatch/geometry files and
  the boundary transcript; append migrations in actual integrated version order.
  Keep one Chrome owner and one transcript blessing owner.
- The keyboard refresh remains a separate follow-up with its recorded mechanical
  and matrix dependencies. Preserve the original implementation handoff's scope.

## Not yet specified

From the [resolver inventory](../edit-settlement/replace-document-resolvers.md). Each
becomes a typed intent, a typed batch intent, or a decision to add it to ADR-0006's
permitted list.

- Project and board renames (setup guide, Library, board inspector): likely next;
  two small operations replacing three resolvers.
- Other outline behavior beyond the confirmed ticket 03 contract requires its own
  decision; outline versions, settings and linked refinements are now specified in 11-15.
- Wiring/net: part-net assignment and the reviewed wiring plan, likely one typed batch
  intent with atomic identity.
- Layout membership and placement: atomic assign-to-layout, part/controller placement.
- Keycap resize plan: check whether existing keycap operations already express it.
- Parts definitions: rename, field patch, new component, models, assemblies, generator
  apply and upload.
- PCB part input settings, pin locks and board-reference edits.
- Bulk actions needing a typed batch intent or a permitted-list decision: physical
  setup, geometry-script apply, generator apply and model upload, assembly placement,
  reviewed wiring plan, matrix and mirrored-half creation, board creation.

## Out of scope

- Enforcing the import-and-recovery restriction (a later decision once the bulk actions
  are classified).
- Smaller or finer-grained Undo history.
