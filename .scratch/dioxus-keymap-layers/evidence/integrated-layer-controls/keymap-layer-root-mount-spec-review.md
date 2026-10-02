# Integrated Keymap layer mount — independent Spec review

**No material Spec source finding. Clear for the fresh production/public gate.** Reviewed dirty integration source over HEAD `e2d57a32de14fd31899530b4ce3b1bb52113a83c` in `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, against source-cleared controller `a8c9524ba6084aca1e777bdb76b6c3ff0fd6bc94`, published issue02 and corrected private contract. No source edits, Cargo or browser runs.

Editor invokes `use_layer_operations` unconditionally at its own lifetime, before Scope/accepted early returns and outside the conditional Keymap panel. The optional LayerSource comes from actual Runtime Scope plus accepted token/revision; no fabricated default scope is supplied. Raw stable layer ID, active workspace, adapter generation and live instance-policy predicate are passed to the cleared controller. Hiding Keymap therefore leaves the root operation observer/single-flight state alive, while workspace/scope gates suppress stale feedback/admission. Root instance normalization cannot admit a layer edit in unresolved scope.

The mounted panel receives full render Scope, raw requested ID, enabled state, filtered feedback and the operation handler, alongside the unchanged browsing and selected-key callbacks. Display fallback remains a projection decision, so remove/rejected Add does not overwrite the ID needed by Undo. Raw blur-only names, keyed draft reset, first-layer protection and stable selection semantics are unchanged.

Compiler-driven changes preserve behavior: unused reexports and unnecessary mutability were removed; an explicit `feedback_guard` keeps the read borrow alive while references are consumed. It introduces no map copies, state mutation or new lifetime authority. Controller comparison against a8c9524 shows only these changes. Panel bytes are unchanged. The five added CSS rules use existing light/dark tokens and scope layout/error/disabled treatment to the new controls; they do not alter callbacks or required controls.

Previously cleared controller guards remain: captured generation; fresh full Scope/token/revision and Ready/saved-current/no-preview/no-gesture admission; pre-submit outcome registration; retained Completed acknowledgement during transient busy states; accepted-name-key filtering for actual failures; exact acknowledgement-token mismatch diagnosis; no replay. Runtime's existing weak operation observer remains the sole terminal outcome source; no new subscription/API/schema/provider/Core path was added.

Source clearance is not a compiler, browser, persistence or accessibility claim. Fresh public tests remain required for custom/virtual Base, Add/rename/remove/fallback, raw invalid retry, unchanged-name unrelated-token errors, accepted-name reset, Undo/Redo and reopen, limits, workspace/scope changes, pending/terminal behavior, keyboard and desktop/compact themes. Keep parent/F3.1/shared acceptance joins open. RF: no new architecture finding from this mount.

Exact reviewed SHA256:
- presentation.rs `233425fcc65428a805a0efc9dc94f3981c90988d57e5717663391f91aa72b1a3`
- layer_controller.rs `75aa28a39cc7a4546e005fe0d056e25e85e9c184bf34e982e1dd719afaf08ce8`
- panel.rs `4c0b9d4c150ab78a0a2b0a128edc43cda44ecbba6da192b8efd96177d1c02c76`
- keymap.rs `7221e3aa16afec993e1ff1c0f9364fdfa43eb69656d9981e98512118fb8a48dd`
- m1.css `76101d0917ac440b441f582fceafadad026e349ca8ad5bdc6993324341d5bb75`
