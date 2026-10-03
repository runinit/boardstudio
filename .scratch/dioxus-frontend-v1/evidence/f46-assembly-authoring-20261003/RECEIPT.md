# F4.6 saved assembly authoring source packet

- Source base: `d3f39f5813135eb9dc59ea57c86497cb7fd55038`.
- React oracle: `5a472a9426e6e38993361da402cd4ec730feb369`.
- Oracle journey: `app/src/ui/PartsInspectorPanel.tsx` exposes `New assembly`, saved selection, and `Duplicate`; `app/src/ui/AssemblyEditor.tsx` edits the local draft's name, member definition, front/back side and XY/rotation before Save; `app/src/ui/assemblyEditorController.ts::saveAssembly` validates name/member count, merges referenced definitions and replaces the matching saved assembly.
- Dioxus before: `web/src/presentation/parts.rs` exposed eight preset selectors and a preset placement action, with no saved assembly list or editor.

The mounted Parts Inspector now has a project-backed saved-assemblies list, New/select/Duplicate controls and a local editor for assembly name, member definition, side and XY/rotation. Add/remove component controls complete the member draft loop. Save validates nonempty name/members, unique member IDs and finite pose values, merges missing referenced catalogue definitions, and submits the saved record through the existing `ReplaceDocument` edit/history path. It rebases against the latest accepted document, preserves unrelated edits, checks session/document/scope identity, and rejects a concurrent edit to the same assembly. Existing placements are untouched because this edit changes the recipe only.

Full F4.6 is not complete. This packet does not yet provide parameter override controls, custom/default model authoring or asset import, isolated assembly preview, or matrix/board Apply/Place. The existing F4.6 acceptance wording and F3.2 join remain intact; the matching source refinement is in `issues/04-parts.md`.

Architecture observation for the coordinator's RF accounting: `ProjectDoc` exposes assembly edits only through whole-document replacement. This editor therefore needs a private conflict check and latest-document rebase to preserve unrelated accepted state. That is a confirmed ownership/transaction-boundary constraint, not a proposed public assembly API change. There is no independent RF ID in this author packet; the coordinator owns the single RF register and post-port ledger update.

Checks run: `rustfmt --edition 2024 --config skip_children=true` on the two named Rust files; `git diff --check`. No tests, build, browser, or review were run, per the current delivery phase.
