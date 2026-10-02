# Explicit current-instance activation repair

Source released for independent review; no Cargo/build/browser/commit by this author.

Finding: the previous native select emitted explicit preference only on change. Unlike React Case assembly rows, it provided no dependable human action to explicitly choose the already-effective fallback. A2 explicit → board B fallback B1 → explicit B1 → return A must therefore have a real B1 activation route before claiming parity.

Bounded correction: only in Case, Objects now shows the existing matching physical instances as native buttons in a labelled group. Actual Session selection drives aria-pressed. Every button click, including the already-pressed item, calls the existing guarded Some(instance_id) navigation wrapper. Native Enter/Space activation supplies the same route. This records explicit intent without using focus, dropdown opening or forced change events. Other workspaces retain their existing selector, and no domain/default policy or Session behavior changed.

Focused CSS uses existing theme tokens, visible focus and active state, wrapping labels, 36px desktop and 44px compact buttons. This is a selection affordance correction, not the full future Case assembly tree or F7.7 completion.

Exact sources:
- web/src/presentation/objects.rs SHA256 39954b6247b2f0529bd26a3206ad199afea060597242da2feb35a4011176d9d5
- web/assets/m1.css SHA256 0e4781c4886d33c600f37c3623e80f30883eb5b07b5d4323249fec79146255e3

Rustfmt (edition 2024, skip_children) and git diff --check passed. Root owns compilation/fresh build. Required public green must use a real pointer click or keyboard button activation for B1, verify return to A resolves A1, and verify passive board visits still preserve A2. Do not substitute selectOption-dispatched onchange as evidence. Existing initial-open, scope/history/gesture and full parent gates remain open.
