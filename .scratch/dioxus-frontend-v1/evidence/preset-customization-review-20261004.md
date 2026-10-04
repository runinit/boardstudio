# Preset customization repair independent diff review — 2026-10-04

Verdict: **source review passes; no concrete defects found in the frozen five-file batch.** Integration checks and public replay remain pending with the coordinator. This is not F4.6 parent acceptance.

## Reviewed identity and scope

Worktree: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`.
Branch: `codex/rust-v1-ui-parity-20261001`.
Base HEAD: `418dcb41411bba4b6225bad42f03d11419dbf525`.
Reviewed tracked working-tree diff against HEAD plus the untracked conversion module. Other staged, dirty and untracked changes were excluded.

SHA-256 of exact reviewed source:

| File | SHA-256 |
| --- | --- |
| `web/src/main.rs` | `6244d314659affac85b4582d70a52ae3fcb7f083b66742370512c2a0825e3c90` |
| `web/src/parts_assembly_preset_draft.rs` | `de22c197fcf122a26ed634bf43f45ac74299f6b8359f285940b588e5d1551fdf` |
| `web/src/presentation/parts.rs` | `4013c57f468b74ec6fbbe1e320ce0741e7620cb12adffca1e31d92da0550f09d` |
| `web/src/presentation/parts/assembly_editor.rs` | `7f8d63173b2af25a49aa562ce1cd303cf88c41a901bb79ee8727e59afaf08673` |
| `web/src/presentation/parts/assembly_presets.rs` | `44faa6c87040e684bf3b296312677a55569de17df08306a6facec14f7ce20817` |

## Standards

No actionable documented-standard violation or baseline smell found. The change adds a pure conversion in the existing native-testable frontend module pattern, keeps draft state local, and reuses existing saved-document/history delivery. The preset refactor extracts shared member construction and retains asynchronous generator parameter merge/normalization in the preview resolver. Visibility remains within the frontend crate; no Core or application API is widened.

Inspected AGENTS.md, handoff.md, CONSTRAINTS.md, domain/issue-tracker instructions and CONTEXT.md. CONTEXT-MAP.md and `.codegraph/` are absent in this worktree; targeted source reads were used. The project contract calls for one independent batch diff review, so both review axes are reported here without another review delegation chain.

## Spec

No missing or incorrect requirement found in this repair's scoped source. `PartsInspectorPanel` passes the current library catalogue definitions; `SavedAssembliesEditor` mounts the originating action for selected presets, reads preset/orientation at click time, checks current ready/saved project identity and scope, and creates an independent draft with `base: None`. Ordinary New assembly still starts empty.

`customization_recipe` deliberately supplies `false` for reversible construction and omits a transient preview-definition override. Shared construction retains switch/diode/LED parameters, library definition IDs, sides and poses; North rotates every member. `from_recipe` retains those values and uses Defaults with empty custom bindings. This matches `app/src/ui/PartsInspectorPanel.tsx` and `app/src/ui/assemblyPresets.ts`. Read-only inspection of Workbench library merging confirmed project overrides are also part of the TypeScript library definition set, matching the Dioxus catalogue semantics.

The new draft saves through the existing scope/session check and `saved_document`, which clones the latest accepted document, checks draft identity/base, merges missing definitions/assets and submits the established committed document edit. No preset selection, accepted definition or placed snapshot is edited by customization itself.

## Verification and limits

No builds, tests, packages or commits were run by this reviewer. Only this report was written.

The author's recorded native RED/GREEN exercises the real conversion mapping for a supplied MX Hotswap RGB recipe. Its empty-stub failure establishes the member conversion gap; it does not detect the absent mounted button. The test's complete member equality usefully covers parameters, IDs, sides, poses, Defaults and empty custom bindings. It supplies an already-built recipe, so it does not execute actual preset selection/construction, North orientation, single-sided policy in a reversible layout, admission, or the saved-document/history path. Those are coverage limits, not additional passing claims.

The coordinator should complete the planned native/wasm gates and paired desktop replay of Customize → parameter edit → Save, retained New assembly behavior, and scope rejection before qualification. North/reversible behavior is source-reviewed here and remains unqualified by this native test. Mobile qualification is deferred by user instruction.

Evidence reviewed: `preset-customization-repair-20261004.md`, `consolidated-34822-20261004/RECEIPT.md`, and F4.6-C01 in the current tasks record.
