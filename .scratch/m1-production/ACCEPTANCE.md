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

## Current release — `a49bb798`

The complete maintained artifact `m1-release-20261002-a49bb798` passes all 18
build commands, 925 source hashes and both 47-file asset inventories. There is
one current page WASM per prefix. [Build provenance](evidence/integration/release-a49bb798-provenance.json)
and [latest-source checks](evidence/integration/final-story-source-checks/record.json)
retain exact commands and outputs: 14 application tests, 12 web tests, formatting,
strict application/page lint, repository/contracts/runtime contracts and boundaries
all pass. Independent [Standards](evidence/review/standards-active-instance-final-e1e8606.md)
and [Spec](evidence/review/spec-active-instance-final-b50ddbdd.md) reviews have no findings.

[Final browser QA](evidence/integration/release-a49bb798-qa/README.md) passes
root/subpath online and cached-offline active restoration, empty/stale preferences,
same-origin storage separation, a real delayed stored-read versus newer explicit
open, slow Core startup, filtered physical navigation, canonical and selected-left
case generation, and cancellation during actual CAD initialization with no stale
canvas. Injected preference API errors are caught; they are not real storage-denial
proof. Closing the owned app tab removes its Core/CAD browser targets and reopening
restores the saved document. This does not observe Rust `use_drop` or GPU memory.
Sofle Right PCB reports its actual unready input; Left physical generation requires
adding case settings through the public control.

[Final physical STEP delivery](evidence/integration/release-a49bb798-qa/physical-left-step-acceptance.md)
passes the public Left PCB / left half path on an owned online root origin with
service workers disabled for the response gate. The 6,417,795-byte download has
SHA-256 `9b697043af32fc028cd7e52cfb93b9c778747318cf411f8555d9652646a4adec`;
its Blob URL is revoked. Holding the actual CAD worker WASM, changing to Canonical,
and releasing unchanged bytes produces no additional URL or download. This is
delivery/cancellation evidence, not an offline or material-oracle result.

Focused source-equivalent keyboard/compact/axe checks pass, with SVG contrast
incomplete. Actual screen-reader interaction remains blocked. All five fresh paired
public pointer sessions pass the unchanged 33/50/100 ms caps at 30/100/200 keys,
with 100 measured samples per scenario and no missing paint markers. Median
candidate p95 values are 18.8/24.4/33.1 ms. See the
[paired timing summary](evidence/performance/runs/paired-pointer-a49bb798-20261002-retry1/summary.json).
The marker records a following animation-frame opportunity, not physical display.
Ancillary startup visibility observations are incomplete in 15 of 30 reloads;
no startup timing gate is claimed. The unchanged reference UI/live frozen
comparisons fail; the five-session CAD run passes per-row limits but its frozen
comparison is ineligible because OS/core/renderer identities differ. No baseline
or budget changed. Complete renderer/STEP material attribution and direct GPU
accounting remain unperformed; unchanged geometry/resource evidence below is reused
only for its explicitly recorded scope. M1 acceptance remains open.

## Retained evidence — release `8f509433` and reviewed overlays

| Gate | Recorded outcome and scope | Record |
| --- | --- | --- |
| Provider reconciliation | Pass: approved transparent boxing, contracts and affected provider checks; ticket 01 closed | [Provider summary](evidence/providers/SUMMARY.md) |
| Session and integer identity | Pass: 13 public native session tests, 12 native web tests; lossless core transport and explicit CAD safe-range rejection | [Final source checks](evidence/integration/final-source-checks/record.json), [decoder checks](evidence/integration/final-decoder-checks/record.json) |
| Durable storage/archive exchange | Pass on recorded component artifacts: real IndexedDB abort/retry and full-fixture archive exchange; final release scoped databases and reload pass | [Final browser QA](evidence/browser-storage/release-8f509433-final-qa.md) |
| Offline/deployment | Pass on final root/subpath assets: control, offline reload, cold/missing asset failure, scoped update and unrelated cache preservation | [Final offline record](evidence/offline/release-8f509433-browser.json) |
| Case/renderer | Both final fixtures generate; focused DPR2, resize, remount, cancellation/retry and visible context-loss stop pass; scoped Worker/observer/listener/RAF/URL counts now recorded, late-import disposal now has a separate public race record; actual tab-close observation was unperformed on this earlier artifact | [Focused renderer](evidence/renderer/focused-2040e23b/README.md), [final QA](evidence/browser-storage/release-8f509433-final-qa.md), [resource counts](evidence/cad-jobs/final-resource-summary-8f509433.json) |
| STEP | Final downloads and URL cleanup pass for both fixtures, stale scope suppresses delivery; final REVIUNG mesh and independent BRep readback agree on volume/bounds; configured historical oracle is a different input, same-input default reference geometry now agrees at reader precision; exact STEP bytes differ by tiny decimal coefficient rounding | [Final exports](evidence/renderer/step-exports-8f509433.json), [geometry oracle](evidence/cad-jobs/step-geometry-oracle.json), [final readback](evidence/cad-jobs/readback-final-reviung41-summary.json) |
| Presentation | Raw axe, contrast, keyboard/pointer and compact views recorded; actual screen-reader interaction blocked on this host | [Assistive technology limitation](#available-assistive-technology--2026-10-01) |
| Performance | Fail on old8f 100/200-key public pointer caps; isolated range-ID fix improves but still fails. Second keyboard-list allocation repair passes one focused session18.6/24.1/33.3ms; the final a49 five-session result above passes. Historical endpoint/environment mismatches remain ineligible; budgets unchanged | [Performance evidence](evidence/performance/) |
| Exact integration | Pass: complete locked build, source/asset verification, native/fmt/strict checks, repository/contracts/boundaries and independent source reviews | [Release provenance](evidence/integration/release-8f509433-provenance.json), [Standards](evidence/review/standards-step-decoder-5a0972a2.md), [Spec](evidence/review/spec-5a0972a2.md) |

Runtime decoder source is `5a0972a2`; pointer repairs are `e7d29ce6` and the
startup/selector source is `b50ddbdd`. The latest complete maintained build is
`a49bb798`. The `8f509433` and `b9748745` records retain their exact artifact scope;
no earlier failure is replaced by a later result. Component evidence is reused
only where relevant source inputs are unchanged. Evidence/document commits do not
change production inputs.

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
