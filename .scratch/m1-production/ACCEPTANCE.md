# M1 acceptance ledger design

All entries below are **unperformed for production M1** until a current ticket records exact commands, outputs, source and built asset identity. P1/P2/P3 results remain historical feasibility evidence. Parent constraints and six specifications define the acceptance bar.

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

## Evidence record shape

Each executed command records argv, work directory, source commit/dirty content hashes, toolchain/features, generated artifact hashes, start/end time, exit status and retained stdout/stderr. Each browser scenario records route, viewport/DPR, fixture/source hashes, actual actions/assertions, runtime/request failures, screenshots where useful and limits. A failed run is retained; a repair records diagnosis and focused red-green evidence. Do not replace old failures with a new success summary.

## Closure

Coordinator updates each local ticket, root TODO and current production run only after integration verification and required review. Gate outcomes are pass, fail, blocked or unperformed; absent checks are never pass. Relevant screen-reader or budget/API approval blockers remain explicit while independent work continues. Ticket 06 cannot close while any required gate is failed or unperformed. M1 acceptance authorizes no database cutover, push, deploy, main merge or React removal.
