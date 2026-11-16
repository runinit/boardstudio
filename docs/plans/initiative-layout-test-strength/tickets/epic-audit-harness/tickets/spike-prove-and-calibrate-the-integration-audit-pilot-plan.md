---
title: 'Prove and calibrate the integration-audit pilot'
type: chore
ticket: 2
created: '2026-11-16'
status: built
baseline_revision: b8d7011a259736ec4838161a095fc6aee0779706
route: full
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Necessist cannot discover Core's integration modules through their normal target mapping.

**Approach:** Calibrate a disposable-only target, audit only `core/tests/matrix_placement.rs`, and deliver measured evidence and a wider-audit recommendation.

## Boundaries & Constraints

**Always:** Obtain explicit approval for this plan, host-created checkout, fresh database, calibration and temporary removals before execution. Native-tool approval currently covers planning only; execution requires exact Gortex routing or separate native-tool approval. Deliver only this plan and epic Notes in the parent checkout.

**Never:** Audit another file, refactor production, change delivered mapping/layout/checks, close the epic, push or open a PR. Leave the existing DB unread/untouched and repair retrospective untouched; exclude both from commits. Commit no scaffolding, mapping, mutations or database.

## I/O & Edge-Case Matrix

| Scenario | Input | Expected result | Failure |
|---|---|---|---|
| Mapping | Temporary target | Named execution/counts | Missing tests block |
| Control | Sentinel act removed | Named failure detected | Blocks pilot |
| Pilot | Restored source/fresh DB | Eligible outcomes only | Blocker, not completion |
| Triage | Passing removal | Normal-target reproduction | Harness lead otherwise |

</frozen-after-approval>

## Code Map

- `core/Cargo.toml`, `core/tests/integration.rs`: `autotests = false`; normal target includes modules through `#[path]`. Preserve delivered mapping.
- `core/tests/matrix_placement.rs`: standalone-compatible; no ignore annotation observed. Exclude assertion-bearing helper calls: `axis_case`, `convert_case`, `internal_constraint_translation`, `assert_reflected_keys`, `assert_same_poses`, `assert_translated`.
- `README.md`, `.cargo/config.toml`, `scripts/check.py`: runners/build settings. Mapping/test files unchanged since research revision `14c276248bb8064c8f5d27760b5c33ac7ce63b53` (Git verified).

## Tasks & Acceptance

**Execution:**
- [x] Host `opencode.worktree_create`, name `integration-audit-pilot-2-2` -- record root as `P`; position only this clean checkout at baseline and verify HEAD. Copy no untracked files. Resolve target location; serialize builds/audit under FD9 `~/.cache/tmp/boardstudio-build.lock`, exporting `BOARDSTUDIO_BUILD_LOCK_FD=9`.
- [x] `P/core/Cargo.toml` -- temporary `[[test]]`: name `matrix_placement_audit`, path `tests/matrix_placement.rs`. Record versions and setup/build/cache cost separately.
- [x] `P/necessist.toml` -- exclude helpers/closures/ignored tests; retain default assertions. Inspect candidates. Disposable `P/.scratch/pilot-bin/cargo` may forward to `/usr/bin/cargo`, logging commands/output to parent `.evidence/integration-audit-pilot-2-2/`; verify pass/fail forwarding.
- [x] `P/core/tests/matrix_placement.rs` -- vector/clear/assert sentinel in `column_offset_moves_only_its_column`. Baseline passes; deleting `clear()` fails in both targets. Restore; exclude other tests for calibration and prove Necessist classifies `clear()` removal failed with its diagnostic. Use `--no-sqlite`.
- [x] Same test file/config -- remove sentinel, restore exclusions, compare source with baseline, list/execute the full file in both targets. Only after calibration allow creation of absent `P/necessist.db` and run the pilot.
- [x] Same test file -- examine every passing span/test; replay findings individually in normal `integration`, then restore/pass. Separate findings, leads, benign and stale results; reject ignored/helper artifacts. No repairs.
- [x] `../epic-audit-harness.md` Notes and this plan -- record setup/revision, named execution/ignored counts, exclusions, control, outcome counts, elapsed/setup time, completed removals per audit hour, setup-inclusive cost, findings per file, triage/recommendation and log paths. Blockers leave R2/R8 unsatisfied and epic open.

**Acceptance Criteria:**
- Given approved isolation, when calibration completes, then logs prove named execution and detected control before accepting pilot results.
- Given restored source/fresh DB, when auditing completes, then exclusions hold and findings reproduce in the normal target.
- Given completion/blocker, when delivering evidence, then cost/outcomes/recommendation are recorded and no temporary material is committed.

## Implementation Notes

Implementation and coordinating Build review complete. No production changes, delivered mapping changes, repairs, push or PR.

### Pilot evidence — 2026-11-16

- Disposable `P = /home/chris/.local/share/opencode/worktree/3ca6f9/integration-audit-pilot-2-2`, created with host `opencode.worktree_create`, then clean-detached at `b8d7011a259736ec4838161a095fc6aee0779706`. No parent untracked files copied. Target resolves to `P/target`; all builds/audit serialized under FD9 with `BOARDSTUDIO_BUILD_LOCK_FD=9`. Gortex rejected the disposable path as outside its indexed roots; used the explicitly approved native fallback. Parent documentation impact returned exact routing/LOW risk.
- Versions: Necessist 4.1.0, Cargo 1.98.0 (`797e8a9bc 2026-08-05`), rustc 1.98.0 (`88d9e12ae 2026-08-18`). Temporary target is `matrix_placement_audit`, path `tests/matrix_placement.rs`. Scratch Cargo forwarding preserved stdout/stderr and exit codes: named baseline exit 0, deleted control exit 101, and Necessist child diagnostics captured in `cargo-*.log`.
- Both targets list and execute the same 29 named tests: standalone 29 passed/0 ignored/0 filtered; normal integration 29 passed/0 ignored/874 filtered. Final restoration reran both with the same passing counts. Source equals Git baseline exactly after pilot/replays/diagnostics.
- Config retains default assertion exclusions and ignores all six helper names from Code Map. Also excludes the three assertion-closure-bearing tests `stagger_axis_matches_location`, `convert_preserves_poses_on_an_unsplayed_column`, `convert_preserves_poses_on_the_last_column_and_a_custom_origin`: the initial candidate dump showed that ignoring helper calls alone still visits nested closure arguments. These tests still run in both baseline targets, but their removals are not audited. No `#[ignore]` occurs in this file. The two `*_translation_preserves_internal_constraint_without_retuning` tests have zero eligible candidates because their assertion-bearing helper is excluded. Remaining pure data/request closures are not assertion helpers. No helper/assertion/control artifacts appeared in the pilot candidate dump.
- Calibration: add vector/clear/assert sentinel to `column_offset_moves_only_its_column`; baseline passes both targets; delete `clear()` and both named executions fail with `integration audit calibration sentinel`. Restore and use `--no-sqlite --verbose --deny all` with every other test excluded: 8 candidates, 2 failed/6 nonbuildable, including `audit_sentinel.clear();` classified **failed** with its child diagnostic in `cargo-1794850353826255677-2106851.log`. Then remove sentinel, restore pilot exclusions and verify source against Git before creating the absent checkout-local database.
- Pilot: `necessist --root "$P" --framework rust --verbose --deny all core/tests/matrix_placement.rs`, exit 0; 165 outcomes: **27 passed, 29 failed, 109 nonbuildable, 0 timed-out/skipped**. Header counts 26 non-excluded tests; 24 have candidates and two helper-only tests have none. Only the fresh `P/necessist.db` was inspected; parent DB remained unread/untouched.
- Cost: pilot 69.09 s = **8,597 completed candidates/audit hour**, counting nonbuildable outcomes; only 56 buildable outcomes = **2,918/hour**. Setup wall interval from host checkout's `.git` timestamp (17:27:31.251 UTC) to pilot start (17:33:39.539 UTC) is approximately 368.29 s, including inspection/calibration; setup-inclusive 437.38 s = **1,358 candidates/hour**. Initial named standalone execution 0.55 s and integration build/execution 24.90 s; deleted controls 2.20/9.13 s; Necessist calibration 5.52 s; restored list/run standalone 0.93/0.19 s, integration 6.37/0.20 s. No manual cache seed; no isolated cold-cache benchmark was measured, so warm-cache rates must not be extrapolated. Individual normal-target replay plus restored module run cost 140.29 s, separate from audit throughput. Final restore builds/runs cost 0.87/5.04 s. Tooling setup hiccup: `/usr/bin/time` absent (exit 127 before any build); reran with Bash `time -p`.
- All **27 passing spans** matched baseline text/coordinates and reproduced individually using normal `integration matrix_placement::<name> -- --exact --nocapture`, followed by source restoration and 29 module passes. No stale results, harness-only leads or confirmed defects; **0 findings/file**. Replays are `replay-01.log` through `replay-27.log` and `replay-summary.log`.

### Triage and recommendation

- **14 benign redundancies:** `column_stagger_moves_this_and_following_columns` lines 257/261 (max-length/reset); `convert_errors_on_a_splayed_default_origin_and_changes_nothing` lines 954/958/966/967/971 (already sized/defaulted/overwritten); roundtrip lines 1045/1049/1061/1062/1066/1074/1077 (equivalent max lengths, already-sized vectors and reset-before-overwrite). No behavior is lost by these removals.
- **13 setup-sensitivity leads, not defects:** roundtrip lines 1044/1052 (stagger configuration), 1075/1080 (row offset), 1083/1084 (cell offset/rotation); no-op line 1254 (authored marker); hold-origin line 1377 (row bend); rigid-align lines 1564–1567 (mirror/rotation/origin/row geometry); precise changed IDs line 682 (unrelated spare). Removing these weakens the fixture while relational/equality/negative assertions remain valid for the simpler input. Restored-source diagnostics confirm the intended nondefault fields, all mirror cases, authored marker and unrelated spare are actually present before the act. Therefore deletion sensitivity alone establishes neither an existing ineffective assertion nor a production bug. Optional future fixture-precondition guards are a hardening idea, not repairs delivered here. Evidence: `fixture-diagnostics-serial.log` (29 passes), with temporary prints removed afterward.
- **Recommendation:** mapping/control are trustworthy for eligible candidates, but do not launch a blanket wider audit from this result. Two-thirds of candidates are nonbuildable (109/165), helper/closure exclusion leaves explicit coverage gaps, and all 27 passing outcomes yielded zero confirmed defects after 27 normal-target replays. If separately approved later, prefer small high-risk files and inspect candidates first; measure setup and triage costs, not just warm audit throughput. This pilot is not production-mutation evidence and does not gate refactors. Epic remains open pending normal Build review/acceptance.
- Logs: repository-root `.evidence/integration-audit-pilot-2-2/` contains `calibration.log`, `candidates-initial.log`, `candidates-pilot.log`, `candidate-counts-pilot.log`, `control-{baseline,deleted}-{standalone,integration}.log`, `restored-{standalone,integration}.log`, `pilot.log`, `dump.log`, child `cargo-*.log`, replays, diagnostics, and `final-restored.log`. Logs only; no copied source trees, custom manifests or Cargo targets in `.evidence/`. Disposable mapping/config/database/scratch remain uncommitted in P for evidence; parent retrospective and existing database were not read or changed.

- 2026-11-16: User explicitly approved the plan, disposable checkout, fresh database, calibration, audit/removals and native execution if Gortex remains unavailable. Plan re-read unchanged before approval was recorded.

## Plan Change Log

## Review Triage Log

- 2026-11-16: Independent quick review returned no findings; no patches or deferred review findings. Evidence limitations and the recommendation against an automatic wider audit remain recorded above. Epic remains open for human acceptance.

## Design Notes

No shell-created worktree. Separate calibration from pilot yield. Necessist measures statement deletion, not production mutations. Report tests with zero eligible candidates due to helper exclusions.

## Verification

- Coordinator inspected the delivered diff, final restored passes, calibration classification, all 27 individual replay pass records and the full-check completion summary. Only plan/epic documentation changed in the parent; disposable Git diff contains only the approved temporary target mapping.

- 2026-11-16: Parent full check under FD9 **passed all six steps**, exit 0: repo 7.1 s, tooling 36.8 s, build 69.5 s, lint 75.7 s, test 58.5 s, browser 801.6 s; scheduler total 921.3 s (shell wall 921.48 s). Log: repository-root `.evidence/integration-audit-pilot-2-2/full-check.log`. No unexpected failing verification command; the deliberate controls and nonbuildable audit outcomes are isolated experimental evidence, not project-check failures. Subsequent coordinating edits only finalize plan status and review evidence.
- Matrix audit: mapping covered by matching 29-name list/execution in both targets; control covered by two detected named failures and Necessist failed classification; restored-source/fresh-DB pilot covered by source comparison and 165 terminal outcomes; triage covered by 27 individually reproduced normal-target passes and restored 29-test pass. No ignored/helper outcome accepted as behavior evidence.

- `P`: `cargo test -p boardstudio_core --test matrix_placement_audit -- --list`, then without list; compare with `cargo test -p boardstudio_core --test integration matrix_placement::`. Record passed/ignored/filtered counts.
- Replay: `cargo test -p boardstudio_core --test integration matrix_placement::<name> -- --exact --nocapture`; standalone uses the unprefixed name. Require expected failure/restored pass.
- `P`: `necessist --root "$P" --framework rust --no-sqlite --dump-candidates core/tests/matrix_placement.rs`; calibration uses `--no-sqlite --verbose`, pilot `--verbose`, both `--deny all`. Verify installed flags after approval. Require child-output execution evidence.
- Parent: inspect diff; run `python3 scripts/check.py` under FD9 without short timeout; logs in `.evidence/integration-audit-pilot-2-2/`. Quote unexpected failures without unsupported attribution. Follow Build review; stage only plan/epic Notes.
