# 03: Decide the outline version intents

Status: resolved
Type: grilling
Blocked by: 01, [module deepening 06](../../module-deepening/issues/06-split-outline-lifecycle.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Source: [review candidate 06](../../../investigations/architecture-review-2026-10-07.html#c6)

## Question

Which typed Core intents replace the outline action planner's whole-document
replacements, and what do they own?

The planner (`plan_action`, separated from UI by module deepening 06) allocates outline
version and entity IDs, checks polygon validity, and owns active-version selection,
cutouts, bridges, protected gaps and corner finishing. The map lists this as the
strongest outline-rule duplication, with broader semantics than a scalar edit.

## Decide

1. One intent per `OutlineAction` variant, or a typed batch intent for actions that
   change several features atomically?
2. Who allocates new version and entity IDs: Core (deterministic from the document) or
   the resolver (seeded, as today)?
3. Validity: which polygon checks become Core edit errors, and which stay as resolver
   retire reasons?
4. Changed IDs and outline recomputation: which intents affect outlines (see
   `affects_outline`, `core/src/lib.rs`)?
5. Domain terms touched (outline version, active outline, protected gap, outline
   bridge): any sharpening goes into `CONTEXT.md`; ADR-0001/0002 stand.

## Done when

`## Answer` records each decision and the build tickets that follow are sliced into
this effort.

## Proposed answer for human approval

This proposal covers the current generated and fixed-version lifecycle. It does not
claim that ADR-0002 linked refinements already exist in the model.

1. Use typed user-command intents for feature add/update/remove, atomic copy-and-edit,
   active-version selection, version deletion, outline-settings patches and connection
   creation. Generated perimeter editing copies, edits and activates in one operation;
   adding a feature without a fixed active version creates and selects that version
   atomically. Preserve one Undo step per current action. Focus remains presentation.
2. Core allocates new version, feature and connection IDs against the accepted document,
   using a captured operation seed in the intent. Core owns collision checks across
   the relevant persisted namespaces; web does not pre-plan IDs or documents.
3. Vanished or changed action targets retire. Malformed polygon/path input (too few
   points or nonfinite coordinates) is a Core edit error. Self-intersection and other
   geometry findings remain visible/editable and block affected exports according to
   ADR-0001; do not introduce a blanket simple-polygon admission rule. Preserve
   ADR-0002 recovery policy separately from queued-target retirement.
4. Report the touched board plus changed durable version, feature, connection or
   protected-gap IDs. Recompute outlines when active geometry or generated inputs
   change, including activation/deletion/copy, protection/settings and connections.
   Metadata-only naming does not recompute. Verify scene and findings cache currency.
5. Clarify domain terms without changing ADR-0001/0002: Generated is an active choice
   with no fixed version ID; an authored outline connection differs from its resolved
   material bridge; an authored protected gap differs from a derived gap candidate.
   Record linked-refinement representation as a separate follow-up rather than imply
   it is implemented.

After approval, slice builds into feature/settings/connection intents and atomic
version lifecycle intents, with shared Core ID allocation and Session/Core Undo tests.
These builds remain separate from the module-deepening waves in the handoff.

## Answer

Confirmed by the user on 2026-10-09 after Q1-Q12 and a complete shared-understanding
summary. This resolves the design decision, not its implementation. The proposal
above is retained as history: its exclusion of linked-refinement implementation is
superseded by the approved scope below.

### Interface and ownership

- One typed `ApplyOutlineEdit` interface carries the specific named change. Core owns
  rules, target eligibility, validation, identity allocation and mutation; web owns
  drafts, selection, focus and submission. Composite copy/edit/activate actions are
  one accepted edit and one Undo step, not a generic batch interface.
- Core allocates version, feature and connection IDs against the accepted document,
  using a captured creation seed where needed. Core owns collisions and stable
  identity; web must not prepare replacement documents or pre-plan entity IDs.
- Core evaluation and apply share private rules. Equal field values settle Unchanged
  without a revision/history step. Vanished or ineligible targets retire; malformed
  submitted data fails with a Core error and no mutation. Keep the unrelated-operation
  no-op policy unchanged and keep private helpers/cache types private.
- Geometry changes report precise board/version/feature/connection/protected-gap
  owners and update dependent outlines/findings. Metadata-only naming does not rebuild
  geometry. Tests cross the typed Core operation and real Session/Core seams.

### Linked refinements are in scope

- Implement [ADR-0002](../../../adr/0002-linked-outline-refinements.md), not just the
  current fixed-version lifecycle. A linked refinement follows the board's current
  included components: placement, rotation, geometry, additions, removals and inclusion
  changes. Do not freeze or unlink it merely because its source changes.
- Each refinement owns its own settings and authored outline choices. Margin, corner
  finishing, connection width and other settings remain editable while linked;
  recalculation changes that version, not unrelated alternatives.
- Removing a component removes only authored connections, protected gaps and cutouts
  that require it. Preserve independent choices and recalculate the outline and
  bridges from the remaining components using that version's settings. Source removal
  and this cleanup share the same Undo step. Do not leave obsolete references or
  silently retarget an authored connection to another component.
- Freeze or manual perimeter editing creates a separate fixed edited outline while
  retaining the linked refinement. Component movement does not reshape edited
  outlines; findings expose clearance problems under [ADR-0001](../../../adr/0001-fixed-edited-outlines.md).
- Persist explicit ownership, sources and each version's choices. Save/reopen keeps
  linked outlines linked and unlinked/edited outlines unlinked. Existing files retain
  authored geometry and effective behavior; missing link state or origin provenance
  must not infer a new link. Append the next ordered migration only if the
  representation needs one, coordinated with mechanical schema work.
- Recompute after source changes, including bridges. If recomputation cannot supply
  valid manufacturing geometry, the link stays active: retain a clearly identified
  last-valid display where necessary, show located findings and block only affected
  exports. Continue saving/editing and retry resolution on later changes. Ordinary
  component removal with valid remaining inputs is not a reason to retain stale shape.

### Pending edits and errors

- A pending geometry edit for version A retires if another version becomes active
  before execution. Never redirect it to the new active version. Preserve the existing
  action-specific eligibility of activation, naming and other lifecycle actions.
- An edit based on a reviewed generated/source shape retires if that relevant shape
  changes before execution. Capture relevant target/source identity and inputs, not
  the whole document revision; unrelated names or other-board edits do not retire it.
- Too few points or nonfinite coordinates fail with an error and change nothing.
  Self-intersection and other geometric findings remain visible/editable and block
  affected manufacturing outputs rather than impose a blanket simple-polygon
  admission rule. Inactive alternatives and unrelated boards do not block valid output.
- Preserve current lifecycle behavior where not explicitly extended: Generated stays
  available; removing an active named version returns to Generated; focus is
  presentation-only. Source references on copies record origin, not an implicit live
  dependency between versions.

### Implementation tickets and gates

1. [11: Ownership, identity and persistence](11-core-outline-ownership.md).
2. [12: Linked resolution and source cleanup](12-linked-outline-resolution.md).
3. [13: Typed edits and atomic lifecycle](13-core-outline-edits.md).
4. [14: Web adapter and linked/fixed presentation](14-outline-web-adapter.md).
5. [15: Integrated verification and reviews](15-outline-integrated-verification.md).

Design and isolated work can proceed alongside mechanical 02. Shared `model.rs`,
`migrate.rs`, `lib.rs`, geometry settlement and the combined boundary transcript need
one coordinated writer/integration window; parallel work does not authorize conflicting
schema versions or concurrent transcript blessing. Mechanical 02 remains open under
its own acceptance. None of the new outline build tickets is resolved by this Answer.
Run affected native/WASM checks, the complete `python3 scripts/check.py`, and independent
Standards and Spec reviews before accepting the outline implementation.
