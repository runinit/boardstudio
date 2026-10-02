# BindingEditor initial dynamic select mismatch

## Real red and bounded cause

Candidate source `e82c039b` at http://127.0.0.1:34685/. Own fresh named session `encoder-select-diagnosis`, disk profile `/var/tmp/frontend-run/encoder-select-diagnosis-profile`. Public Sofle v2 copy action, then Keymap tab, with no selected key or accepted binding edits. Read-only IndexedDB projects transaction retained the actual revision 3 Sofle record (`accepted-documents.json`, JSON-encoded CLI string); hardware boards have empty keyBindings and no authored keymap/layer assignment. Existing controller projections default missing encoder sensors to KeyBinding::None (`binding_controller.rs:395–398`); current key/push default is likewise unassigned. Independent verifier/root reproduced the same discrepancy with a genuine imported archive at revision 9, empty sensors/bindings.

Actual command: `python3 /tmp/frontend-run/encoder-select-diagnosis/initial-select-regression.py encoder-select-diagnosis /tmp/frontend-run/encoder-select-diagnosis/initial-select-red.json`. It exits1: `Fresh accepted unassigned encoder must display Unassigned, not Key press`. All 3 mounted behavior selects (CW, CCW, push) have DOM value `key-press` and `selectedOptions = ["key-press"]`, while the same editor renders no keycode fields. The push control lives in collapsed details but its mounted select exhibits the same state. Screenshot and actual DOM trace retained. No fake Core/provider or scripted storage writes. The independent public Unassigned selection corrected DOM `none` without accepted revision/sensors/bindings mutation, corroborating UI initialization rather than a default-data error.

Ranked hypotheses before diagnostic source discrimination: (1) value written before options mount; (2) wrong accepted projection; (3) a later rerender resets the browser default. Actual projection + absent keycode field + no-op public correction reject a newly authored KeyPress as the explanation. The installed creation and interpreter code directly supports (1); no application mutation or framework patch is needed.

## Version-matched source ledger

Registry base `/home/chris/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`:

- `dioxus-core-0.7.10/src/diff/node.rs`, create template path around lines 615–627: write_attrs explicitly runs before load_placeholders so attribute paths are still valid. BindingEditor's options come from dynamic for loops and are loaded after the parent select value.
- `dioxus-interpreter-js-0.7.10/src/ts/set_attribute.ts:24–34`: value on non-OPTION writes the DOM value property immediately. Setting a select value before its matching options exist does not defer the intended selection; arriving options can leave the browser's first option selected.
- Same interpreter lines 53–55: selected writes `node.selected = truthy(value)`.
- `dioxus-html-0.7.10/src/elements.rs:1536`: option.selected is a supported Bool volatile attribute. This keeps accepted selection reconciled on subsequent renders as well as option creation.

The observed null valueAttribute is expected for a DOM property assignment and is not itself the bug. This report does not claim a new framework-wide defect or ask to change dependency behavior.

## Authorized minimal correction

Only worker `/home/chris/.local/share/boardstudio/worktrees/frontend-keymap-layers-20261002/web/src/presentation/keymap/binding_editor.rs`, base `0302177aac84cb47a86598691688466a8a20e3be`. Add declarative option.selected from the existing accepted behavior, hold modifier, layer ID, and macro ID. Root explicitly authorized all 4 dynamic loops because they share the same creation path. Keep parent select.value and every change/feedback/admission handler unchanged. No lifecycle/eval workaround, default-data mutation, Core/public API change, extra state, or visibility widening.

Before SHA-256 `1b7ca799a8ecb40e530743363b72dfe078a3841e8b8d2421627f74c3347e7f76`; final SHA-256 `01354a385e46ce283f73427f993bd74eb1f0d4385abd76a89d990c14909ff74e`. Exact patch `binding-select.patch`. Owned-file rustfmt edition 2024 / skip_children and git diff --check passed. No Cargo/build/commit. No implementation-mirroring unit test was added: the regression is an actual browser property/option-mount behavior and the retained public oracle exercises that seam.

## Required gates

Independent Spec/Standards source reviews, root compilation/production build, and fresh actual browser red→green remain required. Fresh Sofle/imported empty encoder states must show Unassigned in value and selectedOptions with no code fields and unchanged accepted document. Public behavior change to Key press must show its code field; saved non-first behavior/hold/layer/macro selections must agree with the accepted binding on mount/remount and Undo/Redo. Include the ordinary selected-key shared editor so its default or existing assignments remain correct. No broad frontend or framework acceptance is claimed.
