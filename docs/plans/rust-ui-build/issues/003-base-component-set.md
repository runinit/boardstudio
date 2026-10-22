# 003: Base component set

Status: resolved
Type: build
Blocked by: [001: Walking skeleton: one copied Button, both Themes, one migrated control](001-walking-skeleton.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [mapping](../../rust-ui-migration/mapping.md), [inventory](../../rust-ui-migration/inventory.md), [spec](../../rust-ui-migration/spec.md) section 3, [shared contract](../map.md#shared-contract)

## What to build

The rest of the rust-ui components the mapping needs are available as copied source, ready for the accessibility and migration tickets: Input, Input Group, Field, Label, Select, Checkbox, Collapsible, Accordion, Toggle Group, Toolbar, Popover, Dropdown Menu, Dialog, Alert Dialog, Tabs, Tooltip, Slider, Combobox, Alert, Callout, Badge, Card, Separator, Kbd, Item, Empty, Spinner, Skeleton, Stepper, Toast and Table.

## Acceptance criteria

- [x] Each component is copied with provenance (upstream path, commit, MIT notice) and compiles on the pinned Dioxus
- [x] Built-in borders are removed in the copies, including `dark:`-prefixed classes (cards style, theme contract section 8); focus rings stay
- [x] Copied code contains no inline `<script>`, no `document::eval` and no document listener without an owner; a repository check or test enforces this
- [x] Interactive components are built on `dioxus-primitives` behaviour where a primitive exists, with rust-ui classes on top; where none exists (Command, Sheet, Drawer) the component is left to its own ticket
- [x] Upstream icon imports point at the shared icon module name used by ticket 004 so no `icons` crate is needed
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Outcome

The 32-file copy inventory (including the existing Button) records upstream provenance and the MIT notice, exposes `data-slot`s, uses the shared icon module, and delegates covered behaviour to the pinned primitives. The completed 32-copy review was reused; the final correction diff was read separately. Implementation commits `8e63cd5d4` and `c099a8b85` and the retained RED test commit `50d34d226` remain in lane history.

The review corrections remove the remaining Select, Combobox, Dropdown Menu and Popover popup borders, Item/Tabs base borders and redundant Badge/Item border colours. Focus rings stay. Control `h-8`/`h-9` and small icon-button sizes now consume `h-control`, `h-control-compact` and `size-control-compact`. Ticket 002 owns the utility definitions and Button sizes; this lane preserves that ownership. Item media, the aria-hidden Stepper indicator and table-header dimensions remain visual sizes. Button's Outline border removal and provenance comment were already present and were retained.

Verification on the completed copy corrections, 2026-10-22:

- Reused RED: `.evidence/003/copy-review-r3-red.log`, 2 passed / 1 expected failure at Badge's `border-transparent`. Fresh intermediate checks then reached Combobox and Item's retained borders before GREEN.
- `cargo test -p boardstudio-web-ui-shared --lib`: **14 passed**, including all three copy-rule tests (`.evidence/003/native-final.log`).
- `NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules wasm-pack test --headless --chrome web/crates/ui-shared --lib`: **42 passed** (`.evidence/003/mounted-final.log`). This includes the fresh Checkbox toggle, Toolbar role/slot and Tabs selection mounting test; the older panel-only run was not used as base-component mounting proof.
- `cargo fmt --all -- --check` and `git diff --check`: **passed** (`.evidence/003/fmt-final.log`).
- Exact-view Gortex post-edit `detect` mapped the 12 correction files to 19 symbols. `tests` found no graph-linked test targets; `guards` reported no configured rules; `contract` returned an advisory high-blast warning (34 dependents, lower bound). The actual correction diff was reviewed and the native/mounted tests above were executed; no API or parameter-object refactor was introduced. Mutation receipts and recovery evidence are in `.evidence/003/gortex-r4.log`.

The reused full quick evidence (`.evidence/003/quick.log`) remains **failed** at CAD browser qualification's `worker.onerror` (`cad/scripts/browser-smoke/qualification.mjs:22:63`). Later browser passes are not a root-cause fix or full quick proof. No full quick success is claimed. Complete keyboard/ARIA floor work remains with tickets 006–008; the mounted checks here prove copy/primitive wiring. No decision or acceptance scope was revised.
