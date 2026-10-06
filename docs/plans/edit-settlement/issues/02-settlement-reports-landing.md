# 02: Settlements report where an edit landed

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

When Session settles an operation as completed, it also reports the landing: the
accepted document revision and snapshot token that operation produced. Callers can
then tell exactly whether and where their edit landed, without comparing tokens,
revisions and save states themselves. Web's operation-outcome observer keeps the
landing alongside the outcome so later tickets can read it. Existing callers of
`TerminalOutcome` keep compiling unchanged.

## Background you need

- Read the [spec](../spec.md) ("Settlement reports where an edit landed").
- Session (`application` crate) is synchronous: `submit(Event)` and
  `complete(Completion)` return effects. `Effect::Settled { operation_id, outcome }`
  is the only terminal report.
- For document-changing operations, `TerminalOutcome::Completed` is only produced
  after the save commits. At that moment Session installs a new
  `AcceptedSnapshot { token, document.revision, … }`. That snapshot is the landing.
- Selection, navigation, camera, previews and jobs also settle as `Completed`;
  they have no landing (`None`).
- Web keeps outcomes in `OperationOutcomes` (`observe(operation) -> OutcomeSlot`,
  `settle(operation, outcome)`). About 35 controllers read `OutcomeSlot` as
  `Rc<RefCell<Option<TerminalOutcome>>>`, so don't change that type.
- `Effect::Settled` is matched in about 25 places across 6 files (Session, its
  tests, Runtime, operation outcomes, firmware projection, one web integration
  test). `TerminalOutcome::Completed` is referenced 137 times, so don't change that
  variant.
- Starting points (orientation only, at `a352f95`):
  - `application/src/session.rs`: `Effect`, `settle`, `persist_completed`;
  - `web/src/operation_outcomes.rs`;
  - `web/src/runtime.rs`: the `Effect::Settled` arm in `run`.

## Approach

1. Add a small public `Landing { revision, token }` type to `application`. Add
   `landing: Option<Landing>` to `Effect::Settled`.
2. Fill it in wherever Session settles an operation as `Completed` right after
   installing an accepted snapshot: Open, Edit commit, Undo, Redo, GestureCommit,
   ExportCommit, ReviewElectricalRemap and RetrySave (the retry operation and the
   original). Use `None` everywhere else.
3. Update every `Effect::Settled` match. Most can use `..`.
4. In web, extend `OperationOutcomes` so it retains the landing for an operation.
   Add a second observation entry point that returns both. Keep `observe` and
   `OutcomeSlot` behaving exactly as today for existing callers.
5. Write the tests below first (TDD), then implement.

## Acceptance criteria

- [ ] Every document-changing completion carries a landing whose revision and token equal the accepted snapshot installed for it.
- [ ] Retrying a failed save settles both the retry and the original operation with the same landing.
- [ ] Non-document completions (selection, camera, navigation, preview, generation, export start) carry no landing.
- [ ] Rejected, superseded, cancelled, failed and closed outcomes carry no landing.
- [ ] Existing `OutcomeSlot` consumers compile and behave unchanged. A new observer can read the outcome plus its landing.
- [ ] Native tests in `application` cover each landing case above, using the real `CoreEngine` as in the existing durable-session tests.

## Verification

```sh
cargo test --manifest-path application/Cargo.toml --locked
cargo test --manifest-path web/Cargo.toml --locked --lib --bin boardstudio-web
python3 scripts/check.py typecheck
```

Runtime is wasm-only, so the typecheck step is required to prove that its
`Effect::Settled` arm still compiles.

## Out of scope

- Queued-edit resolution (ticket 03) and the web edit ticket (ticket 04).
- Changing any controller to use landings.
