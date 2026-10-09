# 06: Decide how closure proposals behave on a paired half

Status: ready-for-human
Type: task
Blocked by: none
Map: [map.md](../map.md) · Found by: [02](02-restore-closure-mounts-in-recipes.md)

## Request

Restoring the closure mounts in the shipped splits (02) exposed two Core behaviours around
closure proposals on a half that belongs to a mechanical pair. The shipped recipes avoid
both, so no shipped project is affected. A user doing the same steps in the Case workspace
can hit them. Both are policy questions before they are code changes.

## Findings

Reproduced on the seven measured splits (Corne, Lily58, KLOR, Ferris Sweep, Chocofi,
TOTEM, Cantor). The known-gap test
`closures_proposed_on_a_paired_half_link_to_the_pair_and_block_the_other_halfs_export` in
`core/tests/included_keyboards.rs` pins A and fails if Core changes it. B is shown by the
recipes: after `enable` the second half's stored closure list is still absent. `link_pair`
initializes a half only when its list is absent after `enable`, so a Core fix for B needs
no recipe change.

### A. Proposals made on a paired half are linked to the pair

1. A settings edit on a pair-owned half whose closure list is absent initializes the list
   from that half's own outline (`apply_settings`, `needs_initialization`, in
   `core/src/mechanical/edit.rs`).
2. `persist_edit` then treats every mount that was not there before as a "generic addition
   on a paired half" and links it to the pair, authored by the edited half.
3. The pair map places each linked closure mount on the other half. The measured splits
   put the controller and TRRS at each PCB's right edge on both halves, so the right half's
   outline is not the mirror image of the left's. Its outermost proposals map outside the
   left outline. Four proposals joined the pair in each of the seven, and every left half
   is refused on export: "The linked closure mount at (x, y) lies outside this half's board
   outline" (Corne: (-21.51, -40.16) in the known-gap test).
4. This is designed behaviour, not a defect in the check: `pair_findings`
   (`core/src/mechanical/inputs.rs`) keeps a linked feature exactly as placed, never
   relocates or unlinks it, and a linked closure mount outside its board is a `Pcb` error
   that blocks that board's export. What is questionable is that a proposal made from one
   half's outline becomes a pair feature at all. Sofle is not affected: its halves are
   exact reflections, so the linked proposals stay inside both outlines.

### B. Enabling a legacy second half is a silent no-op for closures

`enable` returns early when `effective_inputs` already resolves
(`already_effective`). A legacy second half resolves from the shared source, so enabling it
changes nothing and its closure list stays absent: no proposals, no closure holes, no
finding. Closure mounts appear only on its next settings edit (which then causes A, if the
half has meanwhile been paired) or from the explicit `initialize-closures` action.

## Options

For A:

1. Proposals from initialization stay local to the half they were made from. Only mounts the
   user adds link to the pair by default. Simple, and consistent with proposals being a
   per-outline suggestion. Existing saved projects that already hold linked proposals are
   unaffected.
2. Link a proposal only when its mapped point lies inside the other half's outline,
   otherwise keep it local. Keeps linking for true mirrors (Sofle) but makes pair state
   depend on geometry at the moment of the edit.
3. Keep linking and downgrade the outside-outline finding for a closure mount to a
   non-blocking warning. Hides a real drill outside the board; rejected unless the user
   wants it.
4. Do nothing in Core. Recipes keep initializing before the link, and users learn from the
   export refusal. The refusal text does not say why the mount is linked.

For B:

1. `enable` also initializes closures when the target resolves but its closure list is
   absent and the mount is not a gasket.
2. Leave `enable` as is and rely on the explicit `initialize-closures` action, shown in the
   Case workspace when the list is absent. Cheapest; the gap stays silent.

## Recommendation

A1 and B1. Together they make `enable` on either half produce that half's own proposals, in
any order, so a recipe or a user cannot reach a refused export by editing the right half
first. Each needs a Core test first (TDD): a paired right half whose closure list is absent
gets local proposals on its first edit, the pair gains no closure feature, and both halves
export. Then flip the known-gap test to assert that outcome, and remove the explicit
`initialize-closures` step and its comment from `link_pair` in
`core/examples/demo_projects/mechanical.rs`. The baseline must not change when that step
goes: if it does, the recipe's order mattered and the difference needs a record.

## Acceptance

- [ ] The user picks an option for A and for B (or accepts the recommendation).
- [ ] If Core changes: tests written first, the known-gap test updated, the shipped
      baseline unchanged, and `test`, `lint` and `build` run.

## Verification

Native Core tests in a non-nested checkout, `lint`, `build`. No ADR unless option A3 is
chosen.
