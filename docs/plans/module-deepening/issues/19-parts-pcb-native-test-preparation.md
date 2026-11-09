# 19: Parts and PCB native tests prepare the panel migration

Status: resolved
Type: build
Blocked by: 03, 04
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Downstream: [Parts and PCB panels settle through PendingEdits](08-parts-and-pcb-onto-pending-edits.md) · Decision: [Native Runtime answer](02-decide-native-test-runtime.md#answer)

## Owner and purpose

Prepared for the user's third AI app as an independent coding stream. The orchestrator
owns the claim, dedicated worktree and integration; agents here must not duplicate the
assignment. This slice can start with the merged native Runtime and constructors,
while captured Scope, keyed settlement and shared helpers are built in other streams.
It changes test preparation only; panel settlement migration still waits for the
complete Matrix tracer gate.

## What to build

Replace the remaining duplicated Session/Core effect pumps in the already native-
compiled Parts test modules with the real test Runtime. Current source has concrete
manual test drivers in these files; this is not a request to invent another harness.

Own only test modules/fixtures in:

- `web/crates/parts/src/parts_custom_definition.rs` (the native Driver);
- `web/crates/parts/src/parts_new_component.rs` (open/advance/resolve helpers);
- `web/crates/parts/src/parts_definition_name.rs` (open/advance/collect helpers);
- `web/crates/pcb/src/pcb_wiring/mode_owner_tests.rs` (the existing registered native
  mounted owner harness; add missing domain regression coverage using its fixtures).

The three Parts modules already compile natively, and PCB's owner tests already use
Runtime and exercise mode/pin/apply/remap hooks. Both crates already have Runtime
`test-support` dev dependencies. No new Cargo dependency, production module seam or
visibility widening is needed. Preserve pure row-identity/projection tests.

## Acceptance criteria

- [x] The three Parts manual Session/Core pumps are removed; helpers open, submit,
  observe, hold/release and fail through the shared native Runtime.
- [x] Existing definition, pad identity, name, create, deterministic-seed, unrelated-
  edit preservation and Undo/Redo assertions retain their domain meaning.
- [x] Cover queued Parts edits preserving prior accepted changes and target departure,
  with gates where needed. Do not duplicate generic ticket settlement tests.
- [x] Audit existing PCB mode/pin/apply coverage and add missing domain-level queue
  preservation/target-eligibility regressions through the existing native hooks;
  retain strict protected-remap fingerprint/revision behavior.
- [x] Tests execute under the native package commands; do not call dormant/unregistered
  scenarios verified. Fixtures open valid documents through Session/Core rather than
  mutating accepted ReadModel or manually settling outcomes.
- [x] Production resolvers, panel feedback/state, catalogue sequencing and APIs remain
  unchanged. No PendingEdits/helper migration occurs here; the later panel ticket
  consumes these tests and updates only approved retirement/Saved presentation cases.
- [x] Test support stays native-test-only where required; WASM compilation introduces
  no unused helper/import warnings. No broad lint allowance or provider changes.

## Verification

```sh
cargo test -p boardstudio-web-parts -p boardstudio-web-pcb --locked
python3 scripts/check.py lint typecheck test
```

Run affected Parts/PCB browser tests when their shared test modules change; coordinate
one Chrome lease through the orchestrator. Use CARGO_BUILD_JOBS=2. For the CAD stage,
unset CARGO_TARGET_DIR or use the worktree default target, because its prepare_case
helper expects that path. Record the known CAD baseline honestly.

Follow handoff rules, including TDD at the existing Runtime/domain seam, impact/source
checks, explicit-path commits, parallel Standards/Spec review, and final rebase on dev.
Do not change tracker status or map. Report commits, deleted pumps, preserved/new
scenarios, actual test counts and limitations. Report a required production fix or
visibility change before editing outside this bounded test-only scope.

## Outcome

Integrated the external third-app branch at `8d203414433f9c2f12ef3bdefc21385d69a270c4`,
including `f700c2acf` (native Runtime migration) and `8d2034144` (queued Apply coverage).
Both external Standards and Spec reviews reported no blocking findings after the
Apply gap was closed. Root confirmed the clean branch, four-file test-only diff,
diff checks and WASM test registration check. No production behavior or lint allows
changed. PCB helpers remain in the native-only registered test module; the review's
hypothetical future WASM divergence is not an active warning or required follow-up.

Removed the Parts custom-definition Driver effect pump, new-component open/advance/
resolve pumps, and definition-name open/advance/collect/submit-name pumps. Helpers now
open and submit through real native Runtime and observe operation slots. Existing
test functions and definition/pad/name/create/seed/preservation/history assertions are
retained. Runtime-dependent tests are native-only; pure row/projection tests remain
available to both targets. Native Parts passed 42 tests and PCB 21 tests.

Added held-Core queue preservation and target-departure coverage for Parts names and
pad fields; queued creates retain distinct identities. PCB mode, pin and Apply edits
preserve earlier accepted changes and reject departed boards; Apply also rejects a
changed wiring mode. Saved feedback remains the existing behavior pending the later
panel migration, not a new settlement policy.

External checks: lint and typecheck passed; WASM test build passed with existing
warnings in Parts preview and PCB wiring. Root then held the exclusive Chrome lease
and executed the missing browser gates at final HEAD: Parts 50/50 and PCB 38/38 passed.
`check-wasm-tests.py` reports no new WASM-only plain tests.

The full test step retains the already independently confirmed CAD baseline:
49 passed, 1 failed, 4 ignored; rotated-concave/bottom expected 80481.2399,
actual 80579.55733514718, tolerance 0.1. No CAD source changed. GitNexus compare found
four changed files but zero symbols; source inspection establishes the test-only
scope, so this is incomplete symbol coverage rather than proof of no dependencies.
Preparation is complete. Parts/PCB settlement still waits for the Matrix tracer gate.
