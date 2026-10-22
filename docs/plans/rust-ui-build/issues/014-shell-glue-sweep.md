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

## Owned surface and overlap inventory

Refreshed against base `3c0f6e26c17e53312eb29e67ca86b18c6fb2732a`, before production edits:

| File / hunk | Remaining migration at claim | 014 ownership |
| --- | --- | --- |
| `workspace_navigation.rs` | `InspectorActions`: legacy Toolbar/Button | Wrappers only; delayed owner/generation guard and focus IDs retained. Header workspace Buttons already migrated. |
| `workbench_shortcuts.rs` | No controls | Read and preserve existing shortcut routing. |
| `workspace_composition.rs` | No controls | Read and preserve routing and domain projections. |
| `inspector.rs` | Two native number inputs, legacy Apply button, inline messages | Field/Label/InputGroup composition, Button, FieldError, guidance Callout; parsing, drafts, precision, transactions, IDs and callbacks unchanged. `BoardPartChoice` is a domain type, not a legacy control. |
| `canvas_status_footer.rs` | Four raw camera buttons, Layout-owned raw findings presentation | Toolbar and zoom Tooltips; private findings Button/Badge with existing trigger ID and callback. |
| `empty_board_canvas.rs` | Buttons already migrated; legacy empty markup/CSS | Copied Empty composition; existing two callbacks and disabled state retained. |
| `presentation.rs` | Raw footer Undo/Redo; findings fallback delegates raw presentation | Only shell stylesheet link, footer Toolbar, and findings fallback call. Test-only ExportPanel and Case/setup controls belong to later tickets and remain. Layout/workspace hunks reserved. |
| `test_role.rs` | Number inputs and associated native labels unsupported | Existing test-only helper gains native `spinbutton` and `label[for]` lookup. No API or visibility change. |
| Styles / tests | Shell-only rules and old inspector test locators | Remove only exclusive footer/empty rules; retain shared legacy CSS for 024 and Layout findings CSS for 016. New `shell.css`, shell browser test, mounted owner/callback checks; existing inspector tests keep their names. |

No edits to ticket 015's Layout files, CSS, or InputGroup API. No new dependency, shared product API, visibility widening, domain state or event is introduced. The legacy input's keyboard handler now bubbles from the same native input through a field-local wrapper, preserving native type/step and ownership without changing the shared input API.

### Initial evidence

- `.evidence/014-red-footer.log`: actual mounted RED, one failure at `camera controls adopt the accessible Toolbar` while footer still used a raw group.
- `.evidence/014-green-mounted.log`: first GREEN attempt blocked at compilation by Toolbar integer-index inference; corrected with explicit `usize` indices.
- `.evidence/014-green-mounted-r2.log`: intermediate 21 mounted tests passed. Later shell findings/tooltip identity and navigation fixture changes require fresh final checks.
- `.evidence/014-repo.log`: `python3 scripts/check.py repo` passed.
- Physical target: `/home/chris/.cache/boardstudio-target/014-shell-lineage`, linked by this checkout's `target`; `.claude/settings.json`, `cad/.cache`, and `cad/wasm/pkg` copied from the primary checkout before builds. Single Cargo/wasm-pack launch per target; project concurrency unchanged.
