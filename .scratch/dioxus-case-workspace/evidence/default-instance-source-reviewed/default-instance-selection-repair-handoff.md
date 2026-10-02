# Default-instance repair — source handoff

Root files are released for independent review and coordinator-only compilation. No commit, Cargo, browser, Runtime, Session, Core, provider or library API changes were made by this author.

Contract: `/tmp/frontend-run/default-instance-selection-contract-review.md` (author-ready); draft `.scratch/dioxus-case-workspace/drafts/04-default-instance-selection-repair.md`. Baseline HEAD inspected: 49b792eb11d94b8535f00fac46005f04c1e12c1a. Existing Case/Keymap/mount code retained.

Changed source and SHA256:

- `web/src/presentation.rs`: 0545037b9e8ec96599a3f4b9d00e88e4c7bdf237831666cff1399d2c7b818e57
- `web/src/presentation/instance_selection.rs` (new): 28dd03ba57c403940ae3ab9d9e7c11c039c5ce3d933d2c9587f3a72c6a07a2b9
- `web/src/presentation/objects.rs`: 0aa614633b3050fa301fc3b2d8521f021471c543cf3a9c0ad484908317ea739f
- `web/src/presentation/case_controller.rs`: bc5ce15ce817090f46801c69057fd698fda741cef520a694475db44e852f7601
- `web/src/cad_presentation.rs`: 16151541526ef3f193737bddd1a0a9775a781c4d28d5114d9a3f033b31d0a580
- `web/src/main.rs`: 67d11f57b415964a1b55e26ecb53c6142fe29a624374fb11997db2fb8016e85e

Behavior: private explicit preference is keyed by session epoch/document, and effective selection resolves that preference against the target board then falls back in document order. Automatic choices never overwrite the preference. Existing guarded navigation remains the only Event::Navigate route, with user routes resolving target defaults and recording only validated explicit choices. A version/preference-driven effect reconciles open/board/accepted changes only at Ready/saved-current with no preview, Session gesture or local pointer interaction. It rechecks captured Scope/token/revision/generation and fresh preference. Local pointer-end/cancel/Escape/workspace cleanup increments a private retry signal so deferred navigation resumes even after a camera pan that has no Session gesture.

Objects now offers only actual matching instances, with a disabled transient selection placeholder while no effective ID is yet selected. No-instance boards retain canonical scope. Unresolved Case children are withheld behind a neutral pending message; the approved additional Case files contain only narrow live policy checks before settings/generation/body submission. Existing scoped request feedback, wall defaults and persisted settings logic remain intact.

Four new source tests cover first-match/no-mechanical/no-instance policy, board-return explicit preference, remove/restore plus epoch/document reset, and real Session open/save deferral with preview/recovery gating. Root authorized the native-test-only main.rs path declaration. Tests are not run by this author; root owns all compilation.

Executed checks: rustfmt edition 2024 with skip_children on the six owned files; git diff --check. Both passed. Prior actual public red and normalized-main diagnostic green remain under `.scratch/dioxus-frontend-v1/evidence/case-keymap-current/public/`; no repaired-build green is claimed. Next gates: exact-source independent reviews, native/WASM/format checks, fresh production build and original public import→Case reproduction without manual selection, plus reviewed busy/scope/history/gesture cases.

RF-006/009 evidence remains the diagnostic scope/accounting finding; no broad refactor, parent completion or graph change. Main reset citation is correctly `app/src/useProjectSession.ts:72`.
