# Final Astra review — next Keymap tickets

**Approve publication of03 and05. Approve04 after the exact default-wording correction below; no further review is needed for that correction.** All four substantive prior findings are resolved: the macro surface is keycode-only, push-gate provenance includes the actual producer, High is the default until the scoped terminal-feedback seam is proven, and blur-only reference commits are explicit.

One remaining precision issue in04: “each new non-wait step defaults to tap of key A; changing the step-kind selector follows those same defaults” can incorrectly turn a selected Press/Release step into Tap. Replace those sentences with:

> Add step appends a tap step with key-press A. Changing the kind to wait creates wait100ms; changing it to tap, press or release keeps that chosen kind with key-press A.

This matches `KeymapPanel.tsx::MacroEditor` exactly. It does not add behavior or change the accepted Core contract.

The62-parent graph and joins remain unchanged: F6K.2 startsF6K.1; F6K.3 startsF6K.1/F6K.2 with INT.2 acceptance; F6K.4 startsF6K.1/F6K.2 with F5.2/F8.2 acceptance. Public APIs/formats/visibility remain unchanged, root/F5/shared ownership is explicit, and fixture implementation does not close integrated acceptance. Publishing these tickets does not prove their prerequisite edit/feedback seam exists or authorize dispatch before it does.

Reviewed SHA-256:
- 03 `a16095a9cde4321b40e4595fdac9bf3d7a8db392bd4d265ddef14c0c77c12c97`
- 04 `a2ddea68a8d2fd8cf7afbb9bb6d582adba27378c397584b983f71c03a71ab1e1` (the exact correction above will change this hash)
- 05 `c71e633c5c2576a2feb1e5078536e121fc9dde0920b39e49d502babdad8f58ee`

Reused prior source evidence. No repository edits, Cargo or browser runs. RF: no new refactoring takeaway observed.
