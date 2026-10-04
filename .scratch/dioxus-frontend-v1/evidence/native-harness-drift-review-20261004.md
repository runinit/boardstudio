# Native integration harness drift repair review — 2026-10-04

Verdict: **CLEAR. No concrete defects found in the frozen test-maintenance batch.** Native execution counts and mandatory gates remain the coordinator's verification responsibility; this review does not claim execution or product qualification.

## Exact reviewed scope

Worktree: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`.
Branch: `codex/rust-v1-ui-parity-20261001`.
Base HEAD: `4e716156a0692972b1309798cdc883739c7ff73b`.
Reviewed working-tree diff against HEAD for the following four files. `git diff --name-only HEAD -- web/tests/matrix_field_lifecycle*` lists only `web/tests/matrix_field_lifecycle/harness.rs`; no other matrix target file changed. Unrelated staged/dirty files and previous reports were excluded and preserved.

| File | SHA-256 |
| --- | --- |
| `web/tests/matrix_field_lifecycle/harness.rs` | `c3d3744d97ec3298c1ddf28718d8559a808dcbc7363dbc97c566d9be3ff49c09` |
| `web/tests/parts_standard_profile_editor.rs` | `00126ac58aedce97cb2b3cc7c5fda8adddd505a4bfa254ed46d889190a4b3fab` |
| `web/tests/keycaps_fit_lifecycle.rs` | `dfd710882a625db3d892f3db25cc91cedf12592be7420e116aac61c21fb3d8c6` |
| `web/src/presentation/keycaps_fit.rs` | `12d7f004c26299d96e89c92c6c2e6fadf64e44707ae3de59e0faa75b6e9dc68e` |

## Standards and specification

No documented-standard breach or actionable baseline smell found. Changes are confined to fixture/interface maintenance and test registration, retaining the native harnesses' compilation of actual production components.

- Matrix projection now initializes all required fields with neutral representative values, and the host supplies inert handlers for newly required actions. Mounted input and same-owner no-remount assertions change from five to seven because production renders Edge gap X/Y in addition to the original five inputs. All five original named-field checks and all four original lifecycle test bodies remain. The change accounts for Edge gap presence; it does not claim new lifecycle coverage of those fields.
- The standard-profile fixture adds the required mechanical-extraction port with a deterministic error response. Fixture definitions have no `kicad_source`; production mounts extraction only when that source exists, so the new seam is outside the exercised standard-profile flow. Existing request, acceptance, selection, and lifetime assertions are unchanged.
- The keycaps fixture supplies empty mechanical layer IDs and an inert navigation callback, consistent with its empty document/findings resolution. Its added `KeycapsPreviewInput` stub exactly matches the production runtime type's fields and derives. All four original absent-source, accepted-source, unmount and completion tests remain unchanged.
- The sole production-file diff changes the inline test module guard to `all(test, target_arch = "wasm32")`. All nine existing tests use `#[wasm_bindgen_test]`; their bodies and browser configuration are unchanged. This avoids compiling browser test fixtures into the native path while retaining their wasm registration. No release behavior changes.

## Evidence and limits

Reviewed the current diff, production component/type counterparts and `native-harness-drift-triage-20261004.md`, using the previously read AGENTS/CONSTRAINTS contract. No builds, tests, packages or commits were run by this reviewer. Only this report was written.

The coordinator reports that the author captured native compilation RED and combined-target exit 0; executed counts are still being collected. Neither exit 0 nor static preservation of nine wasm attributes establishes execution. The pending coordinator gate must confirm actual native counts and wasm registration/execution; the separate renderer lifecycle module remains outside this batch. No new application package is necessary for these test-only changes. Mobile validation remains deferred by user instruction.
