# Independent Standards: integrated Keymap layer mount

Reviewed dirty integration at HEAD `e2d57a32de14fd31899530b4ce3b1bb52113a83c`, with cleared worker `a8c9524ba6084aca1e777bdb76b6c3ff0fd6bc94` as the controller baseline. SHA-256: presentation `233425fcc65428a805a0efc9dc94f3981c90988d57e5717663391f91aa72b1a3`; controller `c4af7a10b55a37a3d87ed34b6e6888f47e2e9e2484d65282e1269790b68fb022`; panel `4c0b9d4c150ab78a0a2b0a128edc43cda44ecbba6da192b8efd96177d1c02c76`; CSS `76101d0917ac440b441f582fceafadad026e349ca8ad5bdc6993324341d5bb75`.

No material Standards finding in this mount delta.

`use_layer_operations` runs unconditionally in Editor before the missing-Scope/accepted-snapshot returns and before workspace-specific rendering. Pending operation/observer ownership therefore survives hiding the panel. LayerSource is absent when real Scope or accepted snapshot is missing; otherwise it carries their actual token/revision. No fallback domain identity is invented. The hook receives the existing stable layer-ID Signal, workspace Signal and adapter generation, with a callback that rereads the current Runtime model for instance-policy admission. The cleared controller still captures generation and checks fresh Scope/token/revision/readiness/gesture state before submission.

KeymapPanel receives full render Scope, filtered terminal feedback, enabled state and the actual controller callback. Existing control identity includes Scope, layer ID and accepted name, so changed accepted names/scope replace stale drafts while unrelated updates retain them. Stable-ID fallback and Core raw-name validation remain unchanged.

Root compiler corrections remove unused private reexports/mutability and retain `feedback_guard` for the borrowed feedback lifetime. They do not alter request semantics, copying, ownership or public API. The controller continues retaining bounded intent/name data and an immutable accepted snapshot; no full-Keymap render copy was introduced.

CSS uses existing theme/error tokens and scoped selectors; inherited compact control sizing and focus treatment remain intact. No new refactoring takeaway observed; CONSTRAINTS.md reactive-lifetime/copy rules remain satisfied.

Read-only source review and diff-whitespace check only; no Cargo or browser execution by reviewer. Root compiler evidence is separate. Fresh-build public layer actions, repeated failure/retry, scope changes, stable-ID fallback/Undo and persistence-recovery acceptance remain open.

The viewer documentation closure is retained in `keymap-layer-controller-closure-standards-review.md` and the appended final viewer review; this mount does not reopen it or close broader viewer gates.
