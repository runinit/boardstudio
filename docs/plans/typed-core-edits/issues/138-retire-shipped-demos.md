# 138: Retire shipped demos and their acceptance machinery

Status: claimed
Type: build
Blocked by: none

Claimed by: F coordinator `01a1273f-b992-7133-b8ed-1eae6bbc90da`. User authorized removal; root grants START after committing this claim.

## Scope and contract

Remove all shipped/demo boards, their data/archives, builders, publication/library cards, dedicated extraction/generation machinery and board-specific tests. Reintroduction is deferred until app issues are fixed. The user explicitly states the demos and their tests are untrusted: neither their geometry nor generated baselines may serve as correctness oracles. Git history is recovery; add no backup registry or board builders.

Preserve user projects and storage, reusable catalogue components/footprints/models, ordinary new/open/import/save/export operations, shared matrix/layout/mirror/link functionality, and independently specified synthetic app/layout tests. Shared regressions currently borrowing demo inputs must use small authored fixtures with expected results derived from operation contracts; do not rename, trim or copy demo boards into synthetic fixtures. Reuse already sufficient synthetic coverage instead of duplicating it.

Exact source/dependency/test classification and file lease: `/home/chris/.cache/boardstudio-baselines/demo-retirement-plan/manifest.json`, captured at integration `b618ece511339b89ece793711c68d17fa3a113cc`. Its writer actions are the only source edit lease (34 removals,21 modifications,2 compact test-only creations); root actions are excluded from lane edits. Manifest SHA256 `716625e8b0f65f57bd85692c05f55edd3c45228c3f764359a5737454205c24e6`; compact plan `/home/chris/.cache/boardstudio-baselines/demo-retirement-plan/plan.md` SHA256 `2686d4ba34b024a736b74518c278429a1a25e2327ddb50c75409c66d404b99f6`. The final manifest correction names the retained matrix fixture and a gasket test; writer/root actions are unchanged.

F checkout: `/home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2`, branch `codex/sofle-adaptations`, synced with verified Export137. Root refreshed uniquely named `boardstudio-ts-boardstudio2` successfully/current; log `/tmp/boardstudio-demo-retirement138-base-index.log`. Rebind exact HEAD/paths/hashes before edits. HIGH impact warning: shipped test loader projects has20 direct graph callers; shared tests must be ported before loader deletion. UNKNOWN UI macro/runtime method answers were source-confirmed, not treated as unused.

## Acceptance and checks

- No shipped demo assets/cards/loaders/builders remain in production or build output. Library retains normal new/import/saved-project actions. Remove demo-specific assertions and output baselines rather than reblessing them.
- Retain generic mirror namespaces/circuits/labels/refusal/undo, archive migration, generator parameters and outline lifecycle/ownership/protection contracts using independently specified simple inputs. Tiny gap/corner tests must establish their own valid geometry and explicit geometric expectations; no copied REVIUNG/Sofle coordinates or snapshots. Existing generic gasket/CAD cases remain.
- Do not change production layout/mirror/link algorithms, schemas, API visibility, component catalogue or configuration. If an actual shared bug appears, retain concrete failing evidence and escalate for a separate bounded fix; never change production logic merely to preserve old demo output.
- Preserve `web/src/presentation/export_workspace.rs` exactly at SHA256 `0e193336011ff1650b7b4c70eca29d7687142aedbe44539494bbf75a774571b0`, including137 synthetic regression.
- Root alone removes demo baseline, two Sofle gasket fixtures, one named STEP expectation and listed ignored generated demo outputs. Boundary transcript remains untouched. No broad staging, stash, push, dev merge or unrelated doc/user-data changes.
- Luna6 HIGH implements one coherent unit, with affected repo/tooling/lint/native test/typecheck/browser checks sequentially under the granted foreground lease, Cargo12. No repeated full suite; root runs full integration batch once with137. Test-first where behavior is changed; removal itself needs reference/build closure and retained synthetic contracts, not an artificial RED. Keep raw check evidence and list exactly what was removed/retained/ported.
- STOP at completed handoff; one fresh independent Sol6.1 MEDIUM review reuses sufficient source-bound writer checks and reruns only concrete gaps. Root owns refresh, complete change-analysis gates, tracking and integration.
