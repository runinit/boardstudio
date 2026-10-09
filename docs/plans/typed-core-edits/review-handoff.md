# Second-opinion handoff: typed Core edits

Written 2026-10-09 for an independent reviewer. Nothing here is merged to `dev`.

## What to review

- **Branch:** `typed-core-edits/mechanical` (local only, worktree `.worktrees/tce-matrix`; **no remote copy yet**, so it must be pushed or bundled before another machine can see it).
- **Base:** `ef45d94d7` (the merge-base with `dev`). About 180 commits, 172 files, +86k/-7k lines; much of that is the generated boundary transcript (`core/tests/fixtures/boundary/transcript.json`, append-only golden).
- **Review it as:** `git diff dev...typed-core-edits/mechanical`, or by phase below.
- Related lane branches (already folded into the integration branch, listed for history): `typed-core-edits/{matrix-core,mechanical-*,outline-*}`.

## The idea in one paragraph

After edit settlement, every product edit is submitted as intent and resolved against the accepted document when it runs ([ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md)). About two dozen resolvers still answered with `ReplaceDocument`: they copied domain rules into the web layer, made Core report every part as changed, and re-ran outline and script work for a single scalar. This effort turns them into **typed Core operations** (`EditOperation::...`), so Core owns the rule, reports only the IDs that actually changed, and runs outline work only when needed. `ReplaceDocument` stays for import and recovery ([ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md)). Spec: [spec.md](spec.md).

## Phases on the branch

1. **Mechanical settings and linked mirrored features** (ticket 02; [map](map.md)): Core owns mechanical settings, mirror pairs and per-feature Linked/Local/Omitted state (`core/src/model.rs` ~900-1100, `core/src/mechanical/`).
2. **Outline edits** (tickets 11-15): outline ownership, linked refinements, typed outline change ([ADR-0001](../../adr/0001-fixed-edited-outlines.md), [ADR-0002](../../adr/0002-linked-outline-refinements.md)).
3. **Matrix layout consistency** (separate map: [matrix-layout-consistency](../matrix-layout-consistency/map.md), tickets 01-11): `SetMatrixPlacement` with Stagger vs Offset meanings, a single web placement builder, the Relations panel ([ADR-0007](../../adr/0007-matrix-gestures-edit-named-parameters.md)).
4. **Check speed** (separate map: [check-speed](../check-speed/map.md)): faster `scripts/check.py`; unrelated to the architecture but on the branch.
5. **Included keyboards** (ticket [04](issues/04-refresh-included-keyboards.md), still `claimed`): split keyboards shipped as mirror pairs ([04-handoff](issues/04-handoff.md)).
6. **Generator parameters** ([generator-parameters](../generator-parameters/map.md)): found while doing 5. Closure holes were written as text where the generator wants numbers; tickets 01-02 resolved, 03 in flight (v3→v4 migration), 04-06 planned. Standard recorded in [ADR-0008](../../adr/0008-linked-halves-stay-linked.md): a pair keeps everything linked, exceptions are explicit.

## Questions I would like a second opinion on

1. **Operation granularity.** `ApplyMechanicalEdit { target, change, mountingHole }` with a `MechanicalChange` enum (about 20 variants) versus one `EditOperation` per intent. Is the enum a sound seam or a second command language hiding inside one operation?
2. **Linked-by-default pairs** ([ADR-0008](../../adr/0008-linked-halves-stay-linked.md)). Pair state is `Linked | Local | Omitted` per feature with a `Pose2` mapping. Is storing one authored value plus a mapping robust when the halves' outlines are not exact reflections (the Corne-style bay problem in generator ticket 06)? Should a refused export on a linked-but-outside feature be an error, or should the pair relocate it?
3. **Wire contract.** The page still sends the mounting-hole template with each mechanical edit even though Core has the generators. Worth removing now?
4. **Versioned migrations.** Is a v3→v4 step the right tool for repairing stored strings, versus validating at the boundary only ([ADR-0004](../../adr/0004-document-format-version-and-id-rename.md))?
5. **Boundary transcript.** A 16-line change for a generator fix suggests the golden is both strong and noisy. Is append-only plus re-bless the right discipline?
6. **Undo cost.** Core history stores a full prior document per step, so typed edits do not shrink Undo. Is that acceptable at the intended project sizes?
7. **Placement authority.** Core publishes `staggerAxis` and companion modes; the web builds operations in one module. Anything still leaking Core maths into web (known: `layout_align_controller.rs:~822` writes `column_offsets` through `SetMatrix`)?

## Known gaps (honest status)

- **Not verified in a live product:** keyboard ticket 04's mounted-browser and paired-edit checks; matrix ticket 11 steps for Row 2 Offset and the mirrored pair were not run in the real app. Shift-drag was verified by dispatched events only.
- **No Standards/Spec review at plan level**; reviews ran per ticket pair.
- **GitNexus `detect-changes --scope compare` is capped and the index is stale**, so there is no clean graph blast-radius result for the whole branch.
- **Full `check.py` never ran as one command**; steps ran per batch (the nested-worktree CAD limitation means native `test` runs in a non-nested checkout). `check.py release` has never been run, deliberately.
- Fit findings on shipped keyboards are recorded, not fixed (controller bay vs closure holes, tight switch-cutout gaps); Sofle v2's gasket carries no closure mounts. Physical fit and electrical qualification are not established.

## How to run it

```bash
git worktree add /tmp/review-tce typed-core-edits/mechanical   # needs the branch available locally
cd /tmp/review-tce && python3 scripts/check.py --list
python3 scripts/check.py repo lint test typecheck   # use a non-nested path; do not run `release`
```

Native tests must run from a path not nested under another cargo workspace. Browser presentation compiles only for WASM; `browser` needs Chrome (`wasm-pack test --headless --chrome`).

## Where the evidence is

- Spec, map and tickets: [spec.md](spec.md), [map.md](map.md), [issues/](issues/); older handoff: [handoff.md](handoff.md).
- Per-ticket Outcomes hold commands, results and deviations (for example [matrix 11](../matrix-layout-consistency/issues/11-integrated-verification.md)).
- ADRs: [0001](../../adr/0001-fixed-edited-outlines.md) to [0008](../../adr/0008-linked-halves-stay-linked.md).
- Core tests to start with: `core/tests/matrix_placement.rs`, `core/tests/included_keyboards.rs`, `core/tests/closure_generator_parameters.rs`, `core/tests/demo_projects.rs`.
