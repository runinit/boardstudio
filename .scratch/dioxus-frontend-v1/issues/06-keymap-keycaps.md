# F6: Keymap and Keycaps frontend

**Status:** planned
**Blocked by:** F3, F4; F5 hardware handoff
**Category:** frontend behavior-preserving port; F1 intentionally replaces the M1 demo layout with the reference shell and authorized temporary placeholders.
**Reference:** React 5a472a94; Rust base f0ac0a19.
**Authority:** [parent spec](../spec.md), [roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md).

## Work

Keymap/layer/key/binding/behavior/macros/encoders and firmware panel; keycap profile/size/legends/fit/2D/3D controls.

## Exit evidence

Every reference-supported control and draft/history state, legacy saved settings, keyboard/visual parity.

- [ ] Port all scoped TSX/UI-hook behavior through existing service contracts.
- [ ] Match reference visuals, themes, responsive and interaction states.
- [ ] Retain affected checks/browser evidence and resolve independent reviews.
- [ ] Update inventory, integrated source, demo and remaining limits.

A visible placeholder does not complete F2–F9. Preserve original projects and
reference source. Backend implementation/format/API changes require separate
scope; never silently remove a frontend behavior when an adapter is missing.

## Reference ownership

The [file ledger](../evidence/tsx-inventory.json) pins exact reference hashes. These
are source responsibilities, not permission to mark whole shared files complete.

- `app/src/ui/FirmwareKeymapPanel.tsx`
- `app/src/ui/KeyBindingEditor.tsx`
- `app/src/ui/KeySizeControls.tsx`
- `app/src/ui/KeycapPanel.tsx`
- `app/src/ui/KeymapLayout.tsx`
- `app/src/ui/KeymapPanel.tsx`
- `app/src/ui/createKeymapWorkspace.tsx`
