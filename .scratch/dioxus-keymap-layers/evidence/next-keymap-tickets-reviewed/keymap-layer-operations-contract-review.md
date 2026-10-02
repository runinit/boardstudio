# Independent Standards/design: corrected Keymap layer operations contract

Re-reviewed contract SHA-256 `3dd9ba6fd2a8bb16654a50fc41ec2077d29d5817970ea5b7e0b711a580846b94`, blob `4223754b277381c76605760b41f89129e1eb2239`, against mounted source `dc81237c51c9d4f43ddb7f0d3f6f7eda9cf685fc` and integration HEAD `49b792eb11d94b8535f00fac46005f04c1e12c1a`.

**Clear for bounded authoring once root records the bounded 01 prerequisite.** Both P2 design findings from contract blob `9944129b` are resolved:

- Synchronous single-flight admission checks Ready/Saved-current/no-preview/no-gesture and records the outcome slot before submission. All terminal variants clear it. Root Editor ownership survives Keymap panel unmount, suppresses wrong-workspace feedback, and retains admitted operation tracking without replay after scope change. Successful completion correlates retained request/operation and current Scope while explicitly allowing the accepted token to advance. Repeated identical failures use new request IDs and preserve retryable draft text.
- Virtual Base rename on `keymap == None` is explicitly admitted through core's existing `unwrap_or_default()` semantics; rendering alone remains read-only. `Some(empty)` remains an acknowledged invalid/legacy discrepancy rather than a page repair.

The corrected prerequisite wording matches published ticket 02: bounded 01 projection/stable-layer-ID proof gates authoring; F3.1 remains an integrated acceptance join. The contract accurately preserves first-layer removal protection regardless of ID/name, exact raw-name validation, native maxLength, stable-ID fallback/Undo, root entity/operation ID separation, and existing typed EditKeymap/history/save authority.

`CONSTRAINTS.md` state ownership, reactive lifetime and copy requirements are adequately addressed at design level. Implementers must retain root pending ownership and fresh guards when composing private panel callbacks; the existing outcome observer alone does not prove the resulting UI. No new public API/source boundary is required.

Keep High profile until the edit seam is implemented and proven. Independent exact-source reviews and public retry/history/scope/keyboard/accessibility/persistence evidence remain required. Base/read-only green and underclassified imported fixture differences are not broader acceptance.

RF: no new architectural takeaway; preserve RF-009 and parent joins. Source-only review, no edits/Cargo/browser execution. This is contract clearance, not feature acceptance.
