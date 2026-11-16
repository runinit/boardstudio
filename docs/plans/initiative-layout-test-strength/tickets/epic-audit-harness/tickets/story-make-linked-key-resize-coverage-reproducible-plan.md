---
title: 'Make linked key-resize coverage reproducible'
type: chore
ticket: 1
created: '2026-11-16'
status: done
baseline_revision: 'a2be9c7b3067ca368e4e4459f2f3125c5b608ac8'
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

**Problem:** Linked keycap-size reflow coverage is ignored because it requires an external project archive. Its history oracle checks only one source key, leaving linked-half restoration unproved.

**Approach:** Replace that fixture with a small synthetic linked document in the existing native test module. Exercise accepted Core geometry, the real resize planner and typed `SetKeycapSize` commit, then verify source reflow, linked state and history for both halves.

## Boundaries & Constraints

**Always:** Keep fixture/helpers local to `keycap_resize.rs` tests. Derive inputs from Core-accepted documents/scenes; use explicit linked matrix correspondence, not geometric mirroring alone. Preserve existing behavioral assertions while strengthening identities and history. Capture nonempty cell/member mappings before editing. Run without an archive or environment variable and without ignore annotations. Preserve unrelated edits/history, including the retrospective and unread `necessist.db`.

**Never:** Change production behavior, refactor unrelated code, weaken assertions, change integration-target mapping, run Necessist or ticket 2.2, create/switch checkout or branch, stash/reset/rebase, push or open a PR. A failing repaired test against unchanged production blocks completion for separate bug investigation.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Accepted resize | One row, three keys on each explicitly linked half; middle source key grows from 1u to 3u | Width 18→56 at pitch 19/gap 1; selected center fixed; source neighbors move outward 19; partner sizes and poses reflect source | Core rejection fails test |
| Stable pairing | Six accepted members mapped by matrix/row/column | Same source/partner IDs and memberships after commit and history; neighbor sizes unchanged | Missing or duplicate members fail test |
| Undo / Redo | One committed resize | Undo restores both halves' relevant accepted state; Redo restores both halves' committed state, excluding revision | Missing history or partial restoration fails test |

</frozen-after-approval>

## Code Map

- `web/crates/layout/src/objects/keycap_resize.rs`: ignored `replace_document_reflows_source_and_core_syncs_linked_partner_through_history` already uses typed `SetKeycapSize`. Reuse `matrix`, `document`, `point`, accepted placement extraction and Core reply matching. Replace archive setup and strengthen its existing oracles; do not edit `plan_resize` production logic.
- `web/crates/layout/src/part_placement.rs`: `linked_target_key_replacement_stays_local_through_core_sync_and_history` demonstrates two layouts/matrices, explicit `LayoutMirrorLink` Matrix correspondence and real Core Open. Reference only; no edits.
- `keycap_resize.rs`: `accepted_rotated_and_mirrored_resize_preserves_assemblies_through_history` supplies a local switch definition and accepted Core setup pattern. Geometric `Mirror::X` there is not linked-half coverage.
- `README.md`, `scripts/check.py`, `web/crates/layout/Cargo.toml`: native package is `boardstudio-web-layout`; ordinary full check includes native workspace tests. No native guard entry currently names the ignored test, so retain its name to avoid unnecessary baseline churn.

## Tasks & Acceptance

**Execution:**
- [x] `web/crates/layout/src/objects/keycap_resize.rs` — replace external setup with local definition, board, two three-cell matrices and explicitly linked layouts; remove archive docs/read and ignore annotation.
- [x] Same file — preserve real planner/typed commit path; assert expected nonzero source reflow, selected center, all paired sizes/reflected poses, stable cell→member identity and document/scene agreement. Add a small relevant-state snapshot for both halves' members, poses, sizes and matrix cell offsets; compare Undo to accepted and Redo to committed state.
- [x] Same file (temporary controls only) — demonstrate unchanged oracles reject missing reflow, stale partner state, skipped Undo and skipped Redo; restore each control before the next normal run.
- [x] This plan — record commands, named execution/ignored counts, counter-example failures, verification and review outcomes concisely.

**Acceptance Criteria:**
- Given only the repository, when the named test runs under the ordinary native gate, then it executes once, is not ignored, needs no external data, and proves the resize/reflow/pairing scenarios above.
- Given the accepted linked fixture and a committed resize, when Undo then Redo execute, then both halves' relevant state matches the corresponding full snapshot and all six pre-edit member identities survive.
- Given each deliberately broken test act, when the unchanged assertions run, then the expected behavioral assertion fails; restoring the act returns the test to green.
- Given the restored final changes, when the full project check and Build review run, then verification passes and no unresolved actionable review finding remains; only test code and this plan are delivered.

## Implementation Notes

- Replaced the archive/environment/ignore setup with six local switch members, two 1×3 matrices at pitch 19/gap 1, and two layouts with explicit Matrix correspondence. Fixture members use Core's matrix cell IDs; all planner inputs and pre-edit identity/state snapshots come from the accepted Open reply.
- The unchanged typed resize route now checks numeric 18→56 widths, source displacements −19/0/+19, all partner sizes/reflected poses, six unique stable cell memberships, document/scene pose agreement, and both halves' member/pose/size/offset snapshots through Undo/Redo.
- Initial fixture-construction probes failed on missing accepted members (0 scene cells with empty part IDs, then missing member IDs with arbitrary a–f IDs). Corrected test setup to use valid generated matrix member IDs; no production behavior failure or production edit.
- Frontmatter `context: []` contains no additional context files. Referenced fixture patterns and command configuration were read through exact Gortex routing.
- Temporary controls were individually applied and restored; no control remains. Rustfmt changed only the repaired test.

## Plan Change Log

## Review Triage Log

- 2026-11-16 — Quick inspection review: no findings (high 0, medium 0, low 0, false 0, maybe-false 0); no patches, loopbacks or deferred work. Parent inspected the unified diff and execution logs; all three I/O matrix rows are covered by the executed repaired test.
- Parent reruns: named test **1 passed, 0 failed, 0 ignored** (`.evidence/linked-resize-parent-named.log`); module **10 passed, 0 failed, 0 ignored** (`.evidence/linked-resize-parent-module.log`). `git diff --check` passed. Final diff remains limited to the repaired test and this plan; production and neighboring tests are unchanged.

## Design Notes

Estimate: more than 100 changed lines (fixture plus complete paired/history oracle), so full route. Low business risk: one test module and its plan, no product behavior change. Keep expected size and outward displacement numeric rather than reproducing planner calculations. Snapshot identities and state from accepted Core results, not planner predictions. Counts prevent empty loops or filtered/ignored tests from masquerading as coverage.

## Verification

### Execution evidence (2026-11-16)

All commands below ran serialized under the specified FD-9 lock, with no other Cargo/wasm/Parts/check process observed before execution. Logs are under `.evidence/`.

- Named command exactly as listed below: **1 passed, 0 failed, 0 ignored, 168 filtered out** (`linked-resize-named.log`).
- Control 1 retained sizes but sent empty moves/offsets: **0 passed, 1 failed, 0 ignored**; failure **"source column 0 must reflow by -19"** (`linked-resize-control-reflow.log`).
- Control 2 replaced committed partner parts/matrix/scene with accepted pre-resize partner state, leaving the source untouched: **0 passed, 1 failed, 0 ignored**; failure **"linked partner reflection at column 0"** (`linked-resize-control-partner.log`).
- Control 3 bypassed Undo by reusing the saved document/scene: **0 passed, 1 failed, 0 ignored**; failure **"assertion `left == right` failed: Undo restores both halves"** (`linked-resize-control-undo.log`).
- Control 4 bypassed Redo by reusing the undone document/scene: **0 passed, 1 failed, 0 ignored**; failure **"assertion `left == right` failed: Redo restores both halves"** (`linked-resize-control-redo.log`).
- Each control was restored before the next control, and each restored named run passed **1 test, 0 failed, 0 ignored** (`linked-resize-restored-{reflow,partner,undo,redo}.log`). Assertions were unchanged throughout controls.
- Module command exactly as listed below: **10 passed, 0 failed, 0 ignored, 159 filtered out** (`linked-resize-module.log`).
- First full `python3 scripts/check.py` invocation was terminated by the tool's 120-second timeout (SIGTERM/143), interrupting build/lint after the native test stage passed. Native workspace log explicitly shows the repaired named test `... ok`; all 2429 baselined native tests executed and 5 new were reported. This is an incomplete full check, not a passing gate or a test failure (`linked-resize-full-check.log`). No-timeout rerun **passed all six steps: repo, tooling, build, lint, test, browser** in 406.3s (`linked-resize-full-check-retry.log`). It explicitly executed the repaired named native test once (`... ok`), executed all 2429 baselined native tests with 5 new reported, and finished 263 browser tests with 0 failures. No native baseline or integration mapping was changed. Final `git diff --check` passed; only the test file and this plan were edited, and the pre-existing retrospective and unread `necessist.db` remain untouched/untracked.
- Gortex detect identified only the test module/test function; guards reported no rules, contract allowed the change, graph test discovery found no covering test edges (the explicit native runs above supply execution evidence).
- Historical implementation-delegate handoff (2026-11-16, superseded): “Build review remains the parent session's next workflow step; implementation delegate has not marked the story built or created a Git commit.” Final state: parent review completed with no findings (Review Triage Log above); implementation committed as `92ac157c2de936b71df297a60212fd7b3e274e67`, then accepted as done in `356b1560ad9b5ed33f24ff6ef9ba715fb5073853`. This label correction applies retrospective A4 only; frozen intent, verification evidence and done status are unchanged.

All build/check invocations acquire `~/.cache/tmp/boardstudio-build.lock` via FD 9 and export `BOARDSTUDIO_BUILD_LOCK_FD=9`; serialize invocations and preserve repository concurrency/debug settings. No Parts build was running during planning; recheck before execution. Long output goes to `.evidence/` logs only.

**Commands (inside that lock):**
- `cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests::replace_document_reflows_source_and_core_syncs_linked_partner_through_history -- --exact` — one named pass, zero ignored; repeat after every restored control.
- Same filtered command with each temporary test-act control: (1) clear resize moves and offsets while retaining sizes; (2) substitute accepted pre-resize partner state for the committed partner at the reply boundary, preserving source state; (3) bypass Undo; (4) bypass Redo. Expect respectively concrete reflow, partner, Undo and Redo assertion failures, never mere compile/setup failure. Controls are not committed.
- `cargo test -p boardstudio-web-layout --lib objects::keycap_resize::tests::` — all module tests pass; record counts.
- `python3 scripts/check.py` — full project check passes and includes the named active native test. Quote unexpected failures accurately; passing retries do not establish Parts flake causation or baseline attribution.
- Inspect final diff/status: no production changes, temporary controls or unrelated files staged; both pre-existing untracked files untouched.
