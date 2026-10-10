# 16: Decide native mirrored halves

Status: ready-for-human
Type: grilling
Blocked by: none
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0008](../../../adr/0008-linked-halves-stay-linked.md#amendment-the-whole-second-half-is-a-reflection-2026-10-09)

## Request

ADR-0008's amendment (user, 2026-10-09): the second half is the first, reflected, in
every respect; Core creates it, recipes never recreate it; exceptions are per item.
Today only same-board layouts link (`CreateMirroredPair`, `core/src/lib.rs:1512`) and the
mechanical pair (`MechanicalMirrorPair`, `core/src/model.rs:1025`) links features.
Recipes mirror halves by hand (`core/examples/demo_projects/recipes.rs`).

## Proposal (from the 2026-10-09 planning pass)

Hybrid: Core creates the half once and stores it in full; the extended pair records
`constructionSource`, `halves.{sourceBoardId, mirrorBoardId, correspondences, exceptions}`;
Core re-derives the mirror from the source after every accepted edit and on Open; edits on
the mirror are inverse-mapped to the source. A reused PCB is a flipped instance with no
copy. Format version 5, no data rewrite; existing projects are never linked automatically.
Rejected: derived-only (the web reads stored parts at about 282 sites) and copy-once with
per-operation propagation (drift).

Planned build tickets once answered: 17 schema and format v5; 18 `CreateMirroredHalf`;
19 reconcile on every edit and exceptions; 20 recipes build one half; 21 web adapter;
22 integrated verification; 23 redo keyboard [04](04-refresh-included-keyboards.md) on the
native model. An ADR-0010 is drafted from the answers.

## Questions (answer each; recommendations in brackets)

1. Representation: accept the hybrid? [yes]
2. Chiral parts on a distinct-PCB mirror: one reflected-heading rule on the same side,
   switching to a reversible variant where the footprint has one? [yes; replaces the
   three current conventions: layout 180-r, pair -r mod 180, recipe -r]
3. Mirror column numbering: keep the source numbering (column 0 is the pinky on both
   halves, same pins) or reverse it as the recipes do today? [keep source; renames
   right-half part and net ids]
4. Reversible shipped boards: two boards (today) or one board plus a flipped instance?
   [two boards for now; the source PCBs' reversibility is unverified]
5. Never mirrored by nature: keymap bindings, instance role/half/name, board references,
   copper traces and vias. Confirm or amend. [confirm]
6. Linking existing independent two-board projects: out of scope for now? [yes]

## Acceptance

- [ ] Answers recorded; ADR-0010 drafted from them; CONTEXT.md term for the mirrored half.
- [ ] Tickets 17-23 written and adjusted to the answers.

## Answer

To be recorded.
