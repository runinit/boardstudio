# 005: Old-CSS check

Status: resolved
Type: build
Blocked by: [001: Walking skeleton: one copied Button, both Themes, one migrated control](001-walking-skeleton.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [branch plan](../../rust-ui-migration/branch-plan.md) section 4, [shared contract](../map.md#shared-contract)

## What to build

A repository check reports how many `.board-*`, `.m1-*` selectors and `--wb-*` variables remain, so progress is visible during the migration and the final merge can be blocked on zero.

## Acceptance criteria

- [ ] `scripts/check.py` has a step that counts remaining `.board-*`, `.m1-*` and `--wb-*` uses in the web sources and styles; the baseline today is 622 selectors and 80 variables
- [ ] The default run reports the counts without failing; a strict mode fails while any remain, and the README lists both
- [ ] The step is covered by the repository check-script tests that already exist for `check.py`
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

Resolved 2026-10-22 on `lane/005-old-css-check`.

**What changed**
- `scripts/check-old-css.py`: counts distinct `.board-*` / `.m1-*` selectors and `--wb-*` variables in every text file under `web/` (build output dirs skipped). Default run prints the counts and passes; `--strict` exits 1 while any remain.
- `scripts/check.py`: `repo` step runs `test-check-old-css.py` and the default report; new opt-in step `old-css` runs `check-old-css.py --strict` (not in the default run; the final merge gate).
- `scripts/test-check-old-css.py` (5 tests, registered in the `repo` step so the existing registration test covers it).
- `README.md` Checks: documents the report and the strict step.
- No `web/` code touched.

**Count differs from the ticket's baseline.** Measured now: 670 selectors and 82 variables, not 622 and 80. The gap did not come from the scope or regex I tried (per-file, css-only, Rust-only, lookbehind variants all gave 631-676 and 82). The baseline was probably measured on a different tree or definition. The strict gate's numbers come from the script, so the plan's 622/80 is stale and should be refreshed when the gate is wired.

**Commands and results**
- `python3 scripts/check.py repo`: pass (includes the new script test and report).
- `python3 -B scripts/test-check.py` (31 tests) and `test-check-quick.py`: pass.
- `python3 scripts/check.py old-css`: fails as designed (670 / 82 remain).
- `python3 scripts/check.py quick`: the lane's `scripts/check.py` change is a shared input, so it ran every Rust check. Lint, typecheck, native tests (2104 baselined executed), dx build and the CAD checks passed. The `browser` step failed in `cad/scripts/test-cad-browser.py` ("page error" from the CAD worker at startup). The same test fails on the unchanged `feat/rust-ui-migration` tip (`b9145709f`) and passes on the `dev` checkout, so it is not caused by this ticket. Follow-up for whoever owns the CAD worker on the migration branch.
- Evidence: `.evidence/005-*.log` in the lane worktree.
