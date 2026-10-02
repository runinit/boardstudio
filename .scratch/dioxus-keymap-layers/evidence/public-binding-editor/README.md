# Mounted binding editor: public UI verification

Candidate: source `6509f557c2a16df0a1f5ce19296eeb635df68256`, build `frontend-keymap-bindings-20261002`, served at `http://127.0.0.1:34673/` and `/boardstudio/`; the copied artifact provenance is `provenance.json`. The detailed public checks used a fresh named Chromium session `public-binding-editor-candidate-native-6509f557`, disk profile `/var/tmp/frontend-case-binding-editor-candidate-native-6509f557/profile`, and the candidate's ordinary public Import control. Other filenames marked `reference-*` are paired React controls on `http://127.0.0.1:5173/`, each in its own named session/profile.

## Matched archive

The input `reference-layered-bindings.boardstudio` is the genuine React public export from the companion [binding editor reference packet](../binding-editor-reference/README.md), SHA-256 `c9aa6a4e387fc206fd3a4a0e944b140f095ba8774a8ab5bdc93a99d74e19c9ea`. The candidate's readonly imported document (`imported-project.json`) exactly equals the archive's embedded `project.json`: same ID `m1-sofle-v2-copy`, revision 29, both layers, all binding IDs and values, and macro record. This exact record match was verified before any candidate actions.

## Public controls and defaults

Through the candidate's visible Keymap controls, I selected each behavior on an actual key. `candidate-behavior-panels.txt` preserves the conditional panels. The eleven choices are Key press, Mod tap, Layer tap, Momentary layer, Toggle layer, Go to layer, Sticky layer, Sticky key, Macro, Transparent, and Unassigned. Defaults observed: Key press `A`; Mod tap tap `A` / hold `LSHIFT`; Layer tap tap `SPACE` / target `Function`; Momentary, Toggle, Go to, and Sticky layer target `Function`; Sticky key `A`; Macro selects `Reference Macro` because the imported document contains one; Transparent and Unassigned have no extra fields. The mode loop used a temporary candidate key and is not part of the exported baseline.

The actual Layer selector exposes `Main` with value `base` and `Function` with value `dc238a57-6d6c-4eaa-b6c2-061d2eedec00`; a public layer-tap selection wrote the selected stable ID to the saved `layerId`. Undo and Redo restored the Function UUID and then `base` (`layer-target-*.json`). The Macro selector shows `Reference Macro` with value `2110cfa6-98b3-429b-b5ff-d019722b7d4b`; the same ID is in the saved SW3 binding and its macro record (`macro-identities.json`, `macro-id-persisted.json`).

## Draft recovery, history, and persistence

On SW4, I changed the public behavior to Key press, typed invalid `A)`, then entered a no-match string in Find a key. The editor remained mounted, continued to show raw draft `A)`, while the selected-key list showed no match. Readonly saved state remained `A` at revision 41 (`sw4-invalid-no-match-fresh.txt`, `sw4-invalid-persisted-fresh.json`). Replacing the draft with `LC(LS(A))` and blurring by clearing Find a key saved that nested expression at revision 42. Public Undo restored `A` at revision 43 and Redo restored `LC(LS(A))` at revision 44 (`sw4-corrected.json`, `sw4-undo.json`, `sw4-redo.json`).

I changed tap and hold fields separately on SW5 and SW6. With the app idle between edits, readonly saved records preserve both fields (`sibling-tap-blur.json`, `sibling-fields-final.json`). A rapid CLI `select` sequence can dispatch a hold change before its preceding tap blur has committed; in an isolated public candidate profile this appeared to leave the prior tap visible/saved. The identical rapid sequence in paired React did the same (`sibling-fast-sequence.json`, `reference-sibling-fast-sequence.json`). Astra's separate event-order diagnosis and actual pointer/keyboard protocol are retained at `sibling-diagnosis/README.md`; do not treat the agent-browser ordering result as a candidate-only migration difference.

For actual keyboard interaction, SW8/SW9 SVG buttons accepted Enter/Space while focused (`key-focus-*.json`, `key-enter-result.txt`, `key-space-result.txt`). On SW6, Tab from the tap field moved focus to BODY in this candidate; after blur saved tap `Y`, a later real pointer click on Hold while idle focused the select, and native Home/ArrowDown/ArrowDown/Enter changed Hold to `LCTRL` while preserving tap `Y` (`candidate-native-*.json`). In paired React, Tab moved from Tap directly to Hold; the same later native select interaction also preserved tap `Y` (`reference-native-*.json`). This is a recorded focus-order difference and is not being treated as a synthetic dropdown parity result.

The UI switched Main → Function and SW1 → SW2 using real controls: Main/SW1 shows Unassigned, Function/SW2 shows the imported `Reference Macro`. Switching Layout → Keymap returned to the previous Function/SW2 view (`main-layer-sw1.txt`, `function-sw2.txt`, `after-workspace-switch.txt`).

The candidate's public Export → Export archive download is `candidate-exported.boardstudio` (SHA-256 `23aa1d651eab68f8bb897387360d5e28f3a93bb55d21a1fbf53358ce1535ee04`). Its embedded project record exactly equals the readonly pre-export document at revision 40. After a full browser reload, the readonly stored document remained exactly equal to the pre-export record (`candidate-before-export.json` and `candidate-after-reload.json`). The app returned to the Layout workspace after reload; opening Keymap → Function → SW1 displayed the persisted `LC(LS(A))` binding (`after-reload-function-sw1.txt`).

Screenshots include [the imported Function layer](baseline-selected-function.png), [recovered nested binding](final-keymap-after-recovery.png), [macro binding](final-macro-binding.png), and [invalid draft while Find has no matches](sw4-invalid-no-match-fresh.png).

## Limits

This verifies the public UI and accepted stored records for these control paths on this exact build and matched archive. It does not execute the firmware semantics of every behavior, test every valid ZMK expression, cover legacy/malformed persisted keymaps, or claim assistive-technology testing. The Tab focus difference is recorded above; no wider accessibility or release acceptance is claimed.
