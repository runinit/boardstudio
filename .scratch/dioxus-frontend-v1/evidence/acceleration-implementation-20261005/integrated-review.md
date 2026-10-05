# Integrated acceleration batch review

Verdict: **CLEAR for source integration after the reported fixes.** No unresolved standards or correctness finding remains in the 24 assigned files. This is the one combined diff review and confirmation pass, not parent acceptance or package qualification.

Reviewed current working-tree/index changes plus assigned untracked files against HEAD `e12b6ab0f17ba879a6d3e7a2b7bdb8365cffc5d1` in `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, branch `codex/rust-v1-ui-parity-20261001`. Final source freeze was explicitly confirmed by the coordinator. Review completed `2026-10-05T02:00:16+00:00`. Reviewer performed no tests, builds, staging, commits, branch changes, or publication; this report is the only reviewer write.

## Reported findings and confirmed corrections

| Finding / original reproduction and risk | Final correction and regression |
| --- | --- |
| Headless gate identity covered its Python runner but omitted the selected Rust compiler; changing the actual selected compiler could leave a WASM receipt eligible. | `scripts/migration_gate_receipts.py:385` adds cargo for the delegated WASM gate; selected sysroot compiler bytes participate. Regression `test_wasm_gate_identity_tracks_rustup_selected_compiler_bytes` passes in the saved 13-test suite. |
| Incomplete counts could be stored/loaded despite the receipt contract; zero passed tests could also be stored. | `scripts/migration_gate_receipts.py:415` and `:444` reject incomplete/failed/excluded and present-but-zero passed counts in lookup/storage; execution rejects incomplete counts. Compiler-only checks retain their count-free receipts. Saved regression suite passes. |
| Stage failing maintained Rust bytes, restore passing worktree bytes, then default-index commit: before/after fingerprints alone could stay stable while committing untested staged bytes. | `scripts/migration-deliver.py:317` rejects initial staged/worktree divergence for maintained inputs actually committed from the index. Explicit-path commits use the tested worktree; unrelated staged evidence remains preserved. Regression `test_commit_rejects_staged_build_bytes_different_from_tested_worktree` is in the passing 30-test suite. |
| Duplicate terminal test lines were collapsed into a last-wins dictionary; FAILED then ok could look successful. Duplicate authoritative list entries were hidden by set operations. | `scripts/run-wasm-tests.py:218` rejects duplicate listed names; `:232` retains raw terminal observations. Duplicate outcomes become problems and `complete=false`. Author reports the three regression cases failed before the fix and runner36/36 passes afterward; raw Python runner-suite log was not independently supplied to this reviewer. |
| `frontier --json` was documented but rejected by argparse. | The frontier parser and handler now support the documented JSON payload; `test_frontier_cli_json_emits_derived_frontier_payload` is in the passing 35-test suite. |
| A fully verified parent with complete criteria/joins but pending formal acceptance disappeared from the waiting metric. | `progress.py:291` includes every nonaccepted parent with all functional criteria verified; `:369` renders the no-gap case as formal acceptance/review pending. `test_complete_unaccepted_parent_remains_visible_for_formal_acceptance` passes. |

The coordinator's additional absent-config completeness correction is confirmed: `qualification.py:151` pins optional absent paths, requires unchanged declared membership, and invalidates reuse on additions. Fixture byte identity, complete source footprints, directory additions, malformed input rejection and intermediate symlink rejection remain enforced. Historical unknown fixtures do not become newly reusable by inference.

## Standards and correctness confirmation

The final delivery contract preserves meaningful native state checks, mounted DOM checks and public outcome qualification at their respective layers. It bounds implementation to two authors and one reviewer, retains exact provenance and required integration checks, preserves explicit-path commits, and leaves mobile/compact validation deferred. The corrected entrypoint and `frontier --json` documentation match source.

Documentation-only follow-up confirmed 2026-10-05T02:01:26+00:00: final `CONSTRAINTS.md` removes the arbitrary two-publications-per-day quota and scheduled retrospective cycles. Required changed journeys may publish when ready; unchanged candidates are reused. One-package serialization, meaningful tests, exact attribution and formal acceptance joins remain required. The existing publication implementation still uses its extra-candidate-reason mechanism for a third same-day publication; the coordinator has been notified. No production source review was reopened, and only this policy confirmation and its file hash were updated.

Snapshot builds pin the committed build/snapshot helper bytes, retain the already-loaded snapshot helper, isolate the committed checkout and locked dependencies, preserve owned warm outputs, reject modified/unowned maintained checkouts, and serialize packages through one repository lock. Publication checks the complete committed input manifest/tree independently of newer coordinator work. Ordinary live-source drift guards remain. Provider literal includes carry their owning module context; inactive cfg(test) inline modules no longer falsely reject release graphs, while ambiguous active registrations still fail closed. A changed helper still requires a fresh full donor; no compatibility/hash bypass was introduced.

The explicit controller/Inspector owner map requires exact listed tests and includes the real mounted controller-to-UI replacement fixture. Its test captures submitted `SetMatrix` intent. It establishes neither persisted document acceptance nor comprehensive coverage of every controller/Inspector branch. The Case control reuses the existing Runtime route and checks the rendered current owner before dispatch; native admission and asynchronous delivery guards remain in that route.

## Outcome evidence actually inspected

- Saved terminal logs and matching SHA-256 receipt hashes: builder reuse39/39, maintained source inventory3/3, real-Git snapshots12/12, candidate publication14/14, gate receipts13/13, delivery30/30, progress35/35, qualification24/24. Receipts: `snapshot-builder-validation.json` and `gate-frontier-validation.json` beside this report. Expected simulated failures inside these suites are test fixtures, not failed suite outcomes.
- `case-local-export-red.log`: mounted product RED reaches `Case mounts Export geometry`, 0 passed/1 failed; `case-local-export-green.log`: 1 passed/0 failed, focused command exits successfully. Existing `../case-export-native-guards-20261005.log`: 3 passed/0 failed, proving application export ownership, cancellation and reopen guards.
- `matrix-transform-inspector-wasm-result.json` plus `matrix-transform-inspector-wasm-run.log`: exactly one expected/passed mapped mounted test, `complete=true`, no problems or known exclusions, 29,947ms. The old broad selection record has seven filters/21 outcomes but lacks comparable timing and exact test names; it cannot support a measured runtime-speedup claim.

The coordinator's required combined native/page/reachability/affected-headless integration gate, actual snapshot package/publication under concurrent live work, eligible real provider-reuse measurement and public Case exported-file replay are still operational finish conditions. Source review and focused event interception do not close those conditions or any parent/final join.

## Frozen reviewed file hashes

Each hash covers the final bytes reviewed in this batch. Any later change to a listed path invalidates this source handshake for that path.

| Path | SHA-256 |
| --- | --- |
| `AGENTS.md` | `d2bfb10817b0675ecae9e1281902b53fa98e8b6b7912163b177c51998b4b7eca` |
| `CONSTRAINTS.md` | `debec4a4e886a1210586f346bcfc7100c592d5574a0491ceab03803f6911111b` |
| `docs/agents/issue-tracker.md` | `876c4eda26fc5160f437dfe0841d4267d834387d18d7a031db4a64b845d27724` |
| `scripts/build-m1.py` | `2597f8a90a464e96b8c5442fb2f85454408e231b80152e47b2c1d344a5e1c5c7` |
| `scripts/migration_snapshot.py` | `b64474f737da3113af166e33448d9ff3fbadb1e1bbd92956960f9d1f94c25937` |
| `scripts/migration_candidate.py` | `8606826e97352263796b009bd7f8034863a89f6433138a675e11389e2f8cb3a9` |
| `scripts/migration-deliver.py` | `0fde5dabb0834be842ffbf265ca84b1b0b43b91eb4ab6f23d1daff97b76fa690` |
| `scripts/migration_gate_receipts.py` | `1505a415f791c3df087a3281d640e8a9b70a0207a5a6beda32390a024e3a553e` |
| `scripts/run-wasm-tests.py` | `c144f389d5137dfa5ad6a982851928237d9949d659a7048f6df324b2a4eb1511` |
| `scripts/wasm-test-owners.json` | `4b46754bb57a52f780643eee0326137e8ab961f6f2ec2bdf03bb9250561565af` |
| `scripts/test-build-m1-reuse.py` | `2e6ba5259cb42bc259b56dfc89e8500337b5b538938e457fa663c85db5c9f9ad` |
| `scripts/test-migration-snapshot.py` | `8c5c45bd5bcc3b8f98fd86ed8ed3f38f8a74d2cbfbd881173c7cdd53665523f1` |
| `scripts/test-migration-candidate.py` | `047795099dac305edccbcaa981d4ab10d00a512faa368d83befa66e805ec8828` |
| `scripts/test-migration-deliver.py` | `e93b23b16e98387442db4f0cb39d2052d052b8357497b29868338e544bbdead7` |
| `scripts/test-migration-gate-receipts.py` | `f715904206c36e41bb3f6878790ca201cd9ded0d8821c7091c6569efa78ae12d` |
| `scripts/test-run-wasm-tests.py` | `d86217027b2d8c8a9d51554eccf7d471be4f4681a2ef137a1781c5e3dcfb2b02` |
| `.scratch/dioxus-frontend-v1/progress.py` | `08f82e81fea777dbd88fc2c50dc0f6c923fecd90c5247f9995c45b207d9a129f` |
| `.scratch/dioxus-frontend-v1/qualification.py` | `856846ebd6eb89de643c3c229f8355afcfae0c41b0e717043ba518eb128da3c6` |
| `.scratch/dioxus-frontend-v1/test-progress.py` | `7296c857a5f78c13a85303c1af20023b8cf59caa2e08042540d469a499f649d8` |
| `.scratch/dioxus-frontend-v1/test-qualification.py` | `e0f289838159c8a22913c955321e079c927e9c3a8a4ef1dd0f6c227c87e51f56` |
| `web/src/cad_presentation.rs` | `d7009cad80c088584ced010b3735220d0b2092801a5bcf473e5411216f2e9495` |
| `web/src/presentation.rs` | `f81f0ee22c0e6d0c851cf0b91c3d1953fb9ca41c31e56bf3ec0293fa0efbff0a` |
| `web/src/presentation/objects/matrix_transform_inspector.rs` | `01f7d32c65b2ba8405b07172d4960871730caa2551c140a8be8d0d03ec0e525d` |
| `web/src/presentation/objects/matrix_transform_inspector_tests.rs` | `215653ec59c19b1fa9670e143eec49d57d906d5cd0842f1989eab85a16f2f9b0` |

## Required operational follow-up: documentation fingerprint

Confirmed 2026-10-05T02:13:51+00:00: prepared `docs-fingerprint.patch` is **CLEAR to apply**. The stopped broad headless gate was incomplete and establishes no passing integration result. This narrow correction responds to the reproduced false rejection when authorized docs/record edits occur after executable gates. `source_fingerprint` now uses the existing maintained executable inventory rather than the broader repository inventory: maintained source bytes, staged identities and root Cargo configuration still participate; unrelated docs/records do not. Initial staged/tested-source divergence protection is unchanged. Raw `fingerprint-red-1.log` and `fingerprint-red-2.log` reach the original fingerprint mismatch and actual commit GuardError; `fingerprint-green-1.log` and `fingerprint-green-2.log` show 14/31 passing in the temporary copy. Patch SHA-256 `d3eab945ab0fea8b09e8a77f941a9905484026218f4c99ba1b1d2d639b60734d`. Reviewer did not apply the patch or run tests. Coordinator application is now confirmed: all three live files are byte-identical to the GREEN temporary copies; their rows in the frozen-file table above have been refreshed using `fingerprint-applied-hashes.json`. The second bounded owner-map correction is confirmed below.

## Required operational follow-up: exact presentation helper owner

Confirmed 2026-10-05T02:18:13+00:00: `exact-root-owner.patch` SHA-256 `170d3cd8f97dc5b19a5966fc372e916ac28aa49036fef7b55274128ca5116e9e` is **CLEAR** and applied. The Case mounted test owns the root helper only while both actual HEAD base bytes (`9c47231afa66375defd2493f73acf500a2e277053135027a1e506437d5314ec4`) and current helper bytes (`f81f0ee22c0e6d0c851cf0b91c3d1953fb9ca41c31e56bf3ec0293fa0efbff0a`) match the reviewed test-only change. Missing or changed base/current bytes restore direct broad module selection. Active exact-owner parents no longer prune separately requested child tests, so changed Inspector/controller coverage remains required. Exact mapped names still must appear in the authoritative list and complete the strict terminal inventory. The regression covers the observed child-pruning failure and both hash-mismatch fallbacks; the author reports isolated runner37/37 GREEN, with no new WASM execution. All three applied owner-map files are byte-identical to the reviewed temporary copies. Final 24 live source hashes match `integration-path-hashes.json` and replace the rows above. The earlier interrupted gate remains incomplete; the coordinator owns the corrected integration run and all pending package/public outcome gates. No broad audit or reviewer test rerun was performed.
