# 02: Settlements report where an edit landed

Status: resolved
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
- Starting points (orientation only, at `9548275`):
  - `application/src/session.rs`: `Effect`, `settle`, `persist_completed`;
  - `web/crates/runtime/src/operation_outcomes.rs`;
  - `web/crates/runtime/src/runtime.rs`: the `Effect::Settled` arm in `run`.

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
cargo test -p boardstudio-application --locked
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py typecheck
```

Runtime is wasm-only, so the typecheck step is required to prove that its
`Effect::Settled` arm still compiles.

## Out of scope

- Queued-edit resolution (ticket 03) and the web edit ticket (ticket 04).
- Changing any controller to use landings.

## Outcome

Commits: d6f10a08d (claim), 12d533d15 (implementation), 60cc4939d (review fixes).

- `Landing { revision, token }` is public in `application`; `Effect::Settled` carries
  `landing: Option<Landing>`. `TerminalOutcome`, `OutcomeSlot` and every existing caller
  are unchanged; the ~25 `Effect::Settled` matches take `..`.
- Session captures the landing where it installs the accepted snapshot in
  `persist_completed`'s Committed branch — the single accepted-snapshot install — so Open,
  Edit commit, Undo, Redo, GestureCommit, ExportCommit, ReviewElectricalRemap and
  RecoverWithDocument all report it. A retried save re-reports the original operation with
  the same landing once the retry commits (`settle_landed` updates the settled set without
  suppressing the report; the earlier `PersistenceFailed` report carries no landing).
  Everything else settles with `landing: None`, including Close's `Completed`.
- Web: `OperationOutcomes` pairs the outcome and landing slots per operation, so
  `observe_with_landing` works whichever way round the two observation calls happen; plain
  `observe`/`OutcomeSlot` behave exactly as before. Runtime's `Effect::Settled` arm forwards
  the landing via `settle_with_landing`.
- Native tests (application/tests/durable_session.rs, real `CoreEngine`): open/edit/undo/redo
  each land at their installed snapshot with a fresh token; retry-after-abort settles the
  retry and the original with the same landing while the failure report has none;
  selection/navigation/camera/job-cancellation complete with no landing; a rejected retry
  and a close carry no landing. Preview, generation and export-start completions settle
  through the same plain `settle` path (no landing by construction; only
  `persist_completed`'s Committed branch attaches one).
- Known behavioural delta: the retried original's re-report passes Runtime's Settled arm a
  second time, re-running its generic "Saved locally." status text and `changed()` for the
  original operation — benign (identical text; the edit did land), noted for ticket 04's
  edit-ticket work.

Checks (all pass): `cargo test -p boardstudio-application --locked` — 22 passed (4 new);
`cargo test -p boardstudio-web-runtime --locked` — 92 passed (1 new observer test);
`python3 scripts/check.py typecheck`; `wasm-pack test --headless --chrome
web/crates/runtime --locked --lib` — 33 passed. Touched files are rustfmt-clean.
