---
title: 'Isolate Chrome discovery fixtures and simplify environment cleanup'
type: bugfix
ticket: ''
created: '2026-11-14'
status: built
baseline_revision: 'ffd838d4149728e33f2f1c757431f2d2e6179e8e'
route: oneshot
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/CODING_STANDARDS.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The Parts inspector launcher fixture controls PATH but inherits the developer's pinned Chrome installation. After shared discovery was implemented, its PATH-fallback and missing-browser assertions fail, stopping quick and integrated acceptance checks in tooling.

**Approach:** Make the existing subprocess fixture hermetic across HOME, PATH, BOARDSTUDIO_CFT_DIR and CHROME_BIN, and simplify the directly related discovery tests with explicit mappings and scoped environment patches. Keep production Chrome discovery, candidate ordering, diagnostics, launcher behavior and toolbar code unchanged. Do not add a fixture framework, move test suites or change the supplied-environment contract of pinned_cft_dir.

### Success criteria

- Given an isolated launcher fixture, when Chrome is available only on its fake PATH or an explicit override is provided, then it selects that exact executable and preserves KEEP_ME and BOARDSTUDIO_PREVIEW_URL.
- Given that fixture with no Chrome executable and no pinned installation, when the launcher runs, then it exits nonzero with an actionable CHROME_BIN diagnostic and neither BUILD nor LAUNCH occurs.
- Given the shared resolver and WASM pinned-install scenarios, when the existing tests run and their environment patches exit, then override/pinned/PATH precedence, selected WebDriver configuration and environment restoration remain covered without manual save/restore in the changed scenario.
- Given the stable candidate, when the focused suites and required acceptance commands execute, then every result is reported honestly; any failed or unrun mandatory gate blocks acceptance. No ticket closure is claimed.

</frozen-after-approval>

## Implementation Notes

Selected oneshot because this is a low-risk, tests-only change estimated below 100 modified lines in three files. The user approved a standalone build and a separate setup commit; no initiative or ticket migration is involved.

Owned edits: scripts/test-check.py (the existing Parts inspector launcher fixture); scripts/test-browser-tools.py (direct resolver environment setup); scripts/test-run-wasm-tests.py (the existing pinned-CfT scenario's manual save/restore). Reuse TemporaryDirectory, self.dir, existing fake executables, patch.dict and all existing assertions and names. scripts/browser_tools.py remains the discovery owner; scripts/run-wasm-tests.py retains its diagnostic adapter. They are references, not edit targets.

Observed RED is recorded in .evidence/acceptance-chrome-86adce29/quick.log and integrated.log. Those runs stopped in tooling, so they do not establish later gate results. Capture a fresh focused RED before fixture edits. No new proof-gap behavior or mutation is proposed: preserve the already-failing assertions and run the affected suites.

Gortex plan creation was rejected twice as stale after the setup commit, with the required 30-second retry. This plan was created through the repository-permitted native patch fallback; no source file has yet used the fallback.

Fresh focused RED reproduced both failures at the expected assertions in parts-inspector-red.log. Implemented the three owned test-only edits without changing names or assertions. The source batch was also rejected as stale twice after the required retry; native patch fallback was used for scripts/test-check.py, scripts/test-browser-tools.py and scripts/test-run-wasm-tests.py. Production files are unchanged.

Focused GREEN: test-check.py 81/81, test-browser-tools.py 13/13, test-run-wasm-tests.py 63/63; the single Parts inspector regression also passes. Logs are in .evidence/chrome-fixture-isolation/*-green.log. git diff --check passes. Gortex detect lists only the three owned test files; tests reports no graph links for these test methods, guards has no configured rules, and contract gives generic test-coverage/co-change warnings. These are graph limitations, not failed runtime checks; no helper extraction or scheduler production change is warranted.

AST verification against the full baseline revision confirms all test names and assertion expressions are retained in each changed file, and all three files parse. See oracle-retention.log. The five acceptance commands are running serially under the shared lock; review and the fix commit wait for their results.

The first acceptance run passed lint, Layout 281+2 and shell 6/0, but quick/integrated exposed five receipt CLI fixture failures (quick.log and integrated.log). CheckCliTests mocked shared locate_chrome as the non-executable bare name chrome; the WASM preflight now validates that shared result. An in-memory fixture experiment changing only the mock result to /bin/true made all five failed cases pass (receipt-fixture-diagnostic.log). This is another fixture compatibility issue from the same shared discovery change, not a receipt-production defect. Added scripts/test-verification-receipt.py to the owned test files: use the executable stand-in and scope CHROMEDRIVER=/bin/true to avoid depending on the host driver. No assertion or production behavior was changed. This small test-only addition remains under the same user-facing goal and the oneshot size threshold. Gortex admitted this fourth-file edit after its view recovered.

The receipt suite is now GREEN, 72/72 (receipt-suite-green.log), bringing affected suites to 229 passing tests. The stable candidate touches four test files, 13 insertions and 21 deletions, with no production edits. Re-run acceptance into .evidence/chrome-fixture-isolation/acceptance-v2/ so the first failed receipts remain intact. Review and commit still wait for acceptance results. The fourth file's post-edit graph guards have no rules; generic co-change advice to edit production receipt code was deliberately not followed.

Final acceptance-v2 results are GREEN for all five commands: quick 72/72 planned commands, lint, Layout 281+2, shell 6/0, integrated 73/73 planned commands. Integrated receipt records 2429 native passes, 12 ignored, 0 missing retention names; every WASM crate launch complete, 262/262 page tests complete, and all three provider/product smoke checks exit 0. Receipts identify the tested dirty working tree at baseline ffd838d4149728e33f2f1c757431f2d2e6179e8e; source/build binding remains unknown as reported, not a claimed packaged visual or ticket-closure proof. Four-file AST retention also passes in acceptance-v2/oracle-retention.log. No test source changed during this successful run. Proceeding to the required quick review with the same Sol high level as this session.

Completion: all intent criteria are satisfied by the original RED, the four GREEN suites (229 tests), preserved assertion/name inventory, successful acceptance-v2 receipts, and a quick review with no findings. No production behavior or toolbar files changed, no native test baselines were updated, and no broader ticket closure is claimed. The fix and this plan are committed separately from setup; the tested source diff is unchanged by final record metadata.

## Plan Change Log

## Review Triage Log

- Quick inspection review by a fresh openai/gpt-6.1-sol session via OpenCode, variant high, completed on the fixed baseline-to-working-tree snapshot: No findings. All four test-file hunks and the plan were supplied; no code edits or test runs were delegated to the reviewer. No findings to classify or defer; no review loopback. Parent confirmation: no source changes after the reviewed snapshot, only this final record/status metadata.

## Verification

Run all test/build commands under the repository shared build lock; do not overlap target owners. Keep logs and receipts under .evidence/chrome-fixture-isolation/. Isolated Python fixture suites hold the outer lock without advertising its descriptor to fake child environments; check.py and the real WASM runner receive BOARDSTUDIO_BUILD_LOCK_FD=9 as documented.

Commands, in order:
- python3 scripts/test-check.py CheckTests.test_parts_inspector_discovers_chrome_before_build_and_preserves_environment — expected RED before the fixture repair, GREEN after.
- python3 scripts/test-check.py — all scheduler/launcher tests pass.
- python3 scripts/test-browser-tools.py — all shared-discovery and build-lock tests pass.
- python3 scripts/test-run-wasm-tests.py — all WASM runner tests pass.
- python3 scripts/test-verification-receipt.py — all receipt tests pass; the newly exposed CLI fixture failures must be GREEN without changing their assertions.
- git diff --check — no whitespace errors; inspect each changed file's full diff and preserve assertions.
- python3 scripts/check.py quick — required gate, fresh receipt.
- python3 scripts/check.py lint — required native/WASM lint gate, fresh receipt.
- python3 scripts/check.py browser --only layout — required Layout mounted subset, fresh receipt.
- python3 scripts/run-wasm-tests.py --files web/src/presentation/workbench_shell.rs — required shell page subset, fresh result JSON.
- python3 scripts/check.py — integrated gate, fresh receipt; no skipped assertions or baseline updates to force GREEN.

After two unchanged-result attempts, stop with the blocker and a materially different next experiment. Newly exposed failures do not authorize unrelated production or toolbar repairs. Keep the fixture repair separate from the already-committed BMad setup.
