# 24: Audit the tests after the typed-edit, matrix and mirrored-half changes

Status: resolved
Type: task
Blocked by: [18](18-core-create-mirrored-half.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Request

User, 2026-10-10: "we have a lot of tests which are a bit pointless after all the changes we
made. We need a full audit." Over this branch we replaced the matrix placement meanings
([ADR-0007](../../../adr/0007-matrix-gestures-edit-named-parameters.md)), the closure rules, the
mirror arithmetic and the document versions. Some tests pin replaced mechanisms, some
duplicate others, and a few fail on a version number alone (boundary transcript lines,
literal-version fixtures). The full check is about ten minutes, so each test must earn its time.

## What to do

- Inventory every test (Rust native and wasm, Python tooling, browser) and its time:
  `cargo test -- --report-time` or per-binary timings from `check.py`, wasm suite timings,
  the 15 CAD checks. Record the slowest 30 and each suite's total.
- Classify each test or group: **protects real behaviour**, **duplicates** another, **pins a
  replaced mechanism**, **pins incidental output** (golden noise), **trivial** (asserts what
  the compiler or a type already guarantees), **slow for what it proves**.
- Propose, per class, delete / merge / rewrite as a behaviour test / keep. Do not delete a
  test that is the only guard of a real behaviour; for goldens (boundary transcript, demo
  baseline, footprint goldens) decide what the minimum pinned surface is.
- Execute the approved deletions and merges in small commits, each with before/after counts
  and time, mutation-checking that coverage of the behaviours it protected survives
  (disable the code under test, confirm some test still fails).
- Look for tests that exist only because of the old design: matrix toolbar stagger paths,
  per-key closure suggestions, recipe mirror arithmetic checks, version-literal assertions.

## Acceptance

- [x] A written audit (counts, time, classification, decisions) is recorded in the audit and phase 2 results.
- [x] Merged tests and retained guards are listed with the behaviours they protect.
- [x] Full check time before and after is recorded; merged scenarios remain covered.
- [x] The user approved the phase 2 action list before execution.

## Verification

Per [verification-process.md](../../../agents/verification-process.md); this is a batch-boundary
ticket, so run the full `check.py` (never `release`) before and after.

Order: schedule after 19 so the new mirrored-half tests are part of the audit, and before 22.

## Outcome (phase 1: audit and proposal only; nothing deleted, edited or merged)

Sonnet 5.5, 2026-10-10, at `92d08fbf2`. Written audit: [test-audit.md](../audit/test-audit.md).

- Inventory: about 2,150 tests (1,407 native Rust, 619 wasm/browser including the 15 CAD smoke checks, 124 Python)
  and about 14 MB of goldens, with counts per binary and per core integration module.
- Time: `check.py test` 74.5 s and `check.py browser` 159.1 s (both warm, sandbox lifted for the CAD container and
  Chrome). Native per-test times from libtest `--report-time` (`RUSTC_BOOTSTRAP=1` at run time, no rebuild); wasm
  has only per-suite and filtered-run times. The slowest 30 are in the audit.
- Finding: 505 of 546 core integration tests run under 0.5 s and the 418 fastest sum to 1.7 s. The `test` step is
  bounded by shipped-keyboard recipe builds done twice in debug (`demo_projects` 58 s, and the shared
  `included_keyboards::projects()` about 50 s, awaited by 15 tests), and `generator_export` KiCad parsing (15 s).
  The browser step is bounded by fixed real-time sleeps in the mounted tests (keymap `settle`, case `rendered`).
- Classification and a proposed action list (D1 to D8, M1 to M3, R1) with, per item, the behaviour protected and the
  surviving test; speed-only options; per-step time estimates (labelled as estimates).
- Not measured: per-test wasm times, the effect of `opt-level` or parallel `prepare`, the `build` step, the full
  `check.py` before/after (phase 2), and bodies outside the groups the audit marks as read.

**Checkpoint for the user (HITL):** approve, change or reject the action list in section 6 of the audit, and say
whether the speed-only options in section 7 (especially options 1 and 2, which change test helpers or the dev
profile) are in scope. Phase 2 executes only the approved items, in small commits with before/after counts and a
mutation check for each.

## Phase 2 (user approved the audit's recommendations, 2026-10-10)

The user approved everything the audit proposed, including the section 7 speed changes. Phase 2 is split into
tickets [33](33-test-speed-rust.md), [34](34-test-speed-browser.md) and [35](35-test-maintenance-cuts.md); this ticket resolves when 35 closes
the audit. The recipe-reflection tests (audit R1) stay until ticket 23.

## Scope reconciliation (2026-10-14)

The later instruction to remove built-in-board work from scope defers D8; no baseline changes were
made. D5/D7 are committed as `84930356`; D6's helper is absent from this checkout. M2 exact Keymap
twins were merged by [38](38-merge-duplicate-browser-tests.md). The #37 wait replacements and
mutation checks are recorded in that ticket. M3 was reviewed and retained: the Layout component
Inspector and legacy position-draft tests use distinct controls, commit paths, and failure boundaries.

## Phase 2 results (2026-10-14)

- **D5/D7:** shipped in `84930356`; boundary transcript fell from 1,143,772 to 108,125 bytes
  (90.5% smaller), with native and WASM parity. D6 had no remaining helper to remove. D8 remains
  deferred by the user's built-in-board scope change.
- **M1 / #37:** fixed waits were replaced with bounded state waits across the ticketed suites;
  negative waits were mutation-checked against both predicates and production actions. Residual
  Parts timers total 790 ms, below the ticket's roughly one-second cutoff. The final baseline/current
  test-name diff and current-tree repeated/full gates are recorded in #37.
- **M2 / #38:** five exact Keymap pairs are table-driven: four keycode/delay pairs and one macro
  text/number pair. Ten names became five. Similar-looking layer-name and position-draft cases were
  retained where owners, commit paths, or failure boundaries differ.
- **Name audit:** native test count is governed by `check.py test`; WASM baseline/current names are
  in `.evidence/ticket37-test-names/name-diff.json`. Added tests are feature coverage from the mirror
  and companion work; the one shipped-demo test was retired under the later scope instruction.
- **Gate boundary:** the default full `python3 scripts/check.py` passed after D5/D7 before this final
  #41 test-only expansion; the final full gate after #41 is still required. `release` was not run.

The final current-tree full check passed in 197.0 s. No test was removed solely as “trivial” or on a
names-only duplicate guess. The separate integrated mirror review work remains in #22.

## Comments

- Unclaimed 2026-10-12 at the user's request; status back to `ready-for-agent`. Existing work on this ticket's branches or worktrees was not touched.
