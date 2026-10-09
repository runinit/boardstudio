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

- 04 (partial, ticket still claimed) the ten split included keyboards open as explicit mechanical
  mirror pairs through the typed operation (Sofle linked, measured splits with local right-half
  bays, Sofle v2 gasket as the pair construction), unibody and review projects untouched, no key
  moved, a reused flipped-PCB demonstration and six Core request tests landed; repo, lint, build
  and native test pass, while mounted/live checks and the Standards/Spec reviews remain:
  [Outcome](issues/04-refresh-included-keyboards.md#outcome).
- 01 `SetWiringMode` lands through a typed Core intent with precise board IDs and ordered
  Undo: [Outcome](issues/01-set-wiring-mode.md#outcome).
- 15 integrated outline verification closes the outline phase: the typed outline transcript is
  appended (60 core entries, native and WASM parity), matrix, mechanical and outline actions hold
  their queue contract in both orders with linked refinements following and fixed copies staying,
  the full check passes at `77b9548b8`, and independent Standards and Spec reviews are dispositioned
  with Core settings validation, base-contour protection, linked-choice cleanup on every removal
  path and linked-refinement settings editing fixed and the rest recorded as accepted deviations
  or backlog: [Outcome](issues/15-outline-integrated-verification.md#outcome).
- 14 the outline web adapter lands: the planner is replaced by the typed adapter
  (zero `ReplaceDocument` consumers in the outline module, Generate through typed
  `CreateGenerated`), ownership is presented distinctly with located findings and
  blocked exports, one-shot Freeze retains the link, and hover/focus create no history:
  [Outcome](issues/14-outline-web-adapter.md#outcome).
- 13 typed Core outline edits land: one `ApplyOutlineEdit` operation with the
  kebab-case change vocabulary, Core-owned identity allocation and non-duplicating
  default names, atomic composite lifecycle, fingerprint-scoped retirement, and
  malformed-input errors without mutation:
  [Outcome](issues/13-core-outline-edits.md#outcome).
- 12 linked outline resolution lands: linked refinements rebuild from the board's
  current included components with version-local choices, source cleanup rides the
  same accepted edit/Undo, failed resolution keeps the link with a located last-valid
  finding, inactive alternatives cannot block, and the expected region count follows
  the current structure: [Outcome](issues/12-linked-outline-resolution.md#outcome).
- 11 Core outline ownership lands: explicit `OutlineOwnership` per version with
  absence-equals-fixed defaults (no migration), retired-identity reservation,
  cross-namespace collision refusal, version-namespaced derived identities, archive
  and browser-persistence parity, and no inferred links on legacy reopen:
  [Outcome](issues/11-core-outline-ownership.md#outcome).
- 02 typed mechanical edits land complete: Core-owned rules/inputs/findings, mirror pairs
  with per-feature sources, atomic lifecycle (removal, Q14 paired disable, legacy
  conversion), durable identity reservations, author-contour gasket resolution,
  per-target admission/export gating, Case adapters with reviewed fingerprints and a
  read-only projection port — integrated, fully checked, boundary transcript unchanged,
  independently reviewed with all blockers fixed through `2a8d5e076`:
  [Outcome](issues/02-mechanical-settings-patch.md#outcome). Children 05-10:
  [05](issues/05-core-effective-inputs.md#outcome),
  [06](issues/06-core-apply-settings.md#outcome),
  [07](issues/07-core-closure-projection.md#outcome),
  [08](issues/08-core-mirror-pairs.md#outcome),
  [09](issues/09-case-adapter-and-setup.md#outcome),
  [10](issues/10-integrated-verification.md#outcome).

## Ready for implementation

- The outline phase (11-15) is resolved. The remaining work is separate: matrix tickets 04-11
  (their own map) and [04: Refresh included keyboards](issues/04-refresh-included-keyboards.md),
  which is blocked only by matrix 11.

## Outline implementation frontier

Decision [03](issues/03-decide-outline-version-intents.md#answer) is resolved and its
build (11-15) is complete. The original proposal is retained as history, but deferring
linked refinements is superseded by the confirmed scope.

```text
03 approved
└── 11 ownership / identity / persistence
    └── 12 linked resolution / source cleanup
        └── 13 typed edits / atomic lifecycle
            └── 14 web adapter / linked presentation
                └── 15 integrated verification / reviews
```

- [12: Linked resolution](issues/12-linked-outline-resolution.md),
  [13: Typed edits](issues/13-core-outline-edits.md),
  [14: Web adapter](issues/14-outline-web-adapter.md) and
  [15: Integrated verification](issues/15-outline-integrated-verification.md)
  followed the dependency chain and are resolved with their recorded deviations.
- Mechanical tickets 02 and 05-10 are resolved; outline tickets own their own gates
  from here.
- Work proceeds in separate worktrees. The parent
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
