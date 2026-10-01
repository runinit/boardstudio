# BoardStudio migration constraints

Last reviewed: 2026-09-30.

Scope: incremental rearchitecture of BoardStudio toward a full-Rust application
with Dioxus 0.7.x, including application coordination, persistence, workers,
rendering, generators and export. This is not a mechanical frontend port.
These requirements govern plans, implementation, review, cutover and removal.
Read them before changing migration code.

**Proposal status:** the policy updates reflect the user's instructions;
the check tiers and additional enforcement below are proposed for review.
No new tooling, gate, CI job or budget has been activated by this update.
Existing requirements and executable gates remain in force. An unimplemented
check is an enforcement gap, not an enforced rule or a passing result.

Evidence: [context assessment and reusable decision index](docs/dioxus-context-baseline.md),
[recorded baseline](docs/dioxus-context-baseline-evidence.json),
[current contracts and ownership](docs/architecture.md) and the preserved
[Wayfinder map](.scratch/dioxus-browser-trial/map.md). Addy's installed
constraint-driven-development skill governs this revision. Link existing
decisions; do not duplicate them or reopen accepted decisions without concrete
contradictory evidence or an explicitly approved scope change.

## Enforcement

- Every migrated slice must satisfy this contract before acceptance. Failed,
  blocked or unperformed gates remain blocking; report them explicitly.
- Separate policy/review requirements, executable checks and CI enforcement.
  A command is enforced only for the assertions it actually executes. A build,
  dependency listing, screenshot capture or successful manual inspection does
  not prove every architecture, visual or compatibility rule.
- The enforcement gaps below must be closed before accepting the relevant
  slice. Present new gates and their measured budgets for review before
  installing or activating them. Trial-run approved checks against reference
  and candidate; preserve failures rather than changing baselines to pass.
- This file is the canonical quality bar. Scripts and CI must implement it;
  existing `pnpm run check` alone does not cover every migration requirement.
- Do not weaken requirements, checks, assertions or budgets to make work pass.
  Changes to the bar require an explicit user decision and recorded rationale.
  Preserve unrelated work and existing required reviews and approvals.

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
  the slice design and [architecture documentation](docs/architecture.md):
  purpose, caller/callee, data contract, state/buffer ownership, error and
  cancellation behavior, cleanup, performance cost and retirement condition.
  Include workers, storage, canvas/renderer integration, file delivery, service
  workers and retained executable generators where relevant. Distinguish
  generated browser bindings from maintained application logic.
- Internal module boundaries may be redesigned with documented ownership and
  dependency direction; preserve agreed public/API contracts. Update boundary
  checks to assert an approved replacement architecture, with negative tests,
  rather than deleting checks or exempting a violating implementation. Widening
  Rust member/API visibility still requires explicit user approval.
- Keep expensive core/CAD work off the UI thread; an async function alone is
  not evidence of background execution. Preserve cancellation, caller
  settlement, stale-result rejection and worker/renderer disposal.

## Compatibility and behavioral parity

- Refactors preserve observable behavior; intentional changes and defect
  corrections follow their approved acceptance oracles above. Compare the
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
  [architecture](docs/architecture.md), [fixed edited outlines](docs/adr/0001-fixed-edited-outlines.md),
  [linked outline refinements](docs/adr/0002-linked-outline-refinements.md),
  [outline decisions](docs/design/board-outline-plan.md) and
  [keycaps/keymap contracts](docs/keycaps-and-keymap.md).
- Moving session/persistence coordination into Rust must preserve the accepted
  durable-save ordering and failure/recovery behavior, without assuming the
  current coordinator or cache implementation must survive. Engine commit and
  browser durable save are distinct operations today; changing their observable
  contract requires a decision and tests, not just moving a React hook.
- APIs and serialization formats must not change accidentally. Preserve field
  names, defaults, null/absent semantics, enum representations, IDs, revisions,
  units, precision and error behavior. Rust owns shared contracts; regenerate
  TypeScript bindings from them and check freshness instead of hand-editing
  generated types. Any intentional API/format change needs a separate explicit
  decision, compatibility strategy and tests before use.
- Preserve keyboard and mouse semantics: shortcuts and modifiers, focus,
  selection, snapping, hit testing, drag thresholds and pointer capture,
  pan/zoom, preview/commit/cancel, Escape, Undo/redo and final pointer samples.
  Preserve transaction grouping, ordering and project/board-switch behavior;
  stale work must never update a different document or session.
- Preserve saved-project discovery, storage keys/schema, atomic writes,
  recovery, browser/offline behavior and deployment paths. Keep trial writes
  isolated as specified in [PLAN.md](PLAN.md); do not overwrite production
  projects or silently fork their persistence format.
- The assessed baseline accepts `boardstudio/v2` and rejects v1; that fact does
  not establish the full corpus of saved documents that must survive. Inventory
  actual supported saved documents before persistence cutover. Do not invent
  an importer or waive compatibility based on historical cleanup prose.

## UI

- Preserve current visual behavior unless an intentional change is documented
  with its reason and comparison evidence. Cover layout, typography, colors,
  spacing, themes, canvas geometry and loading/error feedback.
- Accessibility must not regress. Preserve semantic controls, accessible
  names, keyboard reachability, focus order/visibility/restoration and existing
  assistive-technology behavior. Require zero newly introduced axe violations
  on affected workflows, plus manual keyboard and relevant screen-reader
  checks; automation alone does not establish accessibility parity.
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

## Executable checks and their actual coverage

Commands run from the repository root unless specified. Set
`MIGRATION_MANIFEST` to each affected manifest: currently `core/Cargo.toml`,
`renderer/Cargo.toml`, `cad/wasm/Cargo.toml`, `contracts/rust/Cargo.toml`, and
eventually the Dioxus/app-coordinator manifests. There is no root Cargo workspace.
Use the crate's supported target/features and required CAD build environment.
Record missing tools, lockfiles or generated inputs as blocked checks; do not
use an unlocked build or cached package as proof of a fresh locked build.

The following checks exist today. "Available" describes an executable assertion,
not a pass on this revision. See the [baseline results](docs/dioxus-context-baseline.md#baseline-validation)
for failures and unavailable checks.

| Dimension | Existing command | Assertions / enforcement status |
| --- | --- | --- |
| Rust/native formatting | `cargo fmt --manifest-path "$MIGRATION_MANIFEST" --all -- --check` | No formatting differences. Available; not included in current root CI check. |
| Rust/native lint | `cargo clippy --manifest-path "$MIGRATION_MANIFEST" --locked --all-targets -- -D warnings` | No lint errors/warnings. Available; not included in current root CI check. Apply the supported WASM target/features too, using its actual build toolchain. |
| Rust/native correctness | `cargo test --manifest-path "$MIGRATION_MANIFEST" --locked` | Existing public request, geometry, transaction/history, archive, CAD/cache/export and renderer tests for the affected crates pass. Core/renderer tests are in root CI; standalone native CAD and contracts checks need explicit invocation. Build `cargo build --manifest-path core/Cargo.toml --locked --example prepare_case` before native CAD fixtures. |
| WASM correctness/build | `pnpm run build:core`, `pnpm run build:renderer`, `pnpm run build:cad` | Fresh production modules and binding packages build; root CI includes these. Compilation alone does not assert browser initialization or behavior. |
| Retained frontend build/tests | `pnpm --dir app build`, `pnpm --dir app test` | Typechecking, production assets/service-worker output and Vitest assertions pass. In root CI while React remains. |
| Dependency/architecture boundaries | `pnpm run check:repo` | Tested AST checker asserts current TS ownership/import direction, typed keymap edits, production reachability and used runtime dependencies/exports. In root CI; does not assert Rust crate layering or absence of duplicated domain logic. |
| Shared contracts | `pnpm run check:contracts`, `pnpm run test:contracts` | Rust-generated bindings match checked-in output; runtime import test passes. In root CI; freshness alone does not approve a changed wire/file contract. |
| Native/WASM boundary correctness | `pnpm run check:boundaries` | Asserts native/WASM request replies and archives/buffers agree, including preview/commit, stale revisions, Undo/redo and invalid input. In root CI; despite the name, this is not a dependency-layer or frontend-parity checker. Requires fresh matching core package. |
| Browser runtime/integration | `pnpm run test:e2e`, `pnpm run test:e2e:dev`, `pnpm run test:e2e:pages` | Existing Playwright assertions cover production, development and Pages deployment paths. Current CI runs all three, with performance specs excluded. Tests target React today; no Dioxus target exists. |
| Representative editor interactions | `pnpm --dir app exec playwright test e2e/canvas-interactions.spec.ts e2e/workbench-selection.spec.ts e2e/outline-editing.spec.ts e2e/outline-snapping.spec.ts e2e/keymap-workspace.spec.ts` | Pointercancel/Escape, board switching, one-step drag Undo, Ctrl/Shift selection, keyboard edits, snapping/Alt, outline editing/reload and keymap export assertions. Included in full production suite; select additional affected workflows, including mirrored layouts/final pointer samples. |
| Persistence/archive correctness | `pnpm --dir app exec vitest run src/storage.test.ts src/storageReset.test.ts src/createProjectActions.test.ts src/createProjectExporter.test.ts` | Existing IndexedDB/archive/assets, reset/recovery, failed actions and captured export ordering assertions. In root CI through app tests; does not characterize every saved user document or all session-save failure cases. |
| Persistence/offline browser behavior | `pnpm --dir app exec playwright test e2e/workbench.spec.ts e2e/project-library.spec.ts e2e/startup-recovery.spec.ts e2e/kicad-artifact.spec.ts` | Save/archive/reload, discovery, cached-shell offline reopen, incompatible-document recovery and imported KiCad preservation. Included in full production suite. |
| Export/geometry compatibility | `pnpm --dir cad test`, `pnpm --dir kicad test`, `pnpm --dir ergogen test`, plus native Rust tests above | Existing geometry/cache, STEP reopen/oracle, KiCad/source/asset, generator integrity and runtime fixtures. In root CI; native CAD tests remain a separate invocation. Preserve their tolerances and supported fixture corpus. |
| Performance/resources | `pnpm run test:perf` and the CAD/live/soak commands below | Existing latency, sampled CAD RSS and full-UI retained-memory assertions. Available separately; not run by current `pnpm run check` or CI. A timing report without its comparator is not a gate. |

For fast feedback, existing focused worker/interaction tests include
`pnpm --dir app exec vitest run src/CoreClient.test.ts src/CaseClient.test.ts src/ExportClient.test.ts src/ui/createCanvasInteractions.test.ts`.
Select actual affected tests in the task record; do not treat this subset as
complete slice acceptance.

### Proposed check tiers

These are invocation/acceptance tiers, not newly installed wrapper commands.
Do not invent a duration limit and drop coverage to meet it. Reuse sufficient
successful evidence for the same source revision, features and built artifacts;
rerun when relevant code, dependencies, environment or assumptions change.

| Tier | Run and record | Completion rule |
| --- | --- | --- |
| Fast development | Affected Rust fmt/Clippy, focused Rust tests (`cargo test --manifest-path "$MIGRATION_MANIFEST" --locked "$MIGRATION_TEST_FILTER"`, selecting an existing affected test), focused Vitest tests, `pnpm --dir app exec tsc --noEmit`, `pnpm run check:repo`; contract freshness/runtime checks when contracts change. | Gives feedback during editing. Missing generated inputs or a failed focused check stays visible; passing this tier does not complete a migration task. |
| Task completion | Format, strict Clippy and complete tests for affected Rust crates; affected package tests; fresh affected WASM builds and frontend production build; contract/boundary checks; representative production browser workflows and paired reference/replacement behavior tests. Include relevant performance, persistence/export, responsive/accessibility and visual comparisons. | Every migrated slice passes Rust fmt/Clippy/tests, WASM build, frontend build, browser runtime and behavioral parity. Visual checks apply where appropriate, with a recorded omission reason otherwise. Applicable failed, blocked or missing checks prevent acceptance. |
| Full integration | `pnpm run check` and `pnpm run test:e2e:dev`, plus fmt/Clippy/native CAD/standalone Rust checks not covered by that composite; approved Dioxus production/build/runtime/parity checks; performance/CAD/live/soak checks below and affected visual/accessibility checks. Reuse the just-built production artifacts for `pnpm --dir app test:perf` rather than rebuilding without a relevant change. | Required before cutover or React deletion and for cross-slice integration. Existing CI coverage is retained; approved additional gates must subsequently be wired into CI. No merge, push or deployment authorization follows from passing. |

Record each run's source HEAD plus dirty-diff/fixture hashes, command, environment,
features/target, artifact identity and result. Preserve raw failed attempts,
ignored diagnostics and unavailable checks. The assessed `96dd51d3` baseline has
existing fmt/Clippy failures and blocked/incomplete runtime checks; it is not a
green acceptance baseline. Do not repair unrelated failures in a migration
task or grandfather them silently. Track prerequisite work or obtain an explicit
scoped exception; the gate remains failed/blocked until resolved.

### Enforcement gaps and activation prerequisites

| Requirement | Current evidence / proposed check work |
| --- | --- |
| Dioxus production build and browser runtime | No Dioxus application crate or installed `dx` CLI. Pin compatible 0.7.x crate/CLI and add the exact version-verified web release build command, artifact identity and production browser target. Until then, no executable Dioxus build gate can be claimed. |
| Reference/replacement behavioral parity | Existing Rust and React fixtures are reusable, but no paired frontend runner exists. Run equivalent actions/documents and assert approved documents, history, errors, exports and lifecycle effects on both targets. For approved defect/change tasks compare against the approved corrected oracle, preserving unaffected parity. |
| Rust dependency/layer boundaries | `cargo tree --manifest-path "$MIGRATION_MANIFEST" --locked --edges normal,build` provides dependency evidence, not a pass/fail architecture verdict. After the coordinator/module decision, implement an allow/deny check over the approved dependency graph and negative tests for forbidden edges. Retain meaningful existing TS checks until their paths retire. |
| Browser boundary/domain authority and temporary code | Owner/contract/retirement records and architecture/diff review are manual policy checks today. No automatic checker proves unique domain authority, worktree preservation or adapter retirement. Add a validated inventory checker for required owner/removal-task/retirement fields before claiming automated enforcement. |
| Quality floor | No diff-scoped floor guard currently enforces all suppressions, test/assertion removal, weakened thresholds/boundary rules and unfinished stubs. Propose adapting the installed skill's `references/floor-guard.md`, covering Rust patterns, staged/unstaged/untracked changes and check failures, with negative regression tests. A heuristic guard still requires review for semantic weakening. Do not install or activate it in this proposal session. |
| Visual regression | Current Playwright captures failures; it has no approved screenshot comparison baselines. Add paired assertions at controlled viewport/DPR/themes/fonts and approve the reference before use. Never update screenshots merely to obtain a pass. |
| Accessibility/responsiveness | Existing narrow-layout, zoom, themes and labeling assertions in [case readiness](app/e2e/case-workbench-readiness.spec.ts), [header](app/e2e/header-consolidation.spec.ts), [scroll layout](app/e2e/scroll-layout.spec.ts) and [keymap](app/e2e/keymap-workspace.spec.ts) are partial checks. Axe is not installed/integrated; paired tests and manual keyboard/relevant assistive-technology checks remain required. |
| Saved-document corpus and durability | Archive/storage tests are executable, but actual supported saved-document coverage and coordinator save-failure/rollback characterization are incomplete. Establish that corpus and executable failure/order/recovery tests before replacing their ownership. |
| Coverage and unmeasured resource budgets | No accepted measured coverage threshold, startup/size budget or crossing budget exists. Do not introduce a default percentage, a historical diagnostic as a ceiling, or a pretend passing gate. Measurement proposals are below. |

Gate activation is later work: agree unresolved contracts/measurement protocols,
implement real checks with failure regressions, trial-run them without changing
the reference, and wire the approved tiers into CI. This proposal does not
resolve the [application-coordinator decision](.scratch/dioxus-browser-trial/issues/03-trial-architecture-acceptance.md).

## Performance

Migration must not materially regress startup time, interaction latency,
memory use or bundle/WASM size without documenting the tradeoff. Preserve the
existing budgets and comparison rules in [the performance baseline](docs/performance-baseline.md)
and [generation performance evidence](docs/generation-performance.md).
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
| Workbench 100-single / 100-row / 200-single / 200-row | Worker p95 limits: 5.17 / 4.40 / 7.59 / 7.70 ms; painted p95: 36.96 / 37.84 / 46.97 / 53.57 ms. Derived from the measured baseline multiplied by the existing 1.1 allowance, using median of five session p95s with 100 samples each. | [Baseline JSON](app/performance-baseline.json), [runner and exact comparator](app/scripts/run-performance.mjs), [rounding regression tests](app/scripts/performance-budget.test.mjs). Preserve the comparator's nanosecond normalization, not rounded console limits. |
| Pointer-to-painted movement | p95 <=33 / 50 / 100 ms for 30 / 100 / 200 keys; preserve visible-transform assertions, warmup and measured samples. | [Pointer performance](app/e2e/pointer-performance.spec.ts). Frame-gap diagnostics are not an additional asserted release budget. |
| Outlines and matrices | Outline p95 <100 / 200 ms for 100 / 200 keys. Matrix/row/column painted p95 <=100 / 200 ms for 100 / 200 keys, with 100 measured samples. | [Outline](app/e2e/outline-performance.spec.ts), [matrix](app/e2e/matrix-performance.spec.ts). Thirty-key matrix timing is measured without an asserted latency cap. |
| CAD completion, paint and sampled RSS | Preserve all 30 scenario-specific absolute limits and eligibility/sampling/fixture checks. For example, cached gasket paint is <=38 ms; the historical 31.4 ms result is evidence, not a new threshold. | [Frozen budgets](cad/bench/budgets.json), [comparator](cad/bench/compare.mjs), [protocol](cad/bench/README.md). Do not apply one RSS value to every scenario or rewrite these limits from a candidate run. |
| Live release-to-painted edits | The existing non-regression limit is reference p95 + max(4 ms, 10% of reference p95) for numeric, Undo, gasket and mount actions. Comparable evidence requires at least five sessions and the existing CPU/browser/GPU/viewport/sample provenance. | [Live comparison](app/scripts/compare-live-performance.mjs), [comparison tests](cad/test/live-performance-comparison.test.mjs), [five-session runner](app/scripts/run-live-performance.mjs). The CLI also requires an optimization improvement; see below. |
| Full-UI retained memory | Existing 256-cycle soak compares medians for cycles 129–192 and 193–256, separately by project. Growth allowance: max(2 MiB, 10%) for JS heap; max(2 MiB, 5%) for summed WASM capacity. Pending callers must settle. | [Full-workbench soak](app/e2e/live-soak-performance.spec.ts). This does not bound cold/peak memory or process RSS; the opt-in environment variable must be set so the test actually runs. |

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

## Slice acceptance and removal

1. Before coding, record the workflow, state/domain owners, browser boundaries,
   task category and approved oracle, compatibility fixtures, baseline evidence,
   required checks, rollback path and React/bridge removal conditions in the
   slice design linked from PLAN and TODO.
2. Implement the complete vertical slice while retaining a working fallback
   for unaffected workflows. Isolate trial persistence and preserve source
   documents. Record any intentional visual change or performance tradeoff.
3. Run the gates against the replacement and reference; attach commands,
   revisions, results and comparison artifacts. A missing or failing parity
   test blocks acceptance, cutover and React deletion.
4. React code may only be deleted after its replacement passes parity testing
   and the other required gates. Transfer equivalent regression coverage
   before removing implementation-specific tests; rerun affected checks after
   deletion and verify remaining routes/imports.
5. Do not leave duplicate implementations indefinitely. Every temporary
   implementation, parallel UI or adapter (Rust or JavaScript) must have a named
   accountable owner, a linked removal task, explicit retirement criteria with
   test/contract evidence, and a dated deadline or named follow-up slice. Record
   its purpose, callers and temporary boundary in the existing slice record;
   unowned or untracked temporary work is not acceptable.
   Remove obsolete code, dependencies and runtime paths once those criteria
   pass. Completed migration cannot contain duplicate domain implementations
   or unowned migration bridges.

## Protected work and publication

- Preserve existing protected/historical Rust work, worktrees, research bundles
  and unrelated local changes. The [assessment inventory](docs/dioxus-context-baseline.md#checkout-and-preservation)
  records their locations; recheck live status before editing overlapping paths.
  An ancestor commit or a prunable worktree entry is not permission to discard it.
- Keep migration work on its isolated branch/worktree. Do not reset, clean,
  delete, recreate or switch existing protected worktrees, overwrite unrelated
  edits, or rewrite Git history as part of a migration task. Any separately
  requested destructive operation needs explicit authorization and preservation
  evidence first.
- Do not merge, push or deploy without separate explicit authorization.
  A request to plan, implement or validate a slice does not authorize publication.

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

This floor is a required review policy today; the automatic floor-guard gap is
listed above. Review staged, unstaged and untracked changes against the task's
recorded starting state, including untracked constraints and copied documents.
`git diff` alone misses untracked files and is evidence for review, not an
automatic assertion that the floor is intact.

No exceptions are approved by this document. Any proposed exception must name
the rule, exact scope, reason, evidence, owner, explicit user approval, expiry
date and tracked remediation. A blocked gate remains blocked until its
requirement is met or the user explicitly approves the scoped exception.
Hard requirements remain in force unless the user explicitly revises them.
