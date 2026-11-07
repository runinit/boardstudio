# 011: Layout explicit-submit owners

Status: claimed
Type: build
Assignee: PiSol-011-resumed
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
- [ ] Per-key behaviour follows the contract: landing updates the baseline, failure/refusal never clears, revert-to-baseline clears, newer typing re-raises, older settlement never clears newer input.
- [x] Hidden or keep-mounted instances of these owners keep blocking; unmount or scope change retires them.
- [ ] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.

## Resumed review remediation — blocked on Pair policy adjudication

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
