# 011: Layout explicit-submit owners

Status: resolved
Type: build
Assignee: Coordinator (closure)
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the Layout cluster's
remaining explicit-submit owners — the Layout constraint composite, the legacy
position Inspector, and companion placement — so their unapplied Draft values
defer Update Now until resolved through each form's own submit or cancel
controls. (Geometry scripts, the fourth Layout owner, lands with the tracer.)
The Layout constraint adapter covers the confirmed hazard that an accepted
update currently clobbers an open constraint draft.

## Acceptance criteria

- [x] Each owner defers Update Now with a real valid unapplied draft and with a realistic invalid unsubmitted draft, at both entry points, via the mounted Update Now seam.
- [x] Each owner's deferral message names its editing area and per-key remedy; resolution happens only through that form's own controls.
- [x] The Layout constraint draft is protected while open: an accepted update landing does not silently discard the designer's unapplied constraint draft, and the owner's unresolved judgement follows the settled baseline rules.
- [x] Per-key behaviour follows the contract: landing updates the baseline, failure/refusal never clears, revert-to-baseline clears, newer typing re-raises, older settlement never clears newer input.
- [x] Hidden or keep-mounted instances of these owners keep blocking; unmount or scope change retires them.
- [x] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.

## Approved correction transfer (2026-11-07) — claimed, awaiting fresh reviews

This section supersedes the historical remediation/progress claims below.
The five review findings now have source fixes and focused mounted evidence;
011 remains **claimed, NOT resolved** pending fresh Spec + Standards review
and coordinator verification. No child subagents, full build or full gate ran.

### Decision authority

The coordinator asked: “Approve this narrow contract amendment? Keep Pair
checked after Apply, but advance its safety baseline only when that Apply
succeeds. Failure/refusal would still block, and later changes would block
again.” The user answered: “Approved. Retry a few times when gortex fails the
continue”. Recorded first in [004 Answer](004-choose-draft-safety-interface.md#later-user-amendment-sticky-pair-action-parameter-2026-11-07),
with aligned [map](../map.md) and [spec](../spec.md). The historical
[004 adversarial check](../inventory/004-contract-check.md) is unchanged.
Only Pair gets this exception: mount-default initially, then this binding's
own successful Apply's **submitted Pair parameter**, never a current unsent
parameter, failure/refusal, other key or retired owner's result. No Apply
policy or other owner's parameter baseline changed.

### Findings and distinct regression purposes

1. **Pair reset / resolution:** removed the click reset, retained the sticky
   checkbox through success and Undo, and restored the original refusal test's
   explicit uncheck before tweak mode. A separate Pair baseline advances only
   on `Landed(CompanionPlacement)` with the submitted binding and parameter.
   `mounted_companion_refused_pair_apply_preserves_pair_intent` preserves retry
   intent and now exercises explicit retry to success. The existing landing
   test now checks sticky success, later toggle deferral, and restore/clear in
   one coherent scenario. Added durable-save failure coverage with other fields
   restored (Pair alone still unresolved); a separate pending-control case
   proves a queued toggle is rejected, rather than inventing newer editable
   input while disabled. The held-Core old-owner case protects a new binding's
   checked parameter from an older owner's successful landing.
2. **Constraint kind/section clobber:** canonical comparison now runs before
   kind/section writes, all within the original clean/own-landing guard.
   `mounted_layout_constraint_newer_kind_and_section_survive_an_older_save`
   changes Offset to Mirror line and collapses the section across an older Save;
   both the visible choices and draft deferral survive.
3. **Remove clobber:** Remove records its click-time canonical form and owner
   identity. Its landing resets only matching, unchanged input; failure/retirement
   drops that observation without resetting. Newer editable input survives an
   older Remove. Separate unchanged-invalid and failed-Remove cases prove the
   reset is neither disabled entirely nor allowed on failure; the original
   clean/dirty Remove-resolution tests remain.
4. **Companion poll freshness:** every gate poll calls the actual
   `presentation.rs::companion_placement_projection` with the live accepted
   model and captured owner context/generations. Render-cached accepted baseline
   is removed; rendered targets still describe the live form, not its accepted
   baseline. `mounted_companion_gate_projects_accepted_state_before_rerender`
   invokes the real gate synchronously from a Runtime accepted-state subscriber,
   before Dioxus can refresh the form. A clean-looking stale form must defer.
5. **Binding lifetime:** L1 registration rotates on project scope, owner identity
   and board-target IDs; its reselection test proves an untouched new component
   no longer compares with the old component's constraint. L15 uses a private
   binding identity (context, selection, generations, pair and canonical targets),
   excluding accepted revision/token: revisions refresh admission, not ownership.
   Registration rotates on this binding and scope. The production-projector
   landing test exposed the old revision-driven checkbox reset (RED), and now
   passes. The held-Core same-scope component→key reselection proves an older
   landing cannot reset or baseline the rebound Pair; no keyed-remount claim is
   substituted (production mounts in `layout_workspace.rs` are unkeyed).

All new cases protect distinct failures/timing/ownership paths; no new testing
DSL, configuration modes, ignored tests or baseline/name deletions. Existing
mounted hosts, real Runtime lifecycle, gate and helpers are reused. The new
eligible-companion document builder and small live-projection host were essential:
the earlier synthetic independent-part projection cannot be reconstructed by the
production projector and froze owner identity, masking the revision bug. Review
focus: the older synthetic display/action host still has partially duplicated
subscription/Apply plumbing; it remains for its original test, not broad cleanup.

### RED/GREEN and checks

Isolated runs use the existing `scripts/run-wasm-tests.py` functions
`runner_environment`, `desktop_webdriver_config` and `run_filter` through a
Python import, with the exact full test name; each launches its own browser page.
No panic cascades are treated as independent regressions. Logs all live under
`.evidence/update-draft-safety/011-approved-`:

| Slice | Expected RED | GREEN |
| --- | --- | --- |
| Pair refusal | `red-pair-isolated.log`: checked `false`, expected `true` (1 failed) | `green-pair-refusal.log`: 1 passed |
| Pair success baseline | `red-pair-success.log`: landed form still deferred | `green-pair-success.log`: 1 passed |
| Constraint kind | `red-constraint-kind.log`: newer Mirror line overwritten | `green-constraint-kind.log`: 1 passed |
| Remove newer input | `red-remove-newer.log`: newer `11` reset to `0` | `green-remove-newer.log`: 1 passed |
| Poll freshness | `red-companion-poll.log`: no deferral before rerender | `green-companion-poll.log`: 1 passed |
| Constraint reselection | `red-constraint-rebind.log`: untouched rebound owner falsely deferred | `green-constraint-rebind.log`: 1 passed |
| Production revision binding | `red-companion-binding.log`: sticky Pair became unchecked | `green-companion-binding.log`: 1 passed |

Fixture development failures (missing explicit mirrored parts before Core Open)
were corrected before the poll RED above; they are not behavioral RED evidence.
The first additional guard run found two invalid test assumptions (pending toggle
is rejected; mirror-board selection needs a scope switch). The corrected pending
control and same-board component→key owner cases pass in
`pair-guards-recheck.log` (2 isolated passes). Refusal/retry, unchanged-invalid
Remove and failed Remove pass in `pair-remove-guards.log`; durable Pair failure
passes in `pair-save-failure.log`. These characterize the fix, not new mutation
kill claims. The original 8 killed-mutant claims remain historical; 015 owns fresh
mutation/full-gate closure.

- `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_draft_safety_tests.rs web/src/presentation/layout_component_inspector_tests.rs web/src/presentation/inspector.rs`:
  **106 executed, 0 failed**, `focused-final.log` (first integrated pass was 101/0).
- `rustfmt --check --edition 2024 --config skip_children=true` on the four changed
  Rust files: PASS, `fmt-final.log`. Formatting was applied through native Gortex.
- `cargo clippy --package boardstudio-web --target wasm32-unknown-unknown --all-targets --locked --no-deps -- -D warnings`:
  PASS, `clippy.log`. Builds/checks serialized against the shared target.
- `python3 scripts/check-doc-links.py`: PASS, **717 Markdown files**, `doclinks.log`.
  `git diff --check`: PASS. Full owned Rust and documentation diff read before
  commit; no full acceptance is inferred from these checks.

### Source proof and limits

Native Gortex supplied all source reads/edits, pre-edit impact, post-edit detect,
tests/guards/contract. Actual checkout `01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378`
was exact and usable despite the stale discovery banner; no source fallback,
CLI, tracking, daemon, proxy or reindex changes. No transient retry incident
occurred. Graph coverage reports zero WASM test edges, guards has no configured
rules, and contract warns on behavioral complexity/blast lower bounds; contract
attachment rebuild is **not certified current**. These do not replace mounted
proof or fresh two-axis review.

Native production proof: WorkbenchApp provides `Rc<Runtime>` at
`presentation.rs:463` above Editor (`:722`), whose Layout mount passes both
inspectors through `layout_workspace.rs` (component `:944`, companion `:966`).
Thus optional-context absence is limited to isolated display fixtures, not a
production bypass. L8's `submitted_draft` is already pending-gated in the shared
helper; no redundant `is_pending` patch. Actual canonical enums were inspected:
`MirrorAxis` is a derived unit enum; `CompanionPlacementTarget` is a derived tagged
enum over strings/u32/unit/data-free `MirroredItemKind`, not arbitrary floats or
fallible custom serializers. No concrete serialization-failure collision was
found; no speculative JSON/typed refactor was introduced.

Coordinator-owned `execution.md` and all other tickets/runtime/helpers/notifications
were untouched. 008 trusted activation remains owed to 010; 009's 10+1 reviewed
tests are not packaged acceptance. Worker/cache/Toast debt and umbrella 019 remain
open. Next queue: **fresh 011 Spec + Standards reviews and coordinator independent
full diff/reject runs, then 010 packaged proof before 012–014**. Full gate/build
and final closure belong to coordinator 015.

Authoritative child checkpoint:
`/01a1b6ae-9da7-76e3-bc0e-8ae69844562a/notes/update-draft-safety/011-approved-fixes.md`.

## Historical resumed remediation — previously blocked on Pair adjudication

011 remains **claimed, NOT resolved**. The earlier progress record below is
historical, not current acceptance: both review axes requested changes. Its
claim that an Apply-path checkbox reset preserves failed Pair intent is false.

Native Gortex mutation admission now succeeds in the exact shared worktree
(checkout `01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378`, initial route epoch 455,
post-test epoch 457). No source fallback, CLI, daemon or route changes were used.
Prior delegate/coordinator virtual notes were unavailable to this child
(`file:null` retrieval warnings, bounded retry); the supplied briefing and
native repository evidence were used instead. Authoritative child checkpoint:
`update-draft-safety/011-sol-resumed.md`.

### First vertical slice: Pair refusal RED, no policy invented

Added `mounted_companion_refused_pair_apply_preserves_pair_intent` at the
already-approved mounted S1 seam. It checks the Pair-only draft over unchanged
accepted Literal mirror, forces the real resolver refusal through the existing
fixture, and requires the checked parameter and draft deferral to survive.

Command: `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs`.
Log: `.evidence/update-draft-safety/011-sol-resumed-red-pair.log`.
Result: **44 executed, 17 failed**; first failure is the expected assertion:
Pair `aria-checked` was `false`, expected `true`. The other 16 failures are
shared-WASM panic-hook cascades, not independently adjudicated regressions.
No GREEN, production fix, full build or full gate is claimed.

**Precise decision needed before a Pair fix:** Git change evidence from
`043787ba5` against parent `86bd4620b` proves the original Apply retained the
checked box even through a successful landing and Undo; the existing test
explicitly unchecked it before selecting tweak mode. 011 introduced the reset
and removed that test step. Moving the reset to Landed would fix refusal but
still change the protected original Apply policy. Conversely, retaining the
box with 011's unchecked mount-default action-parameter baseline leaves the
key unresolved after success (the current landing test expects it to clear).
004 B1/B2/S4 allows immutable mount-defaults for action parameters, not a new
last-successful-submit baseline or a silent post-landing reset. Please decide
whether successful Pair submission may reset the checkbox, or retain the
original sticky checkbox and explicitly define its post-success resolution
semantics. Neither choice has been silently implemented.

### Other findings revalidated natively, not fixed or behaviorally proven yet

- L1 kind/section clobber: `LayoutComponentInspector` lines 1322–1333 write
  `constraint_open` and `constraint_kind` before the live canonical comparison
  at 1344–1364. These writes are outside its clean/own-landing guard.
- L1 Remove clobber: lines 1527–1542 reset every constraint field on any Remove
  Landed. The Remove handler at 2056–2069 stores no current-draft snapshot;
  equality/epoch-aware preservation still needs RED/GREEN.
- L15 poll freshness: `CompanionPlacementInspector` lines 260–304 reads
  render-cached `companion_baseline_live` inside the gate closure, not the
  actual `presentation.rs::companion_placement_projection` (1701–1891).
  Required before-rerender regression and production-projector fix remain.
- Identity rebind: L1 registration depends only on `draft_owner_scope` at 1555
  but captures `part_id` and `board_parts` at 1570–1609. L15 likewise depends
  only on scope at 260; targets are refreshed signals, but binding epoch does
  not rotate with owner/target identity. No keyed-remount/reselection proof
  has been established, so this finding remains open.
- L8: current source at `inspector.rs:145` already calls `submitted_draft`;
  it has no extra `is_pending` check to add. No unasked churn performed.
  Production Runtime provider/callsite proof and actual enum-serialization
  risk adjudication remain uncompleted, not waived.

Gortex after the RED-test edit: `detect` identifies the new test symbol;
`tests` reports 0/1 graph coverage, `guards` has no configured rules,
`contract` says allow for that test-only structural change. This does not
certify contract attachment freshness or behavioral closure. The original
8 killed-mutant claims remain historical and were not replaced by this RED.
Fresh independent reviews and coordinator-owned 015 full gate remain due.
008's conditional review/010 packaged trusted-input proof, 009 driver
integration, worker/cache/Toast work and umbrella 019 remain outside this
child's acceptance claim. Coordinator `execution.md` was left untouched.

## Progress (implementation, 2026-11-07; NOT resolved — awaiting independent review)

Implemented on branch `fix/ui-integration-recovery` (three registry adapters
at both Update Now checkpoints). Mounted GREEN at seam S1; packaged proof
belongs to 010. Stopping-point reports filed in notes
(`update-draft-safety/011-implementation.md`) and `.evidence/`
`011-red-{l8,l1,l15}-slice.log`, `011-green-{l8,l1,l15}-*.log`,
`011-green-module-full.log`, `011-green-inspector-file{,-full}.log`, and
`011-mutation{,-recheck}/`.

- L8 legacy position (`inspector.rs`): single Apply key; owner-canonical
  parsed equality (`"1.0"` vs `"1"`); invalid text never matches; in-flight
  exemption only via pending `submitted_draft`. No resync change needed — the
  existing dirty/clean effect already preserves open drafts.
  Area `Legacy position`, remedy
  `Press Enter or Apply position to save, or press Escape to cancel`.
- L1 constraint composite (`layout_component_inspector.rs`): canonical
  per-kind form state (only the active kind's fields participate); the
  007 clobber is closed by a clean-gated accepted-constraint resync
  (previous-baseline comparison), an own-landing follow gated on a
  per-key Landed nonce plus submitted-equality (newer input across an
  older settlement is never clobbered), a submitted-memory record on the
  validated Save/Convert path (ConvertOffsetAxes shares the Constraint
  key), and a Landed(RemoveConstraint) form reset to the no-constraint
  defaults (failure/refusal drains no Landed result, so the draft is
  preserved). Area `Layout constraint`, remedy
  `Save or remove the constraint, or restore its saved values`.
- L15 companion placement (`companion_placement.rs`): per-key canonical
  state (mode, pair flag, tweak iff tweak-mode, sorted targets); invalid
  and pair+tweak-refused states never match; poll-cadence judgement was
  replaced by render-cadence baselines after proving the
  `use_context_provider` value freezes at mount — the shared runtime
  host now refreshes mode/tweak from the live accepted document per
  render (mirroring the production parent), and the component follows
  clean baselines the same way. The pair checkbox resets on the validated
  Apply path only (the accepted baseline never models Pair inclusion); a
  failed Apply keeps the pair intent. The existing refusal test drops
  its now-redundant checkbox click (the reset flips its toggle parity).
  No artificial newer-typing-during-pending test exists: the form disables
  while pending, so that interleaving is structurally impossible; the
  clobber-preservation test covers the temporal rule instead.
  Area `Companion placement`, remedy
  `Apply the placement, or restore the saved values in the form`.
- Tests: 40 mounted seam tests (L8 12 incl. in-flight exemption +
  newer-typing; L1 15 incl. clobber-preservation, in-flight exemption and
  dirty-remove resolution; L15 13 incl. in-flight exemption) + 15
  canonical unit tests (formatting/invalid/kind-scoping/baseline shapes).
  RED first per owner (stash-isolated identical tests fail for the expected
  reason), then GREEN. `check.py quick` (repo/test-css/lint/typecheck/test/
  browser, 204 tests, 0 failed) and `check.py build` pass.
- Mutation: 8 focused mutants over the per-key unresolved branches,
  clean-gated resync, Landed reset and pair reset — 8 killed, 0 survived
  (one needed a new dirty-remove test to kill).
- Gortex: `change(detect)` maps the change to the three owned components;
  `change(tests)` reports 0 graph-tracked tests (the graph is blind to
  `wasm_bindgen_test` edges — behavioural proof is the 40+ mounted tests
  above); `change(guards)` has no rules configured; `change(contract)`
  verdict warn (behavioral change, blast radius reviewed via the full-file
  regression runs above).
- Gotchas preserved for reviewers: `use_reactive` effect bodies must read
  signals with `.peek()`, never `()` (a `()` read subscribes the effect
  to signals it writes — observed once as a render-loop OOM); the
  `pending_edits`/`submitted` helpers are per-key and pending-gated;
  `use_context_provider` freezes its value at mount.
- Trusted-input claims stay reserved for 010: entry buttons in the new
  fixtures call the production gate openers directly (real opener, fixture
  chrome), consistent with 008's scope split.

## Answer

Coordinator acceptance (2026-11-07): original `043787ba5`, isolated Pair RED
`94b9acf` and approved correction `16b85737` have complete Spec and Standards
PASS with no Required changes. Coordinator read both full source diffs and
independently reran **106 executed / 0 failed**, including rejecting/binding
paths (`011-coordinator-corrected-browser.log`). The five required corrections
and distinct RED/GREEN cases are recorded in the approved transfer above;
all six criteria are met. Historical claimed/pending statements are superseded.
[010](010-packaged-a-to-b-proof.md#answer) additionally proves the real imported
Layout constraint with trusted input and exact A/B durable export, with cleanup
complete. This does not expand the Pair-only amendment or certify new mutation
kills: fresh integrated mutations, optimized/full gate and final review remain
015 obligations. Parent umbrella 019 stays open.
