# Migration acceptance requirements

This is the retained quality contract, disclosed when preparing parent acceptance
or cutover. [CONSTRAINTS.md](../../CONSTRAINTS.md) owns execution timing and authority.
The active canonical task rows retain their individual criteria and final dependency joins. The user removed the unfinished VIK-only parent from the first-release roadmap on 2026-10-03; its prior requirements remain in Git history.
A requirement here applies to the affected workflow; global performance, corpus,
retirement qualification belongs to its milestone, not author dispatch. Accessibility qualification is excluded from the frontend migration by the 2026-10-05 user instruction.
User-authorized necessary API/design changes follow the compatibility rules below.

## Task categories and characterization

Every task identifies one category, affected capabilities/contracts, linked
decision or defect, reference revision/fixtures, expected result and required
checks before implementation. Split mixed work into separately reviewable
tasks; a refactor must not conceal a behavior change or defect correction.

| Category | Required evidence and acceptance oracle |
| --- | --- |
| Behavior-preserving refactor | Characterize affected observable behavior before replacement. Use the same inputs/actions and accepted outputs through reference and replacement, including errors, persistence and history. Internal Rust structure and transport may change while agreed contracts remain equivalent. |
| Intentional behavior change | Link the explicit approved decision, before/after behavior, compatibility strategy and changed acceptance tests. Preserve all unaffected contracts; an intentional visual change also needs comparison evidence. |
| Defect correction | Link the defect and approved correct behavior. Confirm a regression test fails for the expected defect before the fix and passes afterward. Test the correction and unaffected contracts; do not require permanent compatibility with the known bug. |

Characterization records current behavior without declaring every observed
quirk correct. Classify suspected defects and resolve their intended behavior
before replacing it; do not silently bless or silently change them. Transfer
equivalent regression coverage when retiring implementation-specific tests.
Changing a test's oracle for an approved correction is distinct from removing
assertions or lowering a threshold to hide a failure.

## Architecture

- **No big-bang rewrite.** Migrate incremental vertical slices: each delivers a
  bounded user workflow through UI, state, Rust services and persistence
  or export where applicable. Keep unaffected workflows working throughout.
- Existing working Rust geometry/core capabilities and agreed contracts are
  authoritative. Reuse them; their current crate layout, types, caches and
  internal structure are not sacred and may be refactored or rearchitected.
  Characterize and preserve validated capabilities and contracts through that
  work. Never port working Rust functionality back into TypeScript.
- Domain rules, validation, geometry, history and serialization have one
  authoritative implementation. Do not duplicate domain logic between
  TypeScript and Rust. Retained TypeScript presentation and transport must call
  the Rust authority rather than reimplementing it. The destination is maintained
  application logic in Rust; generated bindings and development/test tooling
  are distinct from application logic. Any retained executable JavaScript,
  including generator bodies or offline policy, needs an explicit boundary and
  an approved disposition rather than being relabeled as generated glue.
- Prefer Rust ownership of application state where practical. Give each state
  value and lifecycle one owner; document any retained JavaScript ownership,
  why it is needed and how it will be retired. Presentation snapshots are not
  independent writable domain stores.
- Minimize JS/WASM crossings, serialization, copying and redundant module
  initialization. Prefer existing typed Rust calls within a Rust module and
  batch boundary traffic where practical. Measure hot-path crossing frequency
  and transferred bytes rather than claiming that a Rust UI removes them.
- Browser-specific JavaScript boundaries must be explicit and documented in
  the slice design and [architecture documentation](../../docs/architecture.md):
  purpose, caller/callee, data contract, state/buffer ownership, error and
  cancellation behavior, cleanup, performance cost and retirement condition.
  Include workers, storage, canvas/renderer integration, file delivery, service
  workers and retained executable generators where relevant. Distinguish
  generated browser bindings from maintained application logic.
- Internal module boundaries may be redesigned with documented ownership and
  dependency direction; preserve agreed public/API contracts. Update boundary
  checks to assert an approved replacement architecture, with negative tests,
  rather than deleting checks or exempting a violating implementation. Changing
  externally consumed public APIs when necessary is covered by the user
  authorization in CONSTRAINTS.md. Use the narrowest sufficient visibility, record consumers
  and review the coherent replacement in the integrated candidate.
- Keep expensive core/CAD work off the UI thread; an async function alone is
  not evidence of background execution. Preserve cancellation, caller
  settlement, stale-result rejection and worker/renderer disposal.

## Compatibility and behavioral parity

- Refactors preserve observable behavior; intentional changes and defect
  corrections follow their approved acceptance oracles in this document. Compare the
  existing React workflow and its Dioxus replacement with the same fixtures,
  actions and expected results, including loading, empty, error and recovery
  states. A framework trial proves only the workflows it actually exercises.
- Existing project/file formats and saved BoardStudio documents must remain
  compatible. Test representative existing documents, imports, archives,
  assets, save/reload and exports. Preserve supported extension/unknown fields
  where the current implementation preserves them. Loading without a faithful
  round trip or preserving asset references is insufficient.
- Preserve accepted file-format, geometry, transaction, history and export
  contracts unless their change is explicitly approved. This includes geometry
  units/tolerances/topology and manufacturing readiness; preview versus commit,
  revision/stale-command rules and Undo grouping; and captured export snapshots,
  archive paths, asset hashes and KiCad/STEP/firmware outputs. Sources include
  [architecture](../../docs/architecture.md), [fixed edited outlines](../../docs/adr/0001-fixed-edited-outlines.md),
  [linked outline refinements](../../docs/adr/0002-linked-outline-refinements.md),
  [outline decisions](../../docs/design/board-outline-plan.md) and
  [keycaps/keymap contracts](../../docs/keycaps-and-keymap.md).
- Moving session/persistence coordination into Rust must preserve the accepted
  durable-save ordering and failure/recovery behavior, without assuming the
  current coordinator or cache implementation must survive. Engine commit and
  browser durable save are distinct operations today; changing their observable
  contract requires a decision and tests, not just moving a React hook.
- APIs and serialization formats must not change accidentally. Preserve field
  names, defaults, null/absent semantics, enum representations, IDs, revisions,
  units, precision and error behavior. Rust owns shared contracts; regenerate
  TypeScript bindings from them and check freshness instead of hand-editing
  generated types. Intentional externally consumed API, wire-contract or file-format
  changes necessary to this migration are authorized; document the decision,
  compatibility strategy and tests before acceptance. Internal Rust interface
  adjustments follow the same candidate-level review authority.
- Preserve keyboard and mouse semantics: shortcuts and modifiers, focus,
  selection, snapping, hit testing, drag thresholds and pointer capture,
  pan/zoom, preview/commit/cancel, Escape, Undo/redo and final pointer samples.
  Preserve transaction grouping, ordering and project/board-switch behavior;
  stale work must never update a different document or session.
- Preserve saved-project discovery, storage keys/schema, atomic writes,
  recovery, browser/offline behavior and deployment paths. Keep trial writes
  isolated as specified in [PLAN.md](../../PLAN.md); do not overwrite production
  projects or silently fork their persistence format.
- The assessed baseline accepts `boardstudio/v2` and rejects v1; that fact does
  not establish the full corpus of saved documents that must survive. Inventory
  actual supported saved documents before persistence cutover. Do not invent
  an importer or waive compatibility based on historical cleanup prose.

## UI

- Preserve current visual behavior unless an intentional change is documented
  with its reason and comparison evidence. Cover layout, typography, colors,
  spacing, themes, canvas geometry and loading/error feedback.
- Preserve ordinary keyboard operation, shortcuts, visible focus and focus
  restoration as functional UI behavior. Accessibility/axe/semantic-only audits,
  accessibility-only repairs and screen-reader/assistive-technology qualification
  are excluded from this frontend migration by user instruction on 2026-10-05.
  Existing controls and semantics remain; no excluded check is claimed as passed.
- Responsive behavior must not regress. Compare current desktop and compact
  layouts, supported zoom/DPR and overflow behavior, including long content and
  dialogs. Keep required actions reachable at existing supported sizes.
- Reuse the existing styles, tokens and interaction conventions. Do not
  introduce a new design system during migration.

## Rust and Dioxus

- Target **Dioxus 0.7.x**. Record the selected crate and compatible CLI patch
  versions and commit lockfiles for reproducible builds. A newer major/minor
  version requires a separate decision.
- Verify framework-specific decisions against current official documentation
  for the selected version. Use version-matched Context7, then official tagged
  source or installed source when exact documentation is unavailable. Record
  the source, version and verification date in the slice design; unversioned
  examples and React experience are not sufficient evidence.
- Do not assume React concepts map directly to Dioxus. Prefer idiomatic
  Signals, resources and components, with deliberate reactive dependencies,
  async lifetimes, cancellation and cleanup. See the official
  [0.7 Signals guide](https://dioxuslabs.com/learn/0.7/essentials/basics/signals/)
  and [versioned resource documentation](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/hooks/docs/use_resource.md).
- Avoid unnecessary cloning or shared mutable state to work around ownership.
  Distinguish copying a Signal handle from cloning its underlying document or
  geometry. Borrow or transfer data where appropriate; justify large copies
  and shared mutability through actual lifetime/concurrency requirements and
  performance evidence. Preserve validated ownership, mutation isolation,
  bounded resource use and lifecycle guarantees while allowing their internal
  implementation and caller to change.
- Verify browser/wasm-bindgen feature and version compatibility with the
  existing engines. Use the official [0.7 tooling guide](https://dioxuslabs.com/learn/0.7/guides/tools/)
  and the selected CLI's help/source for build configuration. References were
  checked on 2026-09-30; recheck when selecting or changing the toolchain.

## Required qualification

For parent acceptance, retain affected formatting, strict Clippy and crate/package
correctness checks, matching WASM/production builds, contract/boundary checks and
paired production behavior. Include affected performance, persistence/export,
in-scope desktop interaction and visual evidence; state omissions and unmet criteria.
Reuse sufficient unchanged evidence. Failed, missing or blocked applicable checks
prevent acceptance; a successful compiler/package does not establish behavior.

Before cutover or React deletion, retain full integration checks (`pnpm run check`,
`pnpm run test:e2e:dev`, supported standalone Rust/native CAD checks, Dioxus runtime
and parity, performance/CAD/live/soak and applicable visual/keyboard checks).
Existing CI stays enabled. Commands and feature support are read from source;
the superseded check catalogue is historical, not a current tool inventory.

Keep raw failed attempts, command/target/environment, source/fixture/artifact hashes
and limitations. Review semantic weakening as well as staged, unstaged and untracked
changes. A missing automated checker is neither a pass nor an implementation lock.

## Performance

Migration must not materially regress startup time, interaction latency,
memory use or bundle/WASM size without documenting the tradeoff. Preserve the
existing budgets and comparison rules in [the performance baseline](../../docs/performance-baseline.md)
and [generation performance evidence](../../docs/generation-performance.md).
Historical measurements are provenance, not a fresh baseline for this branch.

| Metric | Measurement and comparison |
| --- | --- |
| Startup | Measure cold navigation through usable UI and required worker/WASM readiness in a real browser; distinguish cached/offline loads and lazy CAD/3D startup. |
| Interaction latency | Use `pnpm run test:perf` with the same workflows and user-visible timing endpoints in both frontends; preserve existing sampling and threshold rules. |
| Memory | Compare post-warmup retained/peak JS and WASM/worker memory through equivalent editing, project switching and teardown. Use existing soak scenarios; report process RSS separately when measured. |
| Bundle/WASM size | Record raw and deployed compressed bytes from production builds for initial and lazy JS/WASM/assets, plus their combined totals. Include coexisting React/Dioxus code and duplicate modules. |

### Budgets supported by existing measurements

Propose retaining the following executable budgets, not choosing new numbers.
Their fixtures, endpoints and sampling are part of the contract; a faster
callback is not evidence of a faster painted editor. Sources of truth remain
the existing data and assertions linked here rather than copies of every budget.

| Workload | Existing budget / comparison rule | Executable source |
| --- | --- | --- |
| Workbench 100-single / 100-row / 200-single / 200-row | Worker p95 limits: 5.17 / 4.40 / 7.59 / 7.70 ms; painted p95: 36.96 / 37.84 / 46.97 / 53.57 ms. Derived from the measured baseline multiplied by the existing 1.1 allowance, using median of five session p95s with 100 samples each. | [Baseline JSON](../../app/performance-baseline.json), [runner and exact comparator](../../app/scripts/run-performance.mjs), [rounding regression tests](../../app/scripts/performance-budget.test.mjs). Preserve the comparator's nanosecond normalization, not rounded console limits. |
| Pointer-to-painted movement | p95 <=33 / 50 / 100 ms for 30 / 100 / 200 keys; preserve visible-transform assertions, warmup and measured samples. | [Pointer performance](../../app/e2e/pointer-performance.spec.ts). Frame-gap diagnostics are not an additional asserted release budget. |
| Outlines and matrices | Outline p95 <100 / 200 ms for 100 / 200 keys. Matrix/row/column painted p95 <=100 / 200 ms for 100 / 200 keys, with 100 measured samples. | [Outline](../../app/e2e/outline-performance.spec.ts), [matrix](../../app/e2e/matrix-performance.spec.ts). Thirty-key matrix timing is measured without an asserted latency cap. |
| CAD completion, paint and sampled RSS | Preserve all 30 scenario-specific absolute limits and eligibility/sampling/fixture checks. For example, cached gasket paint is <=38 ms; the historical 31.4 ms result is evidence, not a new threshold. | [Frozen budgets](../../cad/bench/budgets.json), [comparator](../../cad/bench/compare.mjs), [protocol](../../cad/bench/README.md). Do not apply one RSS value to every scenario or rewrite these limits from a candidate run. |
| Live release-to-painted edits | The existing non-regression limit is reference p95 + max(4 ms, 10% of reference p95) for numeric, Undo, gasket and mount actions. Comparable evidence requires at least five sessions and the existing CPU/browser/GPU/viewport/sample provenance. | [Live comparison](../../app/scripts/compare-live-performance.mjs), [comparison tests](../../cad/test/live-performance-comparison.test.mjs), [five-session runner](../../app/scripts/run-live-performance.mjs). The CLI also requires an optimization improvement; see below. |
| Full-UI retained memory | Existing 256-cycle soak compares medians for cycles 129–192 and 193–256, separately by project. Growth allowance: max(2 MiB, 10%) for JS heap; max(2 MiB, 5%) for summed WASM capacity. Pending callers must settle. | [Full-workbench soak](../../app/e2e/live-soak-performance.spec.ts). This does not bound cold/peak memory or process RSS; the opt-in environment variable must be set so the test actually runs. |

The separate **200 ms p95 exact-generation target remains unmet for gaskets**.
It is an existing improvement target, not proof of a green baseline and not a
newly activated migration gate. Existing optimization acceptance criteria
(including targeted improvement/copy reduction) remain intact for optimization
tasks; a migration-only task must preserve non-regression budgets without
claiming an optimization goal was met. The live CLI exits nonzero if the existing
improvement criterion fails, even when its `regressionsPassed` result is true;
report both rather than calling that command a pass. A dedicated migration
invocation asserting the unchanged non-regression result is proposed work,
not an installed gate or permission to weaken the optimization comparator.

### Performance/resource invocations

Use a fresh writable `MIGRATION_RESULTS` directory and retained eligible
`MIGRATION_CAD_REFERENCE` / `MIGRATION_LIVE_REFERENCE` paths. Never overwrite
reference data. Build fresh production artifacts first and preserve their hashes.

| Check | Existing invocation and verdict |
| --- | --- |
| Workbench, pointer, outline, matrix and live scenario assertions | `BOARDSTUDIO_PERF_REPORT="$MIGRATION_RESULTS/workbench.json" pnpm run test:perf` (or `pnpm --dir app test:perf` with the same already-verified production build). Existing runner enforces the workbench host/reference and assertion budgets. |
| Full CAD scenario budgets | `pnpm run bench:cad -- --output "$MIGRATION_RESULTS/cad"`, then `node cad/bench/compare.mjs "$MIGRATION_RESULTS/cad" "$MIGRATION_CAD_REFERENCE"`. Only an eligible complete run with comparator success passes; one-session smoke or host-instrumented diagnostics do not. |
| Five hardware-accelerated live sessions | `node app/scripts/run-live-performance.mjs "$MIGRATION_RESULTS/live" "$MIGRATION_LIVE_REFERENCE"`. Saves raw sessions/summary and applies the existing regression and improvement checks; disclose either failure and the separate exact target. |
| Full-UI memory/settlement soak | `BOARDSTUDIO_UI_SOAK_RESULT="$MIGRATION_RESULTS/ui-soak.json" BOARDSTUDIO_CHROMIUM=/usr/bin/chromium pnpm --dir app exec playwright test --config playwright.performance.config.ts e2e/live-soak-performance.spec.ts --workers=1 --headed`. Existing assertions decide pass; a skipped run is unavailable evidence. |

Run timing workloads serially on a comparable host/browser/GPU with unchanged
fixture and sample coverage. Keep busy-host, synthetic CPU, software-rendered,
real-device and hardware-accelerated evidence distinct. The CAD comparator
currently requires matching core/renderer WASM identities between reference and
candidate. If a Rust refactor or new packaging changes those identities, record
the comparison as ineligible and propose a validated comparison protocol;
do not falsify hashes or remove the guard to obtain acceptance. Reuse unchanged
engine artifacts across UI variants where that is a valid comparison.

### Unresolved measurements

| Metric | Evidence / measurement still needed before proposing a numeric gate |
| --- | --- |
| Cold/cached/offline startup and lazy CAD/3D readiness | Historical navigation and renderer-init diagnostics use older builds and differing endpoints. No accepted migration startup ceiling or paired current-build variance exists. Define usable-editor/engine readiness endpoints and capture comparable production React and Dioxus loads. |
| Initial/lazy bundle and WASM size | Historical byte/gzip measurements are not a current complete deployable bundle baseline. Inventory raw and deployed compressed production assets, shared versus duplicate modules and coexistence totals. No defensible migration size budget is established yet. |
| Peak/retained whole-application resources | Existing soak and scenario RSS budgets cover specified workloads only. Cold peaks, teardown/project-switch retention and complete worker/process memory need comparable measurement and an accepted sampling protocol. |
| JS/WASM crossings and large copies | Existing benchmark transfer/stage diagnostics do not set a crossing-frequency/byte budget. Instrument affected hot paths and define an evidence-based limit; do not equate a Rust component with elimination of boundary traffic. |
| Coverage | No measured, accepted line/branch coverage threshold is established. Preserve meaningful behavioral regressions and measure coverage before proposing a percentage or ratchet. No invented default is adopted. |
| Reference availability | The assessment records five missing historical raw-result links. Retain that limitation; locate eligible protected evidence or explicitly agree a fresh reference protocol before comparative acceptance. Do not reconstruct passing baseline numbers from prose. |

Before implementation, capture the relevant React baseline and define any
missing measurement command, workload and acceptable variance in the slice
design. Propose and review the measured budget before activation. Compare builds
on equivalent hardware/browser, fixtures, viewport, cache state and production
settings. For new metrics, hold the measured line;
a reproducible worsening outside the predeclared measurement variance is
material. Do not invent a passing threshold after seeing candidate results.

Every material regression needs before/after evidence, user impact, rationale,
scope, owner and mitigation/expiry in the slice record. A performance tradeoff
does not silently raise an existing frozen budget; changing a blocking budget
requires the explicit decision described under Enforcement. Keep real-device,
hardware-accelerated and synthetic/headless evidence distinct.

## Acceptance and retirement

Preserve every criterion and acceptance join in the canonical graph. Paired journeys
cover relevant contextual panes, menus, edits, Undo/Redo and save/reopen. Keep supported
source documents and the working reference available throughout qualification.
React removal requires the replacement's parity and other applicable gates to pass;
transfer equivalent regression coverage, then check remaining routes/imports.
Every temporary adapter/parallel implementation has an owner, removal task and
explicit retirement criterion with a named follow-up slice or deadline. Record these
in its existing spec, including purpose/callers/boundary. The completed migration
cannot retain duplicate domain authority or unowned bridges.

## Floor and exceptions

- Do not weaken tests, performance/coverage thresholds, dependency/architecture
  boundary checks or assertions to make a migration task pass. Preserve input
  rejection, ownership, stale-result and manufacturing/export safeguards.
- No new checker suppressions or coverage exclusions to hide failures,
  including `@ts-ignore`, `eslint-disable`, Rust lint allowances, skipped tests
  or removed assertions. Any justified exception must be explicitly reviewed.
- No unimplemented runtime stubs (`todo!()`, `unimplemented!()`, throwing
  "Not implemented") or empty error handlers that turn failure into success.
- No skipped/deleted required tests without a recorded reason and preserved
  equivalent behavioral coverage. Existing ignored diagnostics are not
  permission to ignore new migration regressions.
- No secrets in source. Review only the migration diff and redact any secret
  scanner output; never print matched values.
- Review the diff against its starting state for lowered thresholds, weakened
  tests, suppressions, stubs and undisclosed exceptions. Fix the implementation
  instead of editing the bar to pass it.

The floor remains a review requirement. A scoped exception records its rule, reason,
evidence, owner, explicit user decision, expiry and remediation. Necessary API/design
changes and the execution sequence in CONSTRAINTS.md are already authorized; they do
not imply that unmet behavioral criteria passed.
