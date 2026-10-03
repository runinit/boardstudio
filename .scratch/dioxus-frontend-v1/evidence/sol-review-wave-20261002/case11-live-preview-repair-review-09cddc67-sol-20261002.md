# Case11 live-preview bounded repair rereview

**Standards HOLD; Spec HOLD for regression proof only. All three prior source defects are repaired.** This decision concerns the repaired source packet, not full-parent or public qualification. Root has not joined this source. No source edits or heavy test reruns were performed.

## Exact review scope

- Repaired source/evidence commit: `09cddc677f81eb8ca5a9923e61c792e6fb709b99`, clean isolated worktree `/home/chris/.local/share/boardstudio/worktrees/case11-live-preview-review-repair-20261002` at inspection.
- Original implementation: `590e1eac56430b0ad6dea42edc11df9d142a4bc3`; bounded diff: `git diff 590e1eac56430b0ad6dea42edc11df9d142a4bc3 09cddc677f81eb8ca5a9923e61c792e6fb709b99`.
- Approved contract: docs commit `b71dc8fce37add45117365d8dd84fbf0785c8480`, `.scratch/dioxus-case-workspace/drafts/11-case-generation-readiness-and-live-preview-spec.md` and `issues/11-case-generation-readiness-admission.md`; read immutable contents because this isolated checkout predates publication. Pinned React `5a472a9426e6e38993361da402cd4ec730feb369` remains unchanged.
- Immutable contract content SHA-256: spec `ab083245ca77ef9490d913bc1263f067e4a4717aad0e0a91f9915cc413a17147`; issue `dc73e3712fc86ea975847211167422386fe7d6331cbacb9aa5f5a32367044383`.
- Original review: `.scratch/dioxus-frontend-v1/evidence/sol-review-wave-20261002/case11-live-preview-source-review-590e1eac-sol-20261002.md`.
- Repair handoff: `.scratch/dioxus-case-workspace/evidence/case11-live-preview-20261002/implementation-handoff.md`, SHA-256 `e4236c5e1b730820743dbd09d14baa59312882ce17cc434b906db4a5f2f0920b`.
- Applicable standards: supplied project AGENTS, `CONSTRAINTS.md`, ownership/context and issue-tracker instructions previously inspected in this project, and code-review smell baseline. This checkout has no `.codegraph/`, root AGENTS.md or CONTEXT-MAP.md. No additional baseline smell is reported.

## Standards

**HOLD — one P2 regression-proof finding; original code finding repaired.**

`Runtime::cad_scene` now calls `may_rebind_completed_case_result` with `scene.exact`, same scope, unequal token and matching nonempty physical fingerprints (`runtime.rs:1472–1519`). Unfinished and changed-fingerprint scenes remain bound to their captured token. Completed equivalent-input rebinding is narrow; no public/member visibility, wire contract, store or second cache changes occur. Old-output display uses a separate same-scope exact predicate, while current-token closure initialization and accepted-owner settings mutation remain guarded. The two concepts are appropriately separate.

**[P2] Finish the required failing-before/passing-after production regressions.** Supplied AGENTS and `CONSTRAINTS.md` require bug regressions failing for the expected reason before repair. Only the terminal retry defect has retained expected-red evidence. The exact-rebind test exercises a Boolean predicate, and the stale Inspector test injects a pre-marked row into leaf props; neither would catch the original Runtime getter or mount/Objects projection defects. The prior review specifically required a regression through the production rebind seam. Add bounded tests at those actual seams, with expected-red runs against the original behavior. This is a proof requirement, not an assertion that the inspected repaired code still contains the three original bugs.

## Spec

**HOLD — one P2 regression-proof finding; all three earlier source findings repaired.**

Completed-only reuse now rejects preview output and mismatched fingerprints. Same-scope completed display is retained by `case_workspace.rs:150–160` and `mechanical_settings_mount.rs:303–358`; generated rows and their selected contextual Inspector survive token changes and carry previous-geometry/revision labels. Other document/session/board/instance scopes are rejected. Tree/layer selection still rechecks current rendered owner/token/generation; controller mutation and closure initialization remain current. The final mechanical-finding navigation also retains its exact scene-token guard in `presentation.rs:1915–1950`.

`AutomaticCaseGeneration::observe` no longer clears `attempted` on temporary ineligibility or missing projection. The same owner therefore remains terminal; a new owner or explicit disable/re-enable admits another attempt. Native red/green evidence proves the true → false → true same-owner case.

**[P2] The new tests do not yet prove the repaired production seams.** The approved testing contract requires focused stale-output retention/owner-retirement regressions; the prior review required actual Runtime rebinding coverage. `only_exact_completed_same_owner_output_rebinds` calls only the new helper. `previous_generated_layer_context_stays_visible_and_is_labeled` mounts `MechanicalSettings` using static `is_previous: true` (`mechanical_settings.rs:1446–1503,1748–1767`); it never publishes an accepted physical edit through `use_mechanical_settings_mount`, projects Objects, or transitions an existing selected layer to a previous scene. Add tests that (1) seed production Runtime with completed/unfinished scenes and advance accepted token, asserting getter token/revisions and changed-fingerprint rejection; and (2) drive current completed scene + selected layer → same-scope changed accepted token → Objects/Inspector retention and stale labels, then owner replacement → old context absent. Keep edit/finding/closure guards asserted. Production source looks repaired; these two tests complete the source-review proof without demanding the separate public journey.

## Reused evidence and boundaries

Independently verified retained byte hashes:

- Terminal expected-red: `e1d1451a4ec8c7953b253356e853d11bf54a57cb7fb964ab2e1cfe4655a99452`; failed exactly at `!state.observe(Some(current), true, false, false)` after temporary ineligibility.
- Native lifecycle green: `4fe6cbaadf5093e6bd4cba1946a37073fbc972c1cb7594aac4d55db6e82441a6`; 6 passed/0 failed. Native configuration emits existing dead-code warnings; this does not contradict the separately reported strict WASM check.
- Mounted Inspector green: `686246afe38df4959a63e2cebaa9d51db1b2e8bf366a3ca77a56ad46890d4ea3`; 1 passed/105 filtered. Its verified scope is leaf rendering and return-to-stack behavior, as described above.

Logs live under `/home/chris/.local/share/boardstudio/retained-tmp/20261002/case11-review-repair/`. Strict WASM page Clippy and fmt success are author-reported in the receipt; raw logs for those checks were not supplied. Independent frozen-range `git diff --check` passed. No rerun was needed to establish these proof limits.

RF-003 takeaway retained: display continuity and mutation/currentness authority need distinct ownership concepts. No new RF ID or broad refactor is required. The already open paired public reopen/equivalent-input reuse/physical invalidation/blocked edit-Undo/job-count journey remains outside this bounded source rereview; it is neither a new blocker nor claimed complete. No Issue11, F7.6 or canonical parent/join status was changed.

Summary: all 3 original code defects repaired; Standards 1 P2 proof finding, Spec 1 P2 proof finding. **HOLD pending tests-only production regression evidence at exact repaired source `09cddc677f81eb8ca5a9923e61c792e6fb709b99`.**
