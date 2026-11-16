---
epic: epic-audit-harness
date: '2026-11-16'
verdict: accepted-with-open-items
criteria: declared
headless: false
---

# Audit harness retrospective

## Epic summary

User selected Audit harness interactively. No additional concerns supplied; examine the pilot's helper/closure exclusions against R2/R8.

- 2.1: done.
- 2.2: built; awaits human acceptance.
- Pending unfinished tickets: none. `built` counts as finished for this retrospective, not as human acceptance or permission to close the epic.
- Review HEAD: `086fa1f21725e3868460a66b328328b9ad5fdb13`.

### Evidence inventory and ranges

- Spec: [epic](epic-audit-harness.md), Requirements R1/R2 and Done when; [entry list](tickets/tickets.toml), entries 1/2. No separate story files, companions or run folders exist. Epic restates parent R7/R8, so no source research re-derivation needed.
- Plans: [2.1](tickets/story-make-linked-key-resize-coverage-reproducible-plan.md) (done) and [2.2](tickets/spike-prove-and-calibrate-the-integration-audit-pilot-plan.md) (built), read fully, including frozen intent, implementation, review and verification evidence.
- 2.1 range: `a2be9c7b3067ca368e4e4459f2f3125c5b608ac8..b8d7011a259736ec4838161a095fc6aee0779706`, derived from successive plan baselines. Three commits: implementation `92ac157c2de936b71df297a60212fd7b3e274e67`, acceptance `356b1560ad9b5ed33f24ff6ef9ba715fb5073853`, and unrelated repair-epic closure `b8d7011a259736ec4838161a095fc6aee0779706`. Exclude the latter's three repair-document lines from this epic's attribution/review.
- 2.2 range: `b8d7011a259736ec4838161a095fc6aee0779706..086fa1f21725e3868460a66b328328b9ad5fdb13`; last endpoint inferred from current HEAD/last epic commit, not recorded by its plan. One documentation commit.
- `git_evidence.py` ran for each distinct range with `--stories 2.1` / `2.2`. No merges or binary revisions; commit subjects contain no story refs, which is not missing evidence. Non-merge churn: test file +209/−262/net −53, story plan +107/−1/net +106, spike plan +109/net +109, epic Notes +7/net +7. No duplicated source change counted.
- Review diff: `/home/chris/.cache/tmp/opencode/audit-harness-epic-80bduits.diff`, committed changes in the epic folder and changed test only. The new retrospective is not part of its own review.
- Verification logs: `.evidence/linked-resize-*.log` and `.evidence/integration-audit-pilot-2-2/`, including named execution, deliberate controls, restored runs, pilot candidates/outcomes/replays and full checks. Primary-log verification completed within the explicit scopes below.
- Session evidence: this thread records pilot approvals, native fallback authorization, delegation and completion. Full story-2.1 session transcript is not loaded; its process account is limited to sourced plan/log records. No inferred root causes from missing dialogue.
- Protected evidence: existing parent `necessist.db` and prior repair retrospective remain unread/untouched. Prior-retro follow-through is narrowed accordingly.

## Findings

### Aggregate structure (checked; no blocking defect found)

- Architecture: only `92ac157c2de936b71df297a60212fd7b3e274e67` changes source, confined to one existing test. Its seam remains Core Open → `plan_resize` → typed `SetKeycapSize` → Undo/Redo. The function-local import at `web/crates/layout/src/objects/keycap_resize.rs:1406` adds three existing Core model types; production imports, module graph, APIs, dependencies and delivered mapping are unchanged. External environment/filesystem fixture reads disappear.
- Size: direct `git show` measurements at the two range endpoints confirm `keycap_resize.rs` shrinks 1,717→1,664 lines and 62,168→61,534 bytes. Ten test declarations remain; ignore annotations decrease 1→0. No source-growth finding follows from documentation churn. Scope: changed file, not a repository-wide size audit.
- Duplication/patterns: the replacement reuses local `matrix`, `document`, `part`, `point` helpers (`keycap_resize.rs:663` onward). Its synthetic definition/unpack/accepted-placement scaffolding resembles the neighboring single-matrix assembly test (`:1252`) and linked `SetMatrix` fixture in `part_placement.rs:3769`; behavioral contracts differ, so this is not duplicate coverage. No whole-repository clone scan was performed.
- Oracle: local membership/snapshot closures (`keycap_resize.rs:1487` onward) verify six identities, document/scene pose agreement and paired pose/size/offset history. They intentionally compare relevant state rather than the whole document, as the approved plan requires (story plan:28,53,59). No production behavior is inferred from this structural observation.
- Naming debt: the retained `replace_document_…` name describes an older act although the test already used typed `SetKeycapSize` before this epic. **Instance: accept as-is**, pre-existing and deliberately retained; **prevention: none**, no rename or baseline churn required.

### Spec reconciliation (checked)

| Declared criterion | Primary evidence | Reconciliation |
|---|---|---|
| R1 / parent R7: ordinary synthetic linked resize | `keycap_resize.rs:1404–1663`; story plan:65–76; named ordinary-gate pass in `linked-resize-full-check-retry.log:1321` | Archive/env/ignore dependency removed; Core accepts explicit Matrix correspondence and six unique pre-edit members. Numeric widths 18→56 and source displacement −19/0/+19, partner sizes/reflected poses and paired history snapshots cover the named contract. |
| R2 / parent R8: validate mapping/execution/control before results | `calibration.log:1–14`; child `cargo-1794850353826255677-2106851.log:3–26`; `final-restored.log:5–36,45–76` | Child actually executes the named control and fails its assertion, exit 101; Necessist classifies the clear statement failed. Both restored targets execute identical 29-name sets, zero ignored. |
| Exclude ignored/assertion-helper removals | Disposable `necessist.toml:1–2`; spike plan:74–76; candidate dumps | Six helper names and three closure-bearing tests excluded. The latter still execute normally but have no removal audit; two helper-only tests have zero candidates. No source ignored tests. This is an explicit sensitivity boundary, not blanket coverage. |
| Reproduce findings through ordinary integration | `replay-01.log` through `replay-27.log`; `replay-summary.log`; restored source compared to Git | Every passing removal executes/passes its exact normal-target test with zero ignored; no stale spans. All 27 examined: 14 benign redundancies, 13 fixture-sensitivity leads, zero confirmed defects/findings. Diagnostics verify original nondefault inputs; no speculative repair promoted. |
| Recorded measured result/recommendation | Epic Notes:62–64; spike plan:77–85; `pilot.log`, `replay-summary.log` | 165 outcomes = 27 passed +29 failed +109 nonbuildable, 69.09 s. Independently recalculated 8,597 total outcomes/hour, 2,918 buildable/hour, 1,358/hour including approximate setup interval. Zero findings/one file; wider audit not recommended automatically. |
| Full check and scope preservation | `linked-resize-full-check-retry.log:5974`; pilot `full-check.log` final summary; committed Git diff | Both recorded full checks pass all six steps. Current delivered source remains the same test-only delta; pilot mapping/scaffolding/database never committed. Epic remains open. |

The parent independently parsed both 29-name sets, all 27 replay pass records and pilot terminal outcomes, and compared disposable `matrix_placement.rs` bytes to the recorded baseline. No original database was read.

**Accepted limitations, not unfinished criteria:** three tests excluded from removals, two helper-only zero-candidate tests, 109/165 nonbuildable outcomes, warm-cache timing, approximate setup interval, no isolated cold-cache or separately measured human-triage duration, and no demonstrated wider-audit yield. **Instance disposition: accept as-is** because the declared spike calls for a calibrated bounded pilot and honest recommendation, not complete removal sensitivity or a wider run. **Prevention:** any separately approved extension should state its candidate/exclusion boundary and measure setup/triage cost, rather than extrapolate the warm rate (epic Notes:64; spike plan:74–84). No new work is authorized here.

### Diff-scope review

`bmad-review` ran on the combined committed epic diff with adversarial, edge-case-hunter and verification-gap lenses, weighting ticket boundaries. Edge-case-hunter and verification-gap each returned `[]`. Adversarial returned ten reports, all preserved below and rechecked against primary source. No result is treated as a confirmed production defect.

#### Lens findings (canonical fields; no severity ranking)

```json
[
  {"lens":"adversarial","location":"web/crates/layout/src/objects/keycap_resize.rs:1615-1644","trigger_condition":"Reflection uses the post-edit mirror axis.","guard_snippet":"Capture the accepted axis and compare link metadata across the edit.","potential_consequence":"A coordinated axis/partner-placement error can satisfy the pose relation."},
  {"lens":"adversarial","location":"web/crates/layout/src/objects/keycap_resize.rs:1516-1548","trigger_condition":"History snapshots omit layout ownership and mirror-link correspondence.","guard_snippet":"Include ownership and link metadata in relevant-state snapshots.","potential_consequence":"Metadata restoration errors can escape the geometry/member oracle."},
  {"lens":"adversarial","location":"web/crates/layout/src/objects/keycap_resize.rs:1648","trigger_condition":"Linked rotation reflection is asserted only with zero rotations.","guard_snippet":"Add a nonzero linked rotation case or bound the evidence claim.","potential_consequence":"Copied and reflected nonzero rotations are not distinguished."},
  {"lens":"adversarial","location":"Story 2.1 plan:89,103","trigger_condition":"Reflow control removes moves and offsets together.","guard_snippet":"Separate omissions or describe complete-missing-reflow sensitivity only.","potential_consequence":"The control does not independently establish sensitivity to each payload channel."},
  {"lens":"adversarial","location":"Story 2.1 plan:90,103","trigger_condition":"Combined stale partner control first fails at column-zero reflection.","guard_snippet":"Use a size-only control or distinguish inspected size assertions from demonstrated pose failure.","potential_consequence":"A failing pose oracle alone does not prove experimental sensitivity of the size oracle."},
  {"lens":"adversarial","location":"web/crates/layout/src/objects/keycap_resize.rs:1605-1608","trigger_condition":"Planner supplies partner size in the Core payload.","guard_snippet":"Describe end-to-end planner/Core coverage, not isolated Core propagation.","potential_consequence":"Independent Core size-propagation behavior is not isolated by this test."},
  {"lens":"adversarial","location":"Spike 2.2 plan:72-76,85","trigger_condition":"Exact temporary config and Cargo forwarding text are not preserved in committed evidence.","guard_snippet":"Record exact configuration/reconstruction and forwarding behavior as inert evidence.","potential_consequence":"Reproduction becomes dependent on the disposable checkout surviving."},
  {"lens":"adversarial","location":"Spike 2.2 plan:77","trigger_condition":"Setup-plus-audit throughput excludes replay and human-triage duration.","guard_snippet":"Also give the recorded-phase lower bound and label triage unmeasured.","potential_consequence":"The narrow throughput can be mistaken for delivery-inclusive throughput."},
  {"lens":"adversarial","location":"epic-audit-harness.md:66","trigger_condition":"Earlier same-day decision still says authorization is pending and no pilot started.","guard_snippet":"Label that historical statement as superseded.","potential_consequence":"Readers can misread prior state as current authorization/execution state."},
  {"lens":"adversarial","location":"Story 2.1 plan:97","trigger_condition":"Accepted plan retains a delegate statement that review/commit are pending.","guard_snippet":"Label the delegate handoff historical and link final review/commit evidence.","potential_consequence":"A completed record appears unfinished when read out of sequence."}
]
```

#### Parent verification and dispositions

| Report | Verified evidence and bounded conclusion | Instance disposition | Upstream prevention |
|---|---|---|---|
| 1 | `keycap_resize.rs:1615–1623` reads `saved` axis; it does not assert equality to the accepted axis. This is an actual oracle independence gap, not an observed product failure. Current relation/source-displacement assertions still run. | **Defer**, optional hardening A1. No new blocking axis-invariance requirement is inferred. | Capture stable expected metadata before the act when that invariance is intended. |
| 2 | Snapshot tuple `:1541–1548` includes matrix/row/column/id/pose/size/offset, not layout/link metadata. Story plan:53 explicitly selects this relevant-state subset; it fulfills that bounded history check, not exhaustive document restoration. | **Defer**, optional hardening A1. | Name the snapshot's protected fields; do not call it whole-document equality. |
| 3 | Helpers `:674–676,697–698` and fixture `:1410–1430` start with zero/no rotation; assertion `:1648` cannot distinguish nonzero copying from reflection. Neighboring single-matrix coverage does not close this linked-half case. | **Defer**, optional hardening A2; existing translated/zero-angle linked scenario is valid. | Label fixture-specific evidence; add nonzero cases only through separately approved work. |
| 4 | Plan:89/103 and reflow log:13 match the combined omission exactly; no independent moves-only or offsets-only proof is claimed there. | **Accept as-is**, evidence limited to complete missing reflow. | Do not generalize one composite control into multiple independently proven channels. |
| 5 | Plan:90 and partner log:13 explicitly identify the first failure as column-zero reflection. Numeric selected-partner size assertions exist at source:1637–1641, but their isolated negative control was not run. | **Accept as-is**, inspected size oracle plus experimentally detected stale partner poses, not separate size-control proof. | State the first failing oracle; avoid overclaiming later assertions were reached. |
| 6 | `plan_resize:243–258` inserts partner size; the test submits `plan.sizes` at :1606. Frozen approach (story plan:24) deliberately specifies the real planner plus typed Core seam. | **Accept as-is**, this is end-to-end planner/Core linked-size acceptance, not isolation of Core's source-only propagation. | Keep seam names precise. |
| 7 | Disposable `necessist.toml:1–2` and `.scratch/pilot-bin/cargo:1–23` exist and were read. The committed plan names flags, versions, helper/test exclusions and forwarding behavior, but not exact text/reconstruction. This is retention/reproducibility debt, not an invalidated completed run. | **Defer**, A3, before disposable cleanup or a future rerun. | Preserve inert reconstruction evidence without committing executable scaffolding/databases. |
| 8 | Plan:77 and epic:63 already separate 437.38 s setup+audit from another 140.29 s replay. Arithmetic lower bound is **577.67 s**, about **1,028 candidates/hour** across those recorded phases. Human triage and other finalization remain unmeasured; this is not full delivery throughput. | **Accept as-is** with this explicit reconciliation; no false numerical rate established. | Label every denominator and keep unmeasured human work visible. |
| 9 | Epic:62–64 records completion; :66 retains the earlier pending/no-pilot statement without an explicit superseded marker. Newer evidence and this thread establish the actual chronology. | **Defer**, nonblocking documentation reconciliation A4. | Mark superseded state summaries as historical while preserving dated decisions. |
| 10 | Story plan frontmatter says done; :75–76 records completed parent review/reruns; :97 retains pending handoff wording. Git commits `92ac157c2de936b71df297a60212fd7b3e274e67` / `356b1560ad9b5ed33f24ff6ef9ba715fb5073853` settle final implementation/acceptance state. | **Defer**, nonblocking documentation reconciliation A4. | Label delegate-stage notes rather than leaving competing final-state claims. |

No report demonstrates a missed declared runtime deliverable or invalid pilot result. Reports 1–3 are bounded test-sensitivity improvements; 7/9/10 are evidence-retention/readability improvements. Reports 4–6/8 are accepted with explicit limits rather than promoted into requirements the epic never declared. Findings are consolidated into four proposed owned actions below; no patch was applied.

## Behavior verification

No runtime behavior changed in this epic; the only source change is test code. Therefore no new application/UI end-to-end exercise is applicable, and none was run in this retrospective. Recorded Core seam and audit-control executions are evidence of test behavior, not a claim that the application was interactively exercised.

- 2.1 primary logs inspected: named `linked-resize-named.log:8` and parent named `linked-resize-parent-named.log:7` show 1 pass/0 ignored; parent module log:16 shows 10 passes/0 ignored. Control logs for reflow/partner/Undo/Redo each show 1 failure and the intended oracle diagnostic (respective log:13); each restored log:8 shows 1 pass/0 ignored. The initial full-check interruption is incomplete evidence, not a product failure; the completed retry is considered separately.
- 2.2 primary logs inspected during Build coordination and retrospective: `calibration.log:4` classifies `audit_sentinel.clear();` failed, final-restored log:36/76 shows 29 passes/0 ignored in each target, all 27 replay logs have exactly-one-test pass records, and full-check log ends with all six steps passed. Source/config and outcome/exclusion reconciliation above bound these observations.
- Retrospective primary-log reconciliation confirms the same named sets/outcomes and restored source. Existing check/control runs were inspected, not rerun. The expired/interrupted first 2.1 check is not counted as success; the no-timeout retry at `linked-resize-full-check-retry.log:5974` is.

## Previous-retro follow-through

The previous repair retrospective is explicitly protected from reads by the user's instruction. Its action items cannot be checked from that file; do not treat this as checked and clean.

## Action items

At the retrospective verdict, all four items were **proposed/deferred, not applied or automatically ticketed**. They are nonblocking improvements to a finished epic. The user subsequently requested scheduling A1–A3 and applying A4; the dated follow-through below records that decision without rewriting the reviewed evidence.

1. **A1 — Linked metadata oracle hardening. Owner: Layout test maintainer (Amelia through a separately approved ticket).** Evaluate capturing the accepted axis and comparing layout ownership/mirror-link metadata across commit/Undo/Redo. Preserve the existing numeric reflow/size/identity checks. Source: adversarial reports 1/2; current source:1516–1548,1615–1644. Intended prevention: independent expectations and explicit history scope, not whole-document assertions by default.
2. **A2 — Nonzero linked rotation coverage. Owner: Layout test maintainer (Amelia).** If nonzero rotation reflection needs regression coverage, add a linked-half nonzero fixture/counter-example without replacing the working zero-angle reflow case. Source: report 3, helpers:674–676 and test:1648. Independent moves/offsets or size-only controls are optional future extensions, not prerequisites created by this retrospective.
3. **A3 — Reconstructible pilot evidence. Owner: audit evidence maintainer.** Before deleting the disposable checkout, preserve the exact effective exclusions and a reconstruction specification for Cargo forwarding as inert evidence in the existing plan through an authorized change. Include stdout/stderr, status mapping and FD9 forwarding; commit no operational scratch/config/database. Source: report 7 and the inspected disposable files. Intended prevention: audit provenance surviving disposable cleanup.
4. **A4 — Historical state labels. Owner: Sofia (`bmad-agent-po`) with the user.** When accepting 2.2/closing the epic, explicitly supersede the old pending/no-pilot statement in epic Notes and label the story-2.1 delegate handoff as historical, without deleting evidence or changing the accepted functional requirements. Source: reports 9/10; epic:66, story plan:75–76,97. Intended prevention: machine/human readers choosing obsolete workflow state.

Process lessons already supported by evidence: retain no-timeout full-check execution (story plan:95), named pass/failure controls before removal results, explicit exclusion boundaries and setup/triage cost accounting (spike plan:74–84). These reinforce existing safeguards rather than add a new mandatory validation protocol.

### Follow-through — 2026-11-16

- User requested: schedule A1–A3 and have Sofia apply A4. These are follow-ups, not retroactive failures of the accepted epic criteria.
- A3 scheduled first as [backlog story 2](../../../backlog/story-preserve-integration-audit-pilot-reconstruction-evidence.md), evidence-retention priority before disposable cleanup. No new audit/cleanup is authorized.
- A1 scheduled as [backlog story 3](../../../backlog/story-preserve-linked-resize-axis-and-history-metadata.md).
- A2 scheduled as [backlog story 4](../../../backlog/story-cover-nonzero-linked-rotation-during-key-resize.md), waiting on A1 because they share the linked-resize test lane. A3 is independent; working priority is not a fabricated prerequisite.
- All three are queued, unassigned and not started; their standalone criteria await refinement/approval before Build. Owners remain the action-item owners above.
- A4 **applied**: epic Notes now explicitly supersede the earlier no-pilot/pending-authorization state; story 2.1's pending-review/commit sentence is labeled historical and points to final review, implementation `92ac157c2de936b71df297a60212fd7b3e274e67` and acceptance `356b1560ad9b5ed33f24ff6ef9ba715fb5073853`. Historical evidence and frozen intent are preserved. No ticket/epic status changed.

## Decision records

None proposed or written. Synthetic test-fixture choice, relevant-state snapshot and disposable mapping/exclusion choices are local/reversible test strategies, already explained in the plans; they do not meet the hard-to-reverse/surprising/real-trade-off test for an architectural decision record. No permanent production or target-mapping decision was made.

## Acceptance verdict

**accepted-with-open-items** — machine verdict against **declared** Requirements and Done when.

- R1/parent R7 is delivered by an active synthetic linked fixture, numeric source reflow and partner-state assertions, stable member identities and both-halves relevant-state Undo/Redo checks. This is end-to-end planner/Core coverage in the recorded zero-angle scenario, not every rotation or isolated source-only Core propagation.
- R2/parent R8 is delivered by validated mapping and named execution, an observed failing Necessist control, explicit ignored/helper exclusions, all passing-removal reproductions in the ordinary target and a measured, appropriately limited recommendation. Excluded closure-bearing tests and zero-candidate helper-only tests are disclosed, not accepted as audited.
- Full-check evidence passes all six steps; delivered production behavior, normal integration mapping and test layout are unchanged. No missing ticket forces rejection: 2.1 is done, 2.2 built.
- Four nonblocking proposed items remain tracked above. No production bug or criterion-breaking defect was established; optional hardening is not silently converted into a requirement. The previous retrospective remains unread by instruction, so follow-through is unverified, not certified.
- This verdict does not mark 2.2 done or close the epic. Human acceptance and Sofia's confirmed state transition remain separate. No wider audit or cleanup is authorized.

## Open questions

- Resolved 2026-11-16: user requested scheduling A1–A3; they are queued in the follow-through above, not started.
- Resolved 2026-11-16: user asked Sofia to apply A4; historical labels are corrected without accepting 2.2 or closing the epic.
- Full previous-retro follow-through cannot be resolved while its read prohibition stands; no change to that boundary was assumed.
- No additional shared design decision or ADR approval is required by the evidence. Team discussion was not requested and was skipped.
