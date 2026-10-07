# 01: `SetWiringMode` replaces the PCB wiring-mode replacement

Status: resolved
Type: build
Blocked by: [edit settlement 20](../../edit-settlement/issues/20-preview-only-direct-edit-event.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0006](../../../adr/0006-replace-document-for-import-and-recovery.md)

## What to build

The first typed Core edit, and the pattern later ones copy. Add
`EditOperation::SetWiringMode { board_id, mode }` to Core and move the PCB wiring
panel's mode change onto it, deleting its `ReplaceDocument` proposal.

Today `mode_resolver` (`web/crates/pcb/src/pcb_wiring/mode.rs:174`, line numbers at
`a3b41ae8a`, orientation only) clones the accepted document through `propose_mode`
(`web/crates/runtime/src/pcb_wiring_mode_operation.rs:114`) and submits the whole
document. Core reports every part and top-level outline feature as changed, reapplies
scripts and recomputes outlines without the cache, for one board-scoped enum.

## Behaviour (unchanged for the user)

- The board must exist. In Core, an unknown board is an edit error; the resolver
  retires first with today's reason ("The board was deleted.").
- A board with no electrical configuration counts as `Matrix`. Setting a different mode
  creates the configuration (`ElectricalBoardConfiguration { board_id, mode,
  ..Default::default() }`) in `document.hardware`, creating `hardware` if absent.
- Setting the mode the board already has resolves `Unchanged` in the resolver; Core
  applying it is a valid no-op that changes nothing.

## Core rules this ticket settles (the pattern)

- **Operation shape:** an intent named for the user's change, carrying only IDs and the
  value; Core owns defaulting and creation.
- **Changed IDs:** `apply` returns only what changed, here the board ID; not every part
  and outline feature.
- **Outline classification:** `SetWiringMode` is not in `affects_outline`
  (`core/src/lib.rs:995`), so no outline recomputation runs. Record the reasoning in the
  operation's doc comment.
- **Undo:** one history step; Undo restores the previous mode (and removes a
  configuration the edit created).

## Call sites to change

- `core/src/model.rs` (`EditOperation`), `core/src/lib.rs` (`apply`, `affects_outline`
  stays without it).
- `web/crates/pcb/src/pcb_wiring/mode.rs` `mode_resolver` submits `SetWiringMode`.
- `propose_mode` loses its last product caller; delete it and its tests, or keep it only
  if the wiring plan preview still needs it (check `queued_edit_tests.rs:227`).

## Acceptance criteria

- [x] Core tests: set Direct on a board without configuration creates it; set the
  current mode is a no-op; unknown board errors; changed IDs are only the board; the edit
  does not trigger outline recomputation; Undo restores the prior state.
- [x] Session/Core test: a mode change queued behind an unrelated edit (gated Core reply)
  keeps that edit, and Undo removes them in order.
- [x] The PCB wiring panel's existing mounted tests pass with the resolver submitting
  `SetWiringMode`; no `ReplaceDocument` remains in `mode_resolver`.
- [x] The resolver inventory row for `mode_resolver` is updated or removed.

## Verification

```sh
cargo test -p boardstudio_core --locked
cargo test -p boardstudio-application --locked
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py typecheck test
wasm-pack test --headless --chrome web/crates/pcb --locked --lib
python3 scripts/check-doc-links.py
```

## Out of scope

- Other wiring edits (part-net assignment, reviewed plan): later tickets.
- Enforcing ADR-0006.

## Outcome

Merged `76286df1a`, `efb150479`, `ac8f5594a` from
`typed-core-edits/01-set-wiring-mode`, rebased on `e6eeb05f7`. Core owns the typed
operation, existence checks, defaults and precise changed IDs. Missing configuration
counts as Matrix; creating Direct adds only the target board's electrical defaults.
Existing settings are preserved, same mode is an apply no-op, and an unknown board
is an error. The operation is excluded from `affects_outline`, with rationale in its
doc comment. The PCB resolver preserves its missing-board retirement and unchanged
case, submits the typed intent, and no longer calls `propose_mode`, which was removed
with its shallow tests. The resolver inventory is updated.

Post-rebase checks passed: Core 177 unit and 303 integration (3 ignored), Application
29, Runtime 106, WASM page typecheck, PCB browser 38 including gated unrelated-edit
queueing and ordered Undo, lint and doc links. Both reviews targeted the exact
implementation worktree; neither found TCE-01 issues. PCB browser compilation still
reports the existing unused import at `pcb_wiring.rs:1017`. The broader test script
passed workspace Rust tests but failed the known unchanged CAD volume baseline;
see [ES-20 evidence](../../edit-settlement/issues/20-preview-only-direct-edit-event.md#outcome).

Pattern for the next intent: IDs plus the user's value; Core owns mutation, defaults,
validation and no-op behavior; report only directly changed durable IDs and classify
outline effects from changed inputs. Resolver checks retain owner/target lifetime
and same-value `Unchanged`. Test Core rules, queued unrelated edits and Undo through
the Session/Core seam. Full-document Undo snapshot cost is unchanged.
