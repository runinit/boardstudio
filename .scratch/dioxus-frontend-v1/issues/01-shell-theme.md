# F1: Reference shell and theming

**Status:** implementing
**Blocked by:** M1 implementation
**Category:** frontend behavior-preserving port; F1 intentionally replaces the M1 demo layout with the reference shell and authorized temporary placeholders.
**Reference:** React 5a472a94; Rust base f0ac0a19.
**Authority:** [parent spec](../spec.md), [roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md).

## Work

Project menu, exact six workflow tabs and Export; Objects/canvas/Inspect/footer; light/dark/system; compact navigation. Wire existing Layout/Case/exports, honest placeholders elsewhere.

## Exit evidence

Desktop/compact/theme comparison, keyboard navigation, menu, edit/history retention and existing Case/export paths.

- [ ] Port all scoped TSX/UI-hook behavior through existing service contracts.
- [ ] Match reference visuals, themes, responsive and interaction states.
- [ ] Retain affected checks/browser evidence and resolve independent reviews.
- [ ] Update inventory, integrated source, demo and remaining limits.

A visible placeholder does not complete F2–F9. Preserve original projects and
reference source. Backend implementation/format/API changes require separate
scope; never silently remove a frontend behavior when an adapter is missing.

## First-increment requirements

Use exact labels Layout, PCB, Keymap, Keycaps, Case, Parts and separate Export. Project menu retains fixture/saved/import controls; initial no-document state remains usable. Objects contains board/instance and keyboard component navigation; Inspect contains position editing. Preserve stable selection IDs, pointer callbacks, numeric drafts and history. Include Light/Dark/System with reference tokens and scoped preference. Layout, Case and existing exports stay functional; other workspaces show honest placeholders and a return path. Desktop uses viewport panels and footer; compact uses reachable navigation/panels without horizontal page overflow. No new public Rust visibility or document/schema changes.
