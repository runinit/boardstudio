# 018: Case settings, part 2: mounts, openings, battery, manufacturing and pairs

Status: resolved
Owner: lane/018-case-settings-2 (.worktrees/lane-018-case-settings-2)
Type: build
Blocked by: [017: Case settings, part 1: dimensions, construction and profiles](017-case-settings-1.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The second half of the Case mechanical settings: mounting and mount collections, openings, battery, manufacturing and the paired-feature controls. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [x] The remaining groups of the settings file have zero raw controls and `Board*` uses
- [x] The settings file as a whole shows zero in the inventory's counts; existing tests pass after the rewrite rule
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Answer

All 018-owned groups of `web/crates/case/src/mechanical_settings.rs` are migrated; the
017-owned components, `SettingsSelect`/`ProfileTargetField` and the out-of-scope
`StackupPreview` (SVG only, no controls) are untouched. Production now has zero raw
`button`/`input`/`select`/`textarea`/`details` and zero `Board*` uses (the `Board*`
import is deleted; remaining "Board" strings are the `MechanicalBoardMismatch` domain
type and one test JSON literal).

What changed, per group (all TDD red→green in vertical slices, behavior and callbacks
unchanged; `p.m1-mechanical-help` kept per 017):

- Text: `TextDraftField` → `Field` + `FieldLabel` + `InputGroup` with `InputGroupInput`
or an extended `InputGroupTextarea` (additive optional props only; it had zero
callers), errors via `FieldError` + `aria-describedby`, `Saving…` stays `role=status`.
- Pairs + Linked halves: `MechanicalPairMemberRemoval` (all `Ghost Sm`, the legacy
control analog), `MechanicalPairLinkControls` and `MechanicalPairControls` → `Button`,
`SettingsSelect`, new local `SettingsChoiceField` (the 12-option Select/Combobox
boundary, 020 `PartsChoiceField` precedent) and `SettingsNumberField`; pair failures
and preview findings are Destructive `Alert`s; Linked halves is a keep-mounted
`Collapsible`.
- Mounts: `MountRow` → `SettingsSelect` + `Button`; collections and suggested
locations → keep-mounted `Collapsible`s; empty collections are `Callout`s.
- Openings: `OpeningControls` → keep-mounted `Collapsible` + `Button`s; the
short-footprint message is a Destructive `Alert`.
- Battery + disclosure: envelope toggle → `FieldLabel` + controlled `Checkbox`.
- Closure + fits: both disclosures → keep-mounted `Collapsible`s (open states
preserved as `default_open`); six closure selects → `SettingsSelect`/`SettingsChoiceField`;
`Mount link` and `Generated part` → `SettingsChoiceField`.
- Manufacturing + disclosure: raw Method select and Material/Stabilizer fields →
`SettingsSelect`/`SettingsChoiceField`; process add/remove are named `Button`s; the
unsupported-constraint notice is a Destructive `Alert`.
- Contextual/gasket/diagnostics: navigation → `Ghost Sm` buttons; findings lists →
`ItemGroup` + `Item` + severity `Badge` + `Button`; notices → `Callout`s in
`role=status`; `Mechanical diagnostics` → keep-mounted `Collapsible`.
- Shell + setup: retry/reset/disable/configure → `Secondary` buttons; setup basics →
`SettingsSelect` + controlled `Checkbox`; shell notices/failures classified (`Warning`
callouts for mismatch/configured-elsewhere, Destructive `Alert`s for failures,
`Saving…`/loading stay `role=status`).
- Row actions gained unique accessible names starting with the visible label
(`Remove volume N`, `Remove access opening N vertex M`, `Add vertex to volume N`,
`Remove Hole/Boss N`, `Remove hardware/process/fit …`, `Show finding …`), following
017's unique vertex names. No `Tooltip` was needed: every button has visible text.
- CSS: deleted only rules this file used and nothing else references (verified by
repo-wide grep): the mechanical-settings descendant control rules + narrow override,
the orphaned field/check/alert rules, and the setup-choice rules. Old-CSS count
moves 614 → 611 selectors on this lane (579 after the 021 merge). Layout, openings,
spec-card, pair-row, profile, help and group rules stay.

Commands and results (lane target `~/.cache/boardstudio-target/018-case-settings-2`,
no build-flag overrides; logs in the worktree's `.evidence/018/`):

- `wasm-pack test --headless --chrome web/crates/case --locked --lib`: 94/94,
including 8 new/rewritten 018 mounted tests (role+name; options pointer-tapped,
the event the primitives commit on). Each slice ran RED on old markup first.
- `wasm-pack test --headless --chrome web/crates/ui-shared --locked --lib --
ui::input_group::`: 3/3 (extended textarea forwarding fixture).
- `python3 scripts/test-case-settings.py`: 13/13 PASS — 017's styles, behavior and
10 pointer proofs still pass with the extended fixture, plus new `tests_018`
(disclosures, Select keyboard/typeahead/focus-return, the 14-choice Mount-link
Combobox boundary, battery checkbox incl. label activation, text-draft validation,
typed buttons, narrow 44px targets, four Theme/Appearance captures).
- `cargo test --locked -p boardstudio-web-case --features test-support`: 34/34.
- `cargo clippy --locked -p boardstudio-web-case --target wasm32-unknown-unknown
--all-targets -- -D warnings` and the same for `boardstudio-web-ui-shared`: clean.
- `scripts/check-wasm-tests.py`, `scripts/check-doc-links.py` (571 files), rustfmt
and `git diff --check`: clean. No native test was renamed (no baseline change).
- Packaged app: `build-web.py --providers-only --fast`, `--fast --skip-providers`,
then the page Runtime journey: PASS (durable save, exact Undo/Redo, four captures).
- `scripts/check.py quick`: all steps pass except `tooling`, which fails identically
without this lane's changes (the `test-check.py` script registry lists 19 scripts
as unregistered; this lane adds/removes no script files — pre-existing staleness).
- Merged the current `feat/rust-ui-migration` tip (`e211e4a83`, 021 parts editors)
into the lane with zero conflicts; re-ran the case suite (94/94) and the browser
script (13/13) on the merged tip.

Screenshots: `.evidence/018/browser-merged/settings018-{original,shadcn}-{light,dark}.png`
and `settings018-narrow.png` (inspected: filled controls, disclosure bars, narrow
targets), plus the page journey's four `page-*.png` captures.

Decision notes (no source-document updates needed): the `BoardDisclosure` `open`
prop is initial-only in practice (its reactive edge is unreachable — the Add actions
live inside the disclosure), so `Collapsible default_open` is faithful; the MountRow
fieldset's native border predates this ticket and stays (not a control); findings
lists take the mapping's `Item` + `Badge` row (list semantics give way to the named
mapping, as 019 will for collision lists). The `InputGroupTextarea` extension
fulfills the mapping's named `Textarea` target with additive props only.

Limits: physical coarse-pointer behavior unmeasured (same agent-browser limitation
as 017; narrow 44px metrics pass with explicit coarse classes). Screen-reader,
human 026, first paint, global full/browser and the merge itself remain open.
Worktree, branch and evidence are preserved for the merger.
