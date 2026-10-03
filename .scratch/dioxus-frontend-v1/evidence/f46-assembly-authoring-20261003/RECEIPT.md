# F4.6 saved assembly authoring source packet

- Source base: `d3f39f5813135eb9dc59ea57c86497cb7fd55038`.
- React oracle: `5a472a9426e6e38993361da402cd4ec730feb369`.
- Oracle journey: `app/src/ui/PartsInspectorPanel.tsx` exposes `New assembly`, saved selection, and `Duplicate`; `app/src/ui/AssemblyEditor.tsx` edits the local draft's name, member definition, front/back side and XY/rotation before Save; `app/src/ui/assemblyEditorController.ts::saveAssembly` validates name/member count, merges referenced definitions and replaces the matching saved assembly.
- Dioxus before: `web/src/presentation/parts.rs` exposed eight preset selectors and a preset placement action, with no saved assembly list or editor.

The mounted Parts Inspector now has a project-backed saved-assemblies list, New/select/Duplicate controls and a local editor for assembly name, member definition, side and XY/rotation. Add/remove component controls complete the member draft loop. Save validates nonempty name/members, unique member IDs and finite pose values, merges missing referenced catalogue definitions, and submits the saved record through the existing `ReplaceDocument` edit/history path. It rebases against the latest accepted document, preserves unrelated edits, checks session/document/scope identity, and rejects a concurrent edit to the same assembly. Existing placements are untouched because this edit changes the recipe only.

Full F4.6 is not complete. This packet does not yet provide parameter override controls, custom/default model authoring or asset import, isolated assembly preview, or matrix/board Apply/Place. The existing F4.6 acceptance wording and F3.2 join remain intact; the matching source refinement is in `issues/04-parts.md`.

Architecture observation for the coordinator's RF accounting: `ProjectDoc` exposes assembly edits only through whole-document replacement. This editor therefore needs a private conflict check and latest-document rebase to preserve unrelated accepted state. That is a confirmed ownership/transaction-boundary constraint, not a proposed public assembly API change. There is no independent RF ID in this author packet; the coordinator owns the single RF register and post-port ledger update.

Checks run: `rustfmt --edition 2024 --config skip_children=true` on the two named Rust files; `git diff --check`. No tests, build, browser, or review were run, per the current delivery phase.

## Follow-up: model authoring and isolated draft preview

- Follow-up source base: `7b7265f87e7658015800d1442ea15dfa2604fcec` plus the compile-ownership repair `d11d9b96af883c2db778e387763e2798e48d01e3`.
- Shared importer dependency: `20595f43557a5e591a02ee2deadf78c2ff0be79f` (cherry-picked as `c46a5d0d` in the private source branch). The caller uses its verified STEP/STP/STL/WRL reader and BrowserStore persistence, then owns the collision-checked document `Asset` metadata in the same local assembly draft.
- Reference checked: `AssemblyEditor.tsx` model mode, model rows and transform controls, `attachAssemblyAsset`, and `sampleAssembly`/`AssemblyViewer`. The editor now carries local draft assets, supports visual-only members, default/custom mode, model asset bindings and transforms, imports, parameter/side overrides, and feeds the composition to the existing isolated `PartsPreviewPanel` in 3D mode. `PartsPreviewCapture` merges draft assets into its disposable sample and preserves member generator overrides.
- Bounded gaps: the retained TS `modelBindings` call derives Ergogen-generated model bindings when “Edit model defaults” is selected; this Rust composition currently copies declared definition models and does not reproduce that dynamic derivation. Apply/Place remains held for the F3.2 placement seam. The canonical F4.6 criteria remain open.
- Architecture takeaway: no new shared renderer or writable document owner was introduced. The existing Parts recipe-capture owner can carry local asset additions and pose/model snapshots into an isolated sample; import parsing/storage remains in the shared verified model-file adapter while assembly-specific asset metadata stays in the local draft. No additional refactoring takeaway was recorded; the original accepted-document rebase/conflict finding above remains the concrete RF item for this slice.
- Checks: `rustfmt --edition 2024 --config skip_children=true` on changed Rust sources; `git diff --check`. No builds, tests, browser work, or review were run, as requested for this delivery phase.
