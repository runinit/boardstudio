# 014: Shell glue sweep

Status: claimed
Type: build
Blocked by: [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md), [012: Command palette](012-command-palette.md), [013: Toasts](013-toasts.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The remaining shell files have no raw controls and no `Board*` left.

## Acceptance criteria

- [ ] The inventory's app-shell rows that belong to the shell (workspace navigation, shortcuts, composition, inspector host, status footer, empty canvas) have zero raw controls and zero `Board*` uses
- [ ] Messages are classified by the mapping's rule; icon-only buttons have Tooltips; selects follow the 12-option rule
- [ ] The shell works in both Themes and both Appearances at desktop, mid and narrow widths, with screenshots recorded in the outcome
- [ ] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Claim

- Owner: implementation child in T3 worktree `t3-cff86af3`, branch `feat/rust-ui-shell-glue-sweep`.
- Initial checkout was clean at `b237903705244a266b07a64e5f23b8a3b5219c03`; fast-forwarded the latest local `feat/rust-ui-migration` to `3c0f6e26c17e53312eb29e67ca86b18c6fb2732a` before claiming.
- Lease: shell navigation, shortcuts, composition, inspector host, status footer and empty canvas; `presentation.rs` shell hunks only. Layout workspace hunks and ticket 015's Layout files/CSS/InputGroup API remain reserved. No child agents, threads or watchers.
- Landing held for coordinator approval and one combined Spec + Standards closure review at a clean final pin.
