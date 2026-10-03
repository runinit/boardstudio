# Compact shared-shell parity author packet

Base: `698cee6077afd407c424cc8a1296a2758457ec8d` on `codex/compact-shell-parity-20261003`.

## Paired source oracle

Pinned React `5a472a9426e6e38993361da402cd4ec730feb369` at `http://127.0.0.1:5173/`; frozen Dioxus `9e6f9b62e33d7dc22412cf5793a3f4b857e219c` at `http://127.0.0.1:34768/boardstudio/`. Both were opened in a private `agent-browser` worktree session and inspected at 720×640 with an active Sofle Layout.

React exposes Objects, Inspect and Export buttons beside the Workspace selector. The selector lists the six workspaces while Layout is active. Dioxus candidate 34768 exposed Objects/Inspect under a second `Panel visibility` row, included Export in the selector, and had no visible separate Export action. The paired `.scratch/dioxus-frontend-v1/evidence/candidate-34768-20261003/objects-pane-receipt.md` further records the inline-panel versus side-drawer mismatch.

The pinned React `useCompactPanel` thresholds are 980px for Objects and 820px for Inspect; its workspace selector begins at 1050px. The compact shell issue now carries the 720×640 interaction criteria and these thresholds.

## Source changes

- Move the existing Objects/Inspect open signals to the app shell and pass the same signals to topbar controls and panel leaves.
- Put compact toggles in the topbar; opening one closes the other. Keep Export separate and expose an Export selector entry only while Export is active.
- Render compact panels as mutually exclusive side drawers with a scrim; retain existing panel settings storage, feature draft owners and Session/Core authority.
- Match the independent 980px/820px panel breakpoints and leave desktop panel mode/width behavior intact.
- Refine the existing F1 issue and F2.4 bounded shell spec; record the shared visibility/focus routing observation under RF-001 and POST-PORT.

## Checks and open work

`rustfmt --edition 2024 --config skip_children=true web/src/presentation.rs web/src/presentation/panels.rs` passed. The combined package/build check and one changed paired compact drawer/export journey are intentionally left to the coordinator after integration. No tests or canonical task statuses were changed here.
