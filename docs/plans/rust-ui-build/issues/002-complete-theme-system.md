# 002: Complete Theme system

Status: resolved
Type: build
Blocked by: [001: Walking skeleton: one copied Button, both Themes, one migrated control](001-walking-skeleton.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [theme contract](../../rust-ui-migration/theme-contract.md), [ADR-0034](../../../adr/0034-adopt-rust-ui-components-with-two-themes.md), [shared contract](../map.md#shared-contract)

## What to build

Both Themes are complete in light and dark: every token the copied components and the shell need, so later tickets only consume them. The workbench page gets the desk background that makes cards read, and shared control sizes replace upstream's fixed heights.

## Acceptance criteria

- [x] Every token in the contract's core mapping, extras (`canvas`, `selected`, `selected-foreground`, `field`, `border-strong`, `toolbar`, geometry, grid, keycap, `--icon-stroke`) and authored values (success, warning, info fills, `destructive-foreground`, slider and scrollbar colours) exists for Original and shadcn in light and dark; Original's mapped values equal today's Graphite values per the contract
- [x] `background` is the desk colour and `card` the card colour in both Themes, as in the contract's cards style
- [x] Inter Variable (latin) is bundled with its OFL licence entry in the licence file; Source Sans 3 and Source Code Pro stay for Original
- [x] The shared control sizes (40px default, 36px compact, 44px touch) are tokens identical in both Themes, and the copied `Button` reads them
- [x] The 2D canvases follow the Theme through the geometry tokens; the 3D renderer keeps its own palettes and follows Appearance only
- [x] `--wb-*` variables still exist (they retire in the Cleanup ticket); nothing in the running app regresses visually in Original
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

Completed the Theme layer beside the legacy shell: core colour, status, surface, geometry/grid/keycap, font, shadow and icon-stroke
values exist for Original and shadcn in both Appearances. The page uses the desk background; card remains distinct. Original's 49
mapped values per Appearance were compared against the pre-ticket Graphite source and match (`.evidence/theme-002/original-mapping.txt`).
Inter latin variable (48,256 bytes) and its OFL licence are bundled; the existing Original fonts remain. Shared 40/36/44px tokens feed
the copied Button's normal and icon sizes and legacy control metrics. Legacy 2D canvas, geometry, grid and keymap aliases follow
the Theme. Renderer palette files and the `ResolvedAppearance` input route are unchanged.

Changing `muted` from legacy text to rust-ui's quiet surface required the existing Snap label to use `muted-foreground`.
The Theme contract records this, the authored info/shadow/size names, and the shadcn light desk value (`oklch(0.965 0 0)`), which
intentionally differs from rust-ui's white background so cards remain visible. No interaction, project format, preference, public
API or test-name changes. The existing shell's other legacy colours remain for later migration tickets.

### Verification

Cargo and wasm-pack commands ran serially against lane002's isolated target, with project concurrency settings. Browser commands use
`NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules`; check commands use `BOARDSTUDIO_CHECK_JOBS=1`.

- RED: saved `scripts/test-theme.py http://127.0.0.1:4202/boardstudio/ --evidence .evidence/theme-002/resume-red` reached
  `Legacy canvas alias` on shadcn light after Original light/dark passed (`resume-red.log`). Both legacy canvas definitions now
  point to `var(--canvas)`; the saved test's already accepted accessible-name bootstrap correction was preserved.
- GREEN: the same command with evidence directory `.evidence/theme-002/final-green`, then `.evidence/theme-002/merged-green`
  after rebuilding the merged tree. Four combinations pass: 81 nonempty tokens, distinct desk/card, mounted canvas background,
  geometry/grid aliases, transition-settled 40/36/44px utilities and nonempty loaded font faces. Logs: `final-green.log`,
  `merged-green.log`; observations: `merged-green/mounted.txt`. All four `merged-green/{original,shadcn}-{light,dark}.png`
  screenshots were inspected. This is scoped empty-editor evidence, not all-workspace visual acceptance.
- `python3 scripts/check.py build lint repo`: PASS after merging feature commit `cb58198cfb3fcb032b452e016a95559593c09caf`
  (`merged-checks.log`). The earlier pre-merge build also passed (`final-build.log`).
- `cargo test --package boardstudio-web-ui-shared --lib --locked`: 14 passed, 0 failed (`merged-native.log`).
- `python3 scripts/check.py browser --only ui-shared`: 42 mounted tests passed (40 + 2 panel tests), 0 failed
  (`merged-mounted.log`). The panel launch spent 254.4 seconds including module startup; its tests completed in 0.29 seconds.
  Read-only diagnostics captured the loading page; no retry, timeout increase or assertion change was used. This is a partial
  browser subset, not the full browser gate.
- `python3 scripts/run-wasm-tests.py --files web/src/presentation/empty_board_canvas.rs --result-json .evidence/theme-002/merged-empty-board.json`: 1 executed, 0 failed (`merged-empty-board.log`).
- `python3 -m py_compile scripts/test-theme.py`, `git diff --check` and the documented Inter SHA-256: PASS.
- Combined Standards/Spec source self-review: `.evidence/theme-002/self-review.txt`; no additional ticket002 blocker.
  No subagent review was launched, per the user's explicit no-extra-agents instruction.

The original handoff's build/lint/repo/empty-board evidence was reused while resuming, then superseded by the fresh merged-tree
checks above where listed. Full `quick`/browser/CAD acceptance was not rerun; the earlier CAD `worker.onerror` remains unresolved.
First-paint preference timing is unverified and remains an explicit open build detail in the Theme contract, not a claimed guarantee.
The migration-wide workspace, accessibility and final review gates remain with their owning tickets. Gortex was uninstalled by the
user; its former graph gates do not apply.

### Integration

Implementation commit `53bbacb0`; feature merge `0ee0d300` includes tickets003/005. The sole Button comment conflict was resolved
with both 003's Outline-border removal/provenance comment and 002's shared size tokens retained. No 004/006 source was edited.
Local fast-forward landing is recorded in the lane's completion report; no origin push or dev merge is performed.
