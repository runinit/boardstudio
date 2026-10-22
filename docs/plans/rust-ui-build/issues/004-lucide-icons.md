# 004: Vendored Lucide icons

Status: resolved
Type: build
Blocked by: [003: Base component set](003-base-component-set.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [icon decision](../../rust-ui-migration/issues/008-choose-icon-set.md), [shared contract](../map.md#shared-contract)

## What to build

Icons come from vendored Lucide, one small component per icon, in place of the `icons` crate. Copied rust-ui components import them with only their `use` line changed, and the app's own hand-drawn icons can be replaced by the nearest Lucide icon.

## Acceptance criteria

- [x] Every icon the copied components import exists as a component taking a class, decorative by default (`aria-hidden`), sized by class (16px default) and reading its stroke width from `--icon-stroke`
- [x] A Lucide ISC notice is added to the licence file; no `icons` crate dependency exists
- [x] The accessible name of a button containing an icon contains no icon text
- [x] A short list of the app's hand-drawn icons with no Lucide equivalent (transform, keycap, geometry glyphs) is recorded in this ticket's outcome and stays custom through the same wrapper
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

Replaced the five placeholder glyphs imported by copied components (`Check`, `ChevronDown`, `ChevronsUpDown`, `Loader`, `LoaderCircle`) with Lucide artwork in the existing `ui::icons` module. Each icon is decorative (`aria-hidden="true"`), defaults to `size-4` while allowing class size overrides, and reads stroke width from `--icon-stroke` (fallback 2). The copied components already import through this module, so no behavior or import path changes were needed. The mounted test confirms an icon inside a button does not change its accessible name.

Added the pinned upstream Lucide license and source commit to `web/assets/licenses/README.txt`. Lucide's upstream notice also identifies `Check`, `ChevronDown`, and `Loader` as Feather-derived; the included license file preserves their MIT notice. No `icons` crate dependency is present.

Custom glyphs with no Lucide equivalent remain workspace-owned: transform handles and axes (translate/rotate/scale), keycap shape and legend drawings, and geometry drawings (outline points, segments, curves, and dimensions). They remain custom for their owning workspace tickets; this ticket did not sweep unrelated product controls.

Checks:

- `cargo test --package boardstudio-web-ui-shared --lib --locked` — passed, 14 tests.
- `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py browser --only ui-shared` — passed, 41 mounted tests plus the 2 selected panel tests. This is a focused subset, not the full browser gate.
- `rustfmt --edition 2024 web/crates/ui-shared/src/ui/icons.rs` and `git diff --check` — passed.

Evidence logs: `.evidence/lucide-icons/native.log` and `.evidence/lucide-icons/mounted.log`.
