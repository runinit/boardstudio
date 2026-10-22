# 001: Walking skeleton: one copied Button, both Themes, one migrated control

Status: resolved
Type: build
Blocked by: None (can start immediately)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [spec](../../rust-ui-migration/spec.md) section 3, [theme contract](../../rust-ui-migration/theme-contract.md), [shared contract](../map.md#shared-contract)

## What to build

The smallest end-to-end path through every layer of the new design. In the running app, a user can choose a Theme (Original or shadcn) next to the existing Appearance choice in Workspace settings (ticket 010 later moves both into the header), and the choice restyles a real control that now comes from the copied rust-ui `Button`. One test finds that control by accessible role and name through a new test helper. This proves the dependencies, the copy-and-provenance routine, the token pipeline, the attributes, the saved preferences and the test rule before anything else builds on them.

## Acceptance criteria

- [x] rust-ui `Button` is copied into the shared UI crate with a provenance note naming the upstream path, commit `7120636e68556ddd023a4ec8e083663c7b745318` and the MIT notice; `tw_merge` and `strum` are added; the router `Link` branch becomes a plain `a`; the page and wasm builds pass on `dioxus =0.7.10`
- [x] `<html>` carries `data-appearance` (light or dark, resolved) and `data-theme` (`original` or `shadcn`) from saved preferences; Appearance is stored under `boardstudio:v2:appearance` and reads the legacy `boardstudio:v2:theme` value once; Theme is stored under `boardstudio:v2:workbench-theme` and defaults to Original
- [x] A minimal set of the contract's tokens exists for Original and shadcn in light and dark, enough for `Button`, plus the single `dark` variant line keyed to `data-appearance`
- [x] All the buttons of one small existing component use the copied `Button`; choosing each Theme and Appearance visibly restyles them without reload
- [x] A role-and-name test helper exists and one browser test uses it to find and click a migrated button; the existing tests for the touched files pass
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

Resolved 2026-10-22 on `lane/001-walking-skeleton`.

**What changed**
- `web/crates/ui-shared/src/ui/button.rs`: rust-ui `Button` at `7120636e`, provenance header, MIT text in `web/assets/licenses/rust-ui-MIT.txt` (listed in the licence README). `tw_merge` 0.1 and `strum` 0.28 added to ui-shared (`strum` is not used by `Button`; the ticket asked for it and later copies need it). `Link` became `a`; both elements got `data-slot="button"` (see the revised decision below).
- `<html>` carries `data-appearance` and `data-theme`. Appearance is read from `boardstudio:v2:appearance`, falling back to the legacy `boardstudio:v2:theme` until the first save; Theme from `boardstudio:v2:workbench-theme`, default `original`. Rust names follow the glossary: `AppearanceState` and `ResolvedAppearance` (renamed from `ThemeState` and `ResolvedTheme`, 12 files) and a new `ThemeState` for Original or shadcn. `ThemePicker` (Settings popover and the Library settings page) now has an Appearance select and a Theme select.
- `web/tailwind.css`: the `dark` custom variant keyed to `data-appearance`, token blocks for Original and shadcn in light and dark (only what `Button` reads: background, foreground, primary, secondary, accent, destructive, success, warning, border, input, ring, radius), the matching `@theme inline` entries and `--radius-*` scale. `m1.css`'s `:root[data-theme='dark']` became `[data-appearance='dark']`.
- `EmptyBoardCanvas` ("Add key matrix", "Browse parts") uses the copied `Button`; the old `.m1-empty-primary`, `.m1-empty-secondary` and `.m1-empty-actions > ...` rules are deleted.
- `ui-shared/src/test_role.rs` (test-support): `by_role_name`, `all_by_role_name`, `accessible_name`. Browser test `presentation::empty_board_canvas::tests::empty_board_actions_are_found_by_role_and_name` finds both buttons by role and name, clicks them and checks the disabled state.

**Revised decisions** (source documents updated: theme contract section 9, spec section 3)
- The legacy `m1.css` is unlayered and sets `button`, `input`, `select` and `:focus-visible` globally, and unlayered rules beat Tailwind's layered utilities, so the copy's colours, padding and focus ring did not apply; the app also has no preflight. Coexistence rule until ticket 024: every copied component's root carries a `data-slot`; the legacy global element rules read `button:where(:not([data-slot]))` (specificity unchanged); `tailwind.css` declares `@layer theme, base, utilities` and a four-property `@layer base` reset for `[data-slot]`.
- The legacy Tailwind alias `--color-accent` (unused) was removed because `accent` now means rust-ui's hover surface. `--color-muted` is still the legacy alias; ticket 002 maps `muted`.
- Not done here, left to 002/003: fonts and Inter, shadows, the remaining tokens, the size tokens in the copy, border stripping (cards style), loading the preference before first paint (the `:root` block gives Original light until the attributes are set).

**Evidence** (`.evidence/` in the lane worktree; no screenshots were asked for)
- `check.py lint` (rustfmt, native and wasm Clippy `-D warnings`), `typecheck`, `build` (checked page build), `deny`: pass.
- `check.py quick` (all Rust checks, because `Cargo.toml` changed): repo, tooling, lint, typecheck, test (all 2104 baselined native tests executed), layout-test-css and browser: pass. The new test ran (`run-wasm-tests.py --files web/src/presentation/empty_board_canvas.rs`: executed 1, failed 0).
- By hand on the built page (agent-browser): choosing shadcn and then Dark in Settings changed the two buttons without reload (computed `bg` `oklch(0.922 0 0)` / `oklch(0.269 0 0)`, radius 8px), the attributes and both new keys were written, and a legacy `boardstudio:v2:theme=light` was applied with no new key present.
