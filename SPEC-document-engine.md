# Spec: document-engine milestone contracts

Status: Phase 1 authorized in advance; technical validation required, 2026-10-01.
Module id: `document-engine` in [CAPABILITY-MAP.md](CAPABILITY-MAP.md).
Reference: `96dd51d3e790c28f5554a8c9888a147c8814e2a7` plus the approved
[ADR 0003](docs/adr/0003-rust-application-ownership.md). No engine rewrite is
proposed by this specification.

## Assumptions and objective

The existing Rust engine remains the authority for supported document semantics,
validation, geometry, revisions and history. M1 consumes it for copied layout
and case fixtures, through a long-lived core worker. Storage, selection, camera,
job epochs and accepted/durable acknowledgment belong to other owners. Existing
keymap, electrical and generator-definition data must survive supported loads
and saves even when the trial does not expose their editors.

Provide the stable provider contract used by the session/host specifications.
Reuse existing Rust behavior and regression oracles rather than rebuilding
domain functionality for Dioxus. Preserve the fixed-outline and linked-refinement
ADRs, tolerances, readiness and error meaning.

## Provider contracts

| Surface | Required meaning |
| --- | --- |
| `CoreEngine::new` / `handle(CoreRequest) -> CoreReply` | One engine per core-worker lifetime; ordered typed requests, preserving request IDs and public reply variants in [model.rs](core/src/model.rs). Worker envelopes are external metadata, not new engine/document fields. |
| Open / Snapshot | Validate supported `boardstudio/v2` input. Open resets engine history as today; session must not use it for ordinary save retry. Preserve supported definitions, unknown-field behavior and assets according to the existing archive/storage contracts; never promise arbitrary fields absent from those contracts. |
| Edit preview | Validate base revision and operation. Resolve display geometry without changing committed document/revision, Undo/Redo or storage. A preview reply contains a scene, not a newly accepted durable document. |
| Edit commit | Validate base revision, target/operation semantics and readiness constraints; advance revision and history according to existing public tests. Every successful commit is a history operation; transaction ID is not an idempotency key or commit coalescer. |
| Undo / Redo | Preserve existing revision/history behavior and domain outputs. Session supplies save-before-publication ordering; engine does not acknowledge browser durability. |
| Case preparation and artifact/archive services | Preserve public typed/JSON entrypoints and existing encodings. Core owns preparation, validation and supported format semantics; no dependency on renderer, browser storage or UI. |
| Semantic scenes | Existing `SceneDelta` supplies document-derived transforms, contours, findings and readiness. The engine owns their meaning. External session/job identity is attached by the application boundary, preserving provider payloads. |

Core errors remain errors, including stale revision, invalid input and domain
findings/readiness. An adapter must reject an unexpected reply variant rather
than invent success. No mutation may be replayed automatically after an unknown
worker outcome. Durable retry belongs to [editor-session](SPEC-editor-session.md).

## Rationale, tradeoffs and compatibility

The observed problem is orchestration outside the engine, not absence of Rust
domain authority. Reusing typed `handle` inside its worker avoids a JS JSON
crossing within that worker and retains public testability. Calling the existing
JSON export is a compatible fallback if a packaging probe demonstrates a need;
it adds encoding/copy costs that must be measured. Replacing engine/history
internals or implementing persist-first transactions would widen M1 and is not
selected. New session recovery changes acceptance timing, not engine semantics.

Scene contract metadata is additive at the application boundary. File format,
IDs, dimensions, tolerances, constraints and supported script-expression
semantics are preserved. A Rust-owned domain expression interpreter does not
prove that remaining executable JS generators have been replaced.

## Structure and code style

Retain `core/src/lib.rs`, `core/src/model.rs`, domain modules and `core/tests/`;
retain `contracts/rust/` as the existing shared leaf. Do not move browser or
session state into these packages. Existing public behavior tests are the style
reference; this excerpt from [core tests](core/tests/core.rs) uses exhaustive
typed matching:

```rust
fn preview(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}
```

Test-helper panic is not a production error policy. Rust naming/formatting and
existing serde field names remain the conventions.

## Commands and testing strategy

These are existing commands, not results of this documentation run:

```sh
cargo test --manifest-path core/Cargo.toml --locked
cargo fmt --manifest-path core/Cargo.toml --all -- --check
cargo clippy --manifest-path core/Cargo.toml --locked --all-targets -- -D warnings
pnpm run build:core
pnpm run check:contracts
pnpm run test:contracts
```

Use real-engine native request tests for preview/history/error behavior and
existing archive/domain suites for data fidelity. Worker packaging and browser
ordering require P1 and paired M1 tests. Preserve inherited failing/unavailable
gates from [the assessment](docs/dioxus-context-baseline.md#baseline-validation);
the command inventory is not a passing gate. Follow [CONSTRAINTS.md](CONSTRAINTS.md)
for affected completion/integration checks and frozen budgets.

## Success criteria

- E1: The copied fixtures open through the public provider and retain supported
  fields/assets after archive exchange; invalid/incompatible input is rejected.
- E2: Previews change display only; one final commit adds one history operation;
  Undo/Redo and stale-base rejection match existing public tests.
- E3: Typed worker and existing JSON boundaries produce equivalent documents,
  scenes and domain errors for the characterized milestone inputs.
- E4: Readiness, fixed/linked outlines and prepared-case inputs remain equivalent;
  UI/session/browser types cannot become engine dependencies.

## Boundaries and open validation

Always preserve provider contracts and characterize any future internal change.
Ask before widening existing private APIs, changing the durable format or an
approved behavior oracle. Never add a second geometry/history authority, label
transaction IDs safe replay keys, or lower checks to make a trial pass.

P1 still needs a real typed worker/build proof. Saved-document corpus coverage
is incomplete; M1 fixtures cannot establish every supported document's parity.
This provider specification enables consumers to be specified; it does not
authorize engine implementation work or close the trial.
