# Consolidated functional controls review — 2026-10-03

Verdict: the reviewed application source and supplied focused checks/package gates pass. No remaining actionable source finding was identified after the repairs. Browser qualification is bounded: F2.4-C05, F3.3-C02 and F5.6-C03 are verified for their stated control/regression requirements; F5.5-C02 remains implemented and partially qualified. This review accepts no parent and closes no final integration join.

## Scope and source identity

Reviewed the original `f410ce99..ad26b07a` candidate and its corrective source through final commit `7ddd10e32d24b474366d995171dde89e1577d7fe`. The original seven application files cover preference fallback/root presentation, canvas movement, Case setup, mounted module Inspector and constituent placement. Repairs additionally use the private bundled catalogue loader and exact scope-transition provenance in `parts.rs`, `parts/modules_catalogue.rs` and `selection.rs`. Criterion/spec/RF changes were considered against their stated finish conditions. No public Core API, wire contract or project-document schema change is introduced.

This was a single consolidated review, with targeted source/evidence reads. The reviewer ran no application build, tests or browser session and changed only this review artifact. Current parent acceptance remains 10 of 62, with 52 open; criterion verification does not change those parent states or replace accepted history.

## Correctness and resolved findings

- Preference failures now leave theme and panel state usable in memory and surface one shared notice. Preference storage failure does not conflate local preference persistence with project storage.
- A Layout drag starts on real nonzero client movement, including the reference 1 CSS px gesture. A stationary client pointer during canvas reflow does not produce movement/history. The modified PCB selection guard preserves Shift/Ctrl selection without starting geometry changes; existing capture, arbitration and cancellation machinery remains in place.
- Case setup changes only the selected instance’s board/mechanical board association or flip in an accepted document proposal. The original successful reassignment lost its selected instance. The repair admits only the exact owned prior/next scope, accepted tokens and generation transition, with accepted proposal matching. It suppresses implicit instance fallback while that owned operation is pending, and the effect depends on the current busy value so fallback resumes after settlement. The production transition predicate is also the tested predicate; no loose generation increment exception remains.
- Module actions retain the exact nested constituent definition. Collision materialization reuses equivalent definitions or assigns a distinct private source ID rather than substituting a different global definition. Embedded circuit joins continue through existing Core copy/remove semantics.
- The original connector path rejected repeated Save of an already materialized automatic connector and searched only the selected module’s constituents for its horizontal connector. The repair uses the existing global bundled catalogue privately, normalizes its source definition, and protects async settlement with mount/alive, selection/scope, accepted-token/revision and draft checks.
- Browser/archive diagnosis then exposed reuse of a same-ID connector on another board. Final source admission now requires both selected-board membership and a host-role definition for reuse; automatic creation requires global ID absence. The selector/toggle obey the same constraint, and Save rejects a foreign, missing or non-host connector before submission. Final public qualification on 34780 passed: the occupied foreign-ID warning appears, Save rejects with actionable choose/clear feedback, Board 2 remains empty and Main retains all 33 parts including J_VIK1. Clearing `Assign host connection` then saves the placement while preserving those counts and the foreign connector.

## Supplied verification and packages

The retained Layout regression failed for the expected 1 px threshold before the change, then two WASM Chrome tests passed, including pointer arbitration (`retained-tmp/20261003/functional-controls-{red,green}.log`). The corrective Case production scope-transition predicate compiled and passed its focused WASM test. The final connector ownership regression ran as an actual `wasm_bindgen_test` and passed. These are author/root-executed checks, reused by this review; no broader suite is claimed.

Three packaged sources distinguish the browser evidence:

| Candidate | Source | Evidence use |
| --- | --- | --- |
| `frontend-functional-controls-20261003`, port 34778 | `ad26b07a` | Unchanged preference/Layout qualification, original Case red, bounded module actions |
| `frontend-functional-controls-repair-20261003`, port 34779 | `177f990bad633b75f82a817d9f14fdfe0ec1f3b9` | Repaired Case and connector-flow diagnosis |
| `frontend-module-ownership-20261003`, port 34780 | `7ddd10e32d24b474366d995171dde89e1577d7fe` | Final connector board-admission guard |

The [Case/module repair package proof](../frontend-functional-controls-repair-20261003/package-proof.json) reports 142.539329 seconds, 8 fresh and 22 inherited commands. The [final ownership package proof](../frontend-module-ownership-20261003/package-proof.json) reports 141.121047 seconds, 8 fresh and 22 inherited commands. Both prove their exact source commits, zero source/asset mismatches, root and `/boardstudio/` HTTP 200 with isolation headers, and zero release warnings. Package proof establishes asset/source identity; functional evidence remains separate. Later criterion/evidence edits require no app rebuild. Preference, Layout and Case receipts are reused because the subsequent connector-only guard does not change those application paths.

## Criterion adjudication and remaining evidence

| Criterion | Verdict | Evidence and limits |
| --- | --- | --- |
| F2.4-C05 | Verified | [Preference receipt](preferences/receipt.md): real Chromium quota exhaustion and a separate `--disable-local-storage` session with `window.localStorage === null`; theme and Objects auto-hide remain usable with exactly one session-only notice. A project name survives reload through its separate project store. No Storage API mocking. |
| F3.3-C02 | Verified for the bounded regression | [Paired Layout receipt](layout/RESULTS.md): trusted Alt+1 CSS px U1 movement, one Undo/Redo, stationary pointer across Inspector/canvas reflow, and Shift/Ctrl selection with movement but no geometry/history. The stationary test uses keyboard-driven Inspector expansion; broader pointer lifecycle criteria remain open. |
| F5.6-C03 | Verified for the Case controls | [Case receipt](case/receipt.md): paired ordinary Left→shared Right and reverse association/flip behavior; repaired own edits retain selected Left and controls, unrelated Right remains unchanged, fresh isolated flip and association history work, and persisted association/flip survive reload. Association Undo selects Right; Redo can clear selection until Left is reselected. TS history-focus parity is unqualified. The earlier multi-edit “History is empty” sequence is uncertain and superseded by successful fresh isolated histories, not diagnosed as a defect. |
| F5.5-C02 | Implemented; partially qualified | [Module receipt and archives](modules/vik-splitter-gnd-join-ad26b07a.md): paired circuit GND Copy/Remove and nested J1004 Place outcomes, candidate placement Undo/Redo and archive inspection; final 34780 foreign-connector rejection and clear-connection Save preserve both boards. The imported fixture already contained the automatic connector on Main, so the 34779 Board 2 Save/repeat Save journey is foreign-connector reuse, not fresh creation. Genuine Dioxus attachment/source creation, archive reimport/reload and stale-scope behavior remain unqualified. |

Fresh Dioxus attachment could not be exercised through the packaged public Parts route: it exposes preview/profile controls but lacks the TS `Attach module` and host-assignment action. This remains a bounded F5 consumption-seam investigation; F4.7’s accepted source-authoring boundary is not widened. F5.5-C03 remains unassessed for its full integrated definition/readiness/profile/asset finish condition.

Case F5.6-C04’s broader retained controller/mechanical state journey remains implemented, and F5.6-C05/F7 shared-viewer evidence remains open. The Case fixture displayed terminal/net validation error `left/U1/P2` / `LEFT_SW25_ENCODER-PUSH` after the reverse association. Its baseline-versus-edit causality is unresolved; physical setup saved, but this receipt claims no preview success. Existing provider-failure and other unexecuted preview branches retain their separate limits. No unrelated parent acceptance or release readiness is inferred.
