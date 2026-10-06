# 04: Edit ticket module for presentation

Status: ready-for-agent
Type: build
Blocked by: 02, 03
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Add one web module that every panel uses to submit a pending edit and read what
happened to it. A controller begins a ticket with its owner and a resolver (see
ticket 03), keeps the ticket, and on each render asks it for a settlement:

- **Pending**: show the draft value.
- **Landed { revision }**: the edit is part of the accepted document at that revision.
- **Failed { message }**: show the accepted value again plus `message` inline.
- **Retired**: the owner is gone, or the session moved on. Show nothing and drop
  the ticket.

The ticket owns everything controllers currently repeat:

- the operation ID and transaction label;
- observing the outcome and landing;
- mapping every terminal outcome to the four settlements;
- the standard failure wording, including the save-recovery wording;
- the owner-liveness gate.

Controllers supply only their owner, a liveness answer and an optional feature
phrase for messages.

## Background you need

- Read the spec's "Implementation Decisions" and "Testing Decisions", ADR-0005,
  and ticket 03's shape sketch.
- Today about 35 controllers each keep a `Pending*` struct and decide "landed" with
  heuristics, e.g. "token changed and revision increased and durability is Saved at
  that revision". With landings (ticket 02) that guesswork becomes an exact check.
  Find examples with `grep -rn "struct Pending" web/src`. The Matrix Setup
  controller's `settle_pending` is a representative one.
- `web/crates/runtime/src/operation_outcomes.rs` compiles natively (see `web/crates/runtime/src/lib.rs`: the runtime crate has no
  Dioxus) and has native `#[test]`s. Put the new module beside it, with
  the same `cfg`, so its tests run under native `cargo test`. Presentation itself is
  wasm-only.
- Standard wording to centralise. Collect the current variants first with
  `grep -rn "did not save\|Retry after recovery" web/src`, then pick one phrasing per
  outcome and keep feature-specific nouns as a parameter.

## Approach (test-first)

1. Define a small port the ticket needs: allocate an operation ID, observe an
   operation (outcome plus landing), and submit an event. Implement it for Runtime
   (wasm). Implement a native test driver that owns a real `Session`, a real
   `CoreEngine` and an in-memory save list. Its submit should drive effects to
   completion synchronously, with an option to hold Core replies or saves so tests
   can observe Pending.
2. Write a table-driven native test suite through the driver:
   - each terminal outcome maps to the right settlement;
   - Pending while a reply or save is held;
   - Landed carries the landing revision;
   - Unchanged lands at the current revision;
   - Retire becomes Failed with the resolver's reason, or Retired when the owner is
     gone (decide and document which; the spec says the field reverts with an
     explanation, so prefer Failed);
   - owner liveness false gives Retired whatever the outcome;
   - a session reopen gives Retired.
3. Implement the ticket and the wording helpers to make the suite pass.
4. Document on the module how controllers use it, with a five-line example in the
   module docs. Lesser models will copy it.

## Acceptance criteria

- [ ] The module compiles natively and in wasm. Its tests run under `cargo test` for the web crate.
- [ ] One mapping from `TerminalOutcome` plus landing to the four settlements, covered by native tests through a real Session and `CoreEngine`.
- [ ] Standard failure messages live in this module only. Each outcome has one phrasing, with an optional feature noun.
- [ ] A ticket answers `is_pending()`, so one-shot actions can disable their control while pending ([ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-field-edits-and-one-shot-actions-2026-10-06)).
- [ ] No controller is migrated in this ticket. The module is ready for ticket 06.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
cargo test -p boardstudio-application --locked
python3 scripts/check.py typecheck
python3 scripts/check-wasm-tests.py
```

## Out of scope

- Draft field widgets and Enter/blur handling (the investigation warns against a
  generic numeric-edit framework).
- Migrating any controller.

## Pitfalls

- Don't put presentation types (signals, components) in this module. It must stay
  native-compilable.
- Observe before submitting, so a synchronous settlement can't be missed. The
  existing outcome observer documents this.
