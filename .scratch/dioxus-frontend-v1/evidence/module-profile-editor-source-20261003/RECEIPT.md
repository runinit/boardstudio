# Parts module profile editor source receipt

Base source: `7842edad24307fa765f01897fd24ea0b0afe172a` in the isolated
`codex/parts-definition-profiles-20261003` worktree.

The pinned React behavior is `app/src/ui/ModuleInspector.tsx` mounting
`app/src/ui/ModuleProfileEditor.tsx`. The Dioxus module inspector now mounts a
private profile editor for manual measured volumes/openings, evidence and
qualification, candidate model attachment and transforms, and supported rotary
fields. Saves use the existing `Runtime::submit(Event::Edit)` path and
`EditOperation::SetModuleDefinition`; no worker, Runtime facade, document schema,
Case assembly, or renderer change is included.

The editor captures session/document, Parts scope, selected module identity and
selection generation. Before committing it resolves the latest accepted module,
rejects deletion of an initially project-owned definition and conflicting profile
changes, then merges only edited profile fields so accepted non-profile fields are
preserved. Terminal feedback is qualified to the captured current view owner.
Manual qualification removes only the existing assembled-envelope mechanical gate
when the user reviewed nonempty, fully qualified geometry; other blockers remain.

Scope is the mounted definition-level manual editor only. Source extraction,
mounted-module wiring/placement, library-asset acceptance, Case assembly, and F4.7
parent acceptance remain open. No new refactoring takeaway was observed in this
bounded source pass.

Checks: `rustfmt --edition 2024 --check` on the three changed Rust files and
`git diff --check` pass. No Rust build/test, browser journey, or package proof was
run for this source-first packet.
