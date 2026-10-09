# 04: Redo all included keyboards using the settled editing model

Status: claimed
Type: task
Blocked by: 02, [matrix integrated verification](../../matrix-layout-consistency/issues/11-integrated-verification.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Request

The user requested this follow-up during mechanical Q8 on 2026-10-08: once the
matrix/mechanical behavior is settled, redo all included keyboards. This work
starts after the new model is implemented and verified. It does not opt existing
user projects into new links automatically.

At `f65ed01a6`, the generator/tests enumerate 20 included project outputs: measured
keyboard recipes, Sofle v2/RGB/Choc variants, the archived original REVIUNG41 and
the VIK review project. Re-inventory at execution time. Existing split recipes use
separate board IDs and do not cover the reused-PCB flipped-instance case; add a
bounded demonstration/verification for that model rather than claim it is covered
by rebuilding the current split examples. Most measured keyboard recipes currently
have no mechanical configuration, so this follow-up includes configuration work.

## What to do

- Inventory every shipped keyboard and review project from the Library catalogue
  and the native `prepare_demo_projects` recipe/output list. Cover all included
  keyboards, not only a selected demo. Record the inventory in the Outcome.
- Rework their reproducible recipes under `core/examples/demo_projects/` and
  measured inputs under `content/layouts/` where needed. Generate projects through
  the public Core editing/resolution/archive interfaces and the existing build
  route; generated archives are not the source of truth.
- Adopt the settled matrix placement meanings and explicit mechanical mirror pairs
  where applicable. Name and retain deliberate half-specific hardware/features;
  preserve measured layouts, legitimate wiring, authored geometry and source facts.
  Use independent configuration for keyboards that are not mirrored pairs.
- Regenerate all shipped project outputs, metadata and any dependent previews.
  Review intentional geometry changes per keyboard rather than silently accepting
  new auto-generated placements as measured/source evidence.
- Verify editable opening, matrix/paired mechanical edits, local exceptions,
  Undo/Redo and portable save/reopen. Check the affected PCB/case/plate output and
  fit findings. Retain unresolved hardware qualification as findings; rebuilding a
  keyboard does not establish physical fit or electrical qualification.

## Acceptance

- [ ] The inventory accounts for every shipped keyboard/review project.
- [ ] Regenerated outputs use the implemented matrix/mechanical model and its
  intended link/local states, with a per-keyboard record of intentional changes.
- [ ] Recipes reproduce the shipped projects; exported archives reopen correctly.
- [ ] Applicable mounted browser and live product checks pass, including paired
  editing and deliberate hardware asymmetry.
- [ ] The production build regenerates the full included-keyboard set successfully.
- [ ] Source/data changes receive the required Standards and Spec reviews.

## Verification

Run affected `python3 scripts/check.py` steps while iterating. Before accepting
the complete set or opening a PR, run the full check, including browser and build,
plus the per-keyboard app review above. Preserve result/source pins in the Outcome.
