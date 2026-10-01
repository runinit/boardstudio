# Spec review: Inspector item sharing (`e7d29ce6`)

Review base: `9a5b20985a9db1f4c0f0d635c43e1dd5439bbfd9`  
Reviewed source: `e7d29ce6dd703a4d9b064cb842366538987cf442`  
Changed source: `web/src/presentation.rs` only; SHA-256 `a305af6ed3112e6c9dd4f5dc171dcf13c38f9bb0e3f3e20c6d8aa8bd3a55c980`.

**Spec result: no blocking findings.** M1 story 27 requires keyboard users to have equivalent navigation and visible focus. Ticket 06 also keeps keyboard/focus behavior in the acceptance gate. This change shares the existing immutable component-item vector through `Rc` and changes the Dioxus iteration to `iter().cloned()`. It leaves the active-board filter, item ordering, labels, selected flags, listbox/options, arrow/Home/End index calculations, selection event, focus target IDs, and click behavior unchanged. Each event closure sees the same render-time item snapshot as before, and Rc cloning avoids copying the complete list for every option handler. There is no domain, archive, event, or public API change.

The provider reports public keyboard checks for ArrowDown, Home, End, ArrowUp, Space and Enter passing with expected selected IDs/focus, no browser errors, plus format and strict WASM checks. Those are supporting provider evidence, not independent runs in this review. The focused pointer performance result is recorded separately; this review does not infer a budget pass. Full keyboard/focus and assistive-technology acceptance remains governed by ticket 06.
