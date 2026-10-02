# Macro compiler corrections: Spec review

**Source clear; prior private source and root mount conclusions preserved.** Read-only comparison of integration dirty source against `96874be6b5187eace0fbd65e8ba20cc76938133c`.

Exact SHA-256:

- `macro_editor.rs`: `a44edc316d3c737afc8493a3c74d4be328fe5f03b96701ae11a8fdfc06bae2d2`
- `macro_controller.rs`: `2ffa595d185a441385b62ebfe8af372f56a15dbc123fad9e76871be21f89e06d`
- `web/assets/m1.css`: `0b50ba5e8c92e1325ee5fe344cd48442153402fe07aa46a01e0b7247b198f2d8`

No material Spec finding. Corrected RSX interpolation retains macro-ID/index keys and the same accessible step label. Each event closure clones the existing macro ID and shared sequence Rc; it neither allocates another sequence stamp nor changes the captured target. The keycode clone preserves the exact accepted-value draft identity before moving the owned string into props. Ignoring the unused iterator value does not change order or indices.

Controller changes remove unused imports, outer mutability and an unused source binding, and introduce private `PreparedMacroChange` as an exact alias for the existing return tuple. The removed source check was redundant after `current_display_source(...)?`, which already requires that source and validates Scope/token/revision/generation. Admission, single-flight, shared request sequence, field merging, outcome correlation and Rc identity checks are unchanged. No visibility/API expansion.

New styles are restricted to macro classes, use existing theme tokens and remain within the reviewed KeymapPanel scroll container. Existing panel control rules supply focus styling and 44px compact targets; the new rules do not override those controls or other workspaces.

`git diff --check` passed for these files. No Cargo or source edits. The coordinator’s compilation result and actual public macro editing, lifecycle, recovery, responsive layout and firmware evidence remain separate acceptance gates. Earlier reports: `keymap-macro-source-spec-review.md` and `keymap-macro-root-mount-spec-review.md` in this directory.
