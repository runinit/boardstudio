# 03: Decide the outline version intents

Status: ready-for-human
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
