# Keymap Inspector composition parity — 2026-10-03

## Oracle

The paired 1280×940 Light-theme captures were supplied from Dioxus candidate 34765 and the pinned React app at `5a472a9426e6e38993361da402cd4ec730feb369`. The fixtures differ by one part in the Objects lineage (34 versus 35); this packet assesses Keymap Inspector composition and styling only, not tree/canvas or full-workbench parity.

- Dioxus before: [dioxus-before-34765.png](dioxus-before-34765.png), SHA-256 `ee883abd471baaf93660e5909e27db556e48c0351ffab17969ecbca47c35c49e`.
- React reference: [react-reference-5173.png](react-reference-5173.png), SHA-256 `087a04e899cce0161ed80fb0cf3c9ae433e9205c9fda05176ebe5146f6fc021c`.

## Observed mismatch

React renders the Keys/Macros/Encoders tabs, the active editor disclosure and its controls, then a full-width primary “Export ZMK source” action followed by “Local source export. Configure controller and wiring in PCB before building firmware.” Dioxus previously put a neutral export button between the editor tabs and active editor, omitted the helper note, and used a 12px horizontal panel inset and a generic 14px gap with a second heading rule.

## Change and boundary

The existing export button now follows whichever editor is active, uses the existing enabled flag and callback, and is followed by the pinned helper note. Keymap-local spacing follows the React Inspector content inset, title, heading-to-section rhythm and tab-to-disclosure rhythm. No provider, readiness, source, history, or delivery semantics changed. The shared Inspector/shell width and the screenshot's 34/35 part-lineage difference are outside this packet.

## Verification status

Source was compared directly with React `KeymapPanel.tsx`, `InspectorSection.tsx`, `inspector.css`, and `keymap.css`. No new UI test or package build was run per the coordinator's instruction to avoid ordinary test matrices and heavy work during the frozen root build. `git diff --check` and the one-shot Impeccable detector are the bounded local checks; an integrated paired screenshot remains open for the coordinator's next served candidate. This evidence does not close F6K.4c or F6K.4.

## Layout evidence pause

The separate F3.1 criterion accounting remains as recorded in `keymap-frontier-audit-20261003/criterion-accounting.md`; the root's already-served 34765 shared-footer observation and F3.1 acceptance join remain open. This Keymap presentation change does not alter that accounting.
