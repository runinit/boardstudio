# M1 acceptance ledger design

The implementation and completed checks below are attributable to the recorded source and artifacts. M1 acceptance remains **open**: an absent or ineligible check never becomes a pass. P1/P2/P3 results remain historical feasibility evidence. Parent constraints and six specifications define the acceptance bar.

| Gate | Oracle / required observation | Evidence owner |
| --- | --- | --- |
| Provider reconciliation | Exact newer dev commits, overlap resolution, full affected public provider suites, unchanged generated wire meanings | 01 |
| Durable session | Real engine edit/open/history before save; old accepted snapshot until complete; abort/retry same retained commit and one history entry; blocked dependents; stale IDs/variants/executors | 02 |
| Pointer/numeric interaction | Both fixtures; final sample; threshold/modifier/snap/Alt; unrelated pointer; Escape/lost capture/cancel; one-step Undo/Redo; no preview storage/history | 02 |
| Storage | Real IndexedDB completion versus request success, abort and retry, schema keys/value representation, exact asset hashes, active preference, reload | 02,03 |
| Archive exchange | Rust archive → React import/save/load/archive → Rust reopen, faithful supported extension/component/module data and all assets | 03 |
| Offline/deployment | Root and /boardstudio/ release asset identity; scoped cache install/update, offline cached navigation, cold/missing lazy assets, unrelated cache preservation | 03 |
| Case-setting generation | Saved change, prepared inputs/readiness and exact cached/uncached provider output on REVIUNG41 and Sofle; true off-page worker execution | 04 |
| Async resource safety | Stale equal-revision reopen, board/instance switch, worker failure, caller settlement/cache consistency, cancellation and root close | 02,04,05 |
| Renderer | Current exact body/material input; same-canvas DPR/resize, context loss, panel remount, late import cancellation, tracked listeners/observers/RAF/URLs/GPU disposal | 04 |
| STEP | Independent export worker/cache on captured accepted input; no provisional authority; stale/failure/cancel prevents delivery; independent reopened geometry/material/bounds/volume oracles | 05 |
| Integer identity | Preserve lossless core transport; test CAD safe-range edge and wide inputs. Reject incompatible identities; no unsupported complete parity claim | 05 |
| Presentation | Paired reference desktop/compact/short views, theme/overflow and long/error/recovery content; names, real keyboard/focus order/restoration, relevant screen-reader, raw axe and contrast | 06 |
| Performance | Existing affected CAD/live/UI budgets and controls retained; startup/size/crossing measurements with reference variance, no invented threshold | 06 |
| Exact integration | Locked builds, native/WASM fmt/strict lint/tests, contracts, repository/boundary, React tests/build/affected browser matrix; source/asset hashes and Standards/Spec reviews | 06 |

## Retained evidence — release `8f509433` and reviewed overlays

| Gate | Current outcome and scope | Record |
| --- | --- | --- |
| Provider reconciliation | Pass: approved transparent boxing, contracts and affected provider checks; ticket 01 closed | [Provider summary](evidence/providers/SUMMARY.md) |
| Session and integer identity | Pass: 13 public native session tests, 12 native web tests; lossless core transport and explicit CAD safe-range rejection | [Final source checks](evidence/integration/final-source-checks/record.json), [decoder checks](evidence/integration/final-decoder-checks/record.json) |
| Durable storage/archive exchange | Pass on recorded component artifacts: real IndexedDB abort/retry and full-fixture archive exchange; final release scoped databases and reload pass | [Final browser QA](evidence/browser-storage/release-8f509433-final-qa.md) |
| Offline/deployment | Pass on final root/subpath assets: control, offline reload, cold/missing asset failure, scoped update and unrelated cache preservation | [Final offline record](evidence/offline/release-8f509433-browser.json) |
| Case/renderer | Both final fixtures generate; focused DPR2, resize, remount, cancellation/retry and visible context-loss stop pass; scoped Worker/observer/listener/RAF/URL counts now recorded, late-import disposal now has a separate public race record; actual tab-close observation remains unperformed | [Focused renderer](evidence/renderer/focused-2040e23b/README.md), [final QA](evidence/browser-storage/release-8f509433-final-qa.md), [resource counts](evidence/cad-jobs/final-resource-summary-8f509433.json) |
| STEP | Final downloads and URL cleanup pass for both fixtures, stale scope suppresses delivery; final REVIUNG mesh and independent BRep readback agree on volume/bounds; configured historical oracle is a different input, same-input default reference geometry now agrees at reader precision; exact STEP bytes differ by tiny decimal coefficient rounding | [Final exports](evidence/renderer/step-exports-8f509433.json), [geometry oracle](evidence/cad-jobs/step-geometry-oracle.json), [final readback](evidence/cad-jobs/readback-final-reviung41-summary.json) |
| Presentation | Raw axe, contrast, keyboard/pointer and compact views recorded; actual screen-reader interaction blocked on this host | [Assistive technology limitation](#available-assistive-technology--2026-10-01) |
| Performance | Fail on old8f 100/200-key public pointer caps; isolated range-ID fix improves but still fails. Second keyboard-list allocation repair passes one focused session18.6/24.1/33.3ms; complete rebuilt-release five sessions remain pending. Historical endpoint/environment mismatches remain ineligible; budgets unchanged | [Performance evidence](evidence/performance/) |
| Exact integration | Pass: complete locked build, source/asset verification, native/fmt/strict checks, repository/contracts/boundaries and independent source reviews | [Release provenance](evidence/integration/release-8f509433-provenance.json), [Standards](evidence/review/standards-step-decoder-5a0972a2.md), [Spec](evidence/review/spec-5a0972a2.md) |

Runtime decoder source is `5a0972a2`; the latest completed maintained build is
`b9748745`, including both pointer allocation repairs. Its bounded
[root/subpath smoke](evidence/integration/final-browser-smoke-b9748745.md) passes.
The wider `8f509433` observations above retain their exact source scope. Saved
active-project restoration and physical-instance selection were gaps in that
release. Reviewed source `a49bb798` now implements both; its
[focused root online QA](evidence/integration/startup-restore-instance-focused-b50ddbdd-qa/record.json)
passes restoration, stale preference, filtered options, keyboard selection and
compact layout/axe checks. Fourteen session and twelve web tests pass on the
updated candidate. The complete rebuild, offline/subpath and startup/case race
checks remain pending; neither flow was established by the earlier bounded smoke. Failed paired
performance attempts remain [recorded](evidence/performance/README.md); a valid
five-session aggregate is still unperformed. The unchanged
[reference gates](evidence/performance/runs/reference-gates-20261002T002536Z/assessment.md)
completed: UI/live frozen worker/main-thread and interaction timing comparisons
fail. CAD ran all five sessions with all per-row completion/paint/RSS budgets
passing, but its frozen comparator rejects OS and core/renderer WASM identity
mismatches. These are retained outcomes, with no changed baselines or budgets.
Evidence/document commits do not
change production inputs.
Component evidence is reused only where relevant source inputs are unchanged;
final release observations are distinguished explicitly.

## Evidence record shape

Each executed command records argv, work directory, source commit/dirty content hashes, toolchain/features, generated artifact hashes, start/end time, exit status and retained stdout/stderr. Each browser scenario records route, viewport/DPR, fixture/source hashes, actual actions/assertions, runtime/request failures, screenshots where useful and limits. A failed run is retained; a repair records diagnosis and focused red-green evidence. Do not replace old failures with a new success summary.

## Closure

Coordinator updates each local ticket, root TODO and current production run only after integration verification and required review. Gate outcomes are pass, fail, blocked or unperformed; absent checks are never pass. Relevant screen-reader or budget/API approval blockers remain explicit while independent work continues. Ticket 06 cannot close while any required gate is failed or unperformed. M1 acceptance authorizes no database cutover, push, deploy, main merge or React removal.

## Available assistive technology — 2026-10-01

The host inventory found no Orca command, installed package or Flatpak.
Speech Dispatcher and eSpeak are present, but provide speech synthesis rather
than screen-reader navigation. Relevant screen-reader interaction remains
**blocked**; AX tree inspection and raw axe/contrast checks cannot close it.
The remaining browser and performance gates continue independently. A later
manual check should cover opening a copied project, reaching/selecting a part,
preview/cancel/commit announcements, save-failure/recovery status, case generation
and export using an actual screen reader on the tested release.
