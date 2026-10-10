# 06: Paired-half closure proposals and linked shipped splits

Status: claimed
Type: task
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0008](../../../adr/0008-linked-halves-stay-linked.md) · Found by: [02](02-restore-closure-mounts-in-recipes.md)

## Request

Restoring the closure mounts in the shipped splits (02) exposed two Core behaviours around
closure proposals on a half that belongs to a mechanical pair (findings A and B below). The
user decided on 2026-10-09 ([ADR-0008](../../../adr/0008-linked-halves-stay-linked.md)): a
mechanical pair keeps everything linked and mirrored. Exceptions are explicit, per feature.
A proposal made from one half's outline is made once, from the pair's construction source,
and mapped to the other half. Geometry that cannot reflect is a fault of the data: Core keeps
refusing that half's export and says the feature is linked and which half authored it. Core
does not relocate or downgrade the finding.

This ticket supersedes the earlier recommendation to keep proposals local (option A1) and the
known-gap test that pinned it. The findings below are still accurate.

## Findings

Reproduced on the seven measured splits (Corne, Lily58, KLOR, Ferris Sweep, Chocofi, TOTEM,
Cantor) before this ticket.

### A. Proposals made on a paired half were linked to the pair from the wrong outline

1. A settings edit on a pair-owned half whose closure list is absent initializes the list from
   that half's own outline (`apply_settings` and `initialize_closure_mounts`, in
   `core/src/mechanical/edit.rs`).
2. `persist_edit` treats every new mount as a generic addition on a paired half and links it,
   authored by the edited half.
3. The measured recipes put the controller bay and TRRS jack at each PCB's outer edge on both
   halves. The right outline was therefore not the mirror of the left, and its outermost
   proposals mapped outside the left outline. Each left half was refused on export: "The linked
   closure mount at (x, y) lies outside this half's board outline."
4. `pair_findings` (`core/src/mechanical/inputs.rs`) keeps a linked feature as placed and
   reports a linked closure mount outside its board as a `Pcb` error. That is the designed
   behaviour; the defect was the data and the per-half proposal.

### B. Enabling a legacy second half was a silent no-op for closures

`enable` returns early when `effective_inputs` already resolves. A legacy second half resolves
from the shared source, so enabling it changed nothing and its closure list stayed absent.

## What to do

1. Recipe geometry (`core/examples/demo_projects/recipes.rs`): place the second (right) half's
   controller bay, reset switch and TRRS jack at the reflection of the first half's, through the
   pair map. Do not move any measured key or any key matrix position: the 1e-9 comparison against
   `core/tests/fixtures/demo-projects-baseline.json` still holds for keys and for every other
   measured part. Record each moved part per keyboard in the Outcome.
2. Core, TDD (failing tests first):
   - `enable` on an already-effective half whose closure list is absent initializes it (B).
   - A paired half whose closure list is absent gets no proposals of its own. The pair's
     closure mounts are proposed once, from the pair's construction source (its first member,
     the frame of the map), and linked to the pair, authored by that source. The half stores an
     explicit empty list (A). A pair that already has closure mounts proposes nothing more.
   - A refusal for a linked feature says the feature is linked and names the authoring half.
     A linked mount that maps outside the other half's outline is still refused.
   - The known-gap test is flipped to the ADR outcome: proposals link to the pair, both halves
     export, and no closure falls outside either outline on true-reflection geometry.
3. Shipped recipes (`core/examples/demo_projects/mechanical.rs`): remove the explicit
   `initialize-closures` step and its comment if Core now does it; link the controller bay and
   TRRS jack openings and the closure mounts for the ten paired keyboards. Keep any genuinely
   half-specific feature as a recorded exception. Sofle v2 has no closure mounts (gasket mount):
   leave it and say so.
4. Re-record `core/tests/fixtures/demo-projects-baseline.json` deliberately for the ten paired
   fixtures. The Outcome records per keyboard: moved parts, closure counts per half, linked and
   exception features, and new fit findings (closure against controller bay, switch gaps). Report
   them; do not hide a finding by moving a mount without a source fact.
5. The boundary transcript is append-only and checked natively and in WASM. Re-bless it only if
   it must change, and explain the diff.
6. Docs: update the known-gap and "pre-ADR-0008" notes in `map.md`, the pointer in
   [02](02-restore-closure-mounts-in-recipes.md)'s Outcome, and the bay and closure description in
   [keyboard ticket 04](../../typed-core-edits/issues/04-refresh-included-keyboards.md)'s Outcome.
   Keyboard ticket 04 stays claimed.

## Open items this ticket does not settle

- The pair does not record its construction source. Closure proposals use the first member
  (`members[0]`), which is the map's frame and the default author. `link_mirror_pair` accepts
  `construction_source` = the second member as well, so for a pair linked that way the proposals
  still come from the first member until the model records the source (a schema decision).
- `AddSuggestedClosureMounts` takes its suggestions from the caller. A paired half's suggestions
  therefore come from that half's outline when the caller computes them. Core routes only its own
  initialization proposals to the pair. The Case caller is not changed here.

## Acceptance

- [ ] Recipe: for each of the ten paired keyboards, the second half's `U1`, `RST` and `TRRS`
      (and every other `left/*` part) sit at the reflection of the first half's through the pair
      map, 1e-9. Every measured key and matrix part is unchanged against the baseline.
- [ ] Core: `enable` on a legacy second half initializes its closures. A paired half with no
      closure list gets the pair's proposals once, from the construction source, linked.
- [ ] Core: the known-gap test is flipped; the linked-but-outside refusal is kept and its text
      names "linked" and the authoring half.
- [ ] Shipped: no explicit `initialize-closures` step remains; the bays and closures of the ten
      paired keyboards are linked (Sofle v2 excepted); exceptions are recorded.
- [ ] Baseline re-recorded for the ten paired fixtures with the per-keyboard record in the Outcome.
- [ ] Boundary transcript unchanged, or re-blessed with the diff explained.
- [ ] `repo`, `lint`, `test`, `build`, `typecheck` and `browser` run in the places the map names.

## Verification

Native Core tests in a non-nested checkout (`/tmp/claude-1000/boardstudio-tce-verify`), `repo`,
`lint`, `typecheck`, `browser`, and `build` (fixtures regenerate). Never `check.py release`.
Standards and Spec reviews are not part of this ticket.

## Outcome

To be recorded when the implementation is verified.
