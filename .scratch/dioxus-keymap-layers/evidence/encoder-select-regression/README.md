# Dynamic binding select regression

The genuine imported Sofle archive and fresh built-in Sofle both reproduced the initial failure on source `e82c039b` / demo 34685: accepted unassigned rotations and push bindings displayed the first `Key press` option, with no code field and no document mutation. The retained public DOM assertion exited 1 for the expected reason; see `encoder-root-initial-select-red.log` and the independent genuine-archive packet in `../encoder-public-workflow/`.

Installed Dioxus 0.7.10 applies parent attributes before dynamic options are inserted. The exact diagnosis/source ledger is retained in `DIAGNOSIS-AND-HANDOFF.md`. Four declarative `option.selected` expressions mirror existing accepted behavior, hold modifier, layer ID and macro ID. No handlers, defaults, Core behavior or public APIs changed. Independent Spec and Standards reviews cleared SHA-256 `01354a385e46ce283f73427f993bd74eb1f0d4385abd76a89d990c14909ff74e`.

Root integrated the fix as `868edfcbdf93315e866962c9c57543d26672f379`. Strict WASM all-targets Clippy with warnings denied, formatting and whitespace checks passed. The frozen build `frontend-encoder-select-fixed-20261002` passed all eight commands and verified 973 source hashes. Native tests had already passed 34; this WASM presentation-only correction does not alter those native modules.

The same read-only DOM regression script now exits 0 on a fresh built-in Sofle at demo 34687: all three mounted behavior selects are `none`, selectedOptions is `["none"]`, and there are no code fields. Root viewed the final themed screenshot, retained as `encoder-root-final-controls.png`. Genuine archive checks and non-first selection/persistence results are reported separately by the two public verifier packets; the old failing evidence is retained.

This is a bounded application correction, not a framework-wide defect or full Keymap acceptance claim. Actual F5 input-plan/fingerprint, attached-module rejection, firmware delivery, assistive technology and the parent joins remain open. Existing RF-009 records the source/UI projection lesson; no new architectural finding is asserted from this DOM behavior.
