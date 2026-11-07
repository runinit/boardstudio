# 004: Choose the draft-safety interface and lifetime contract

Status: resolved
Type: grilling
Label: wayfinder:grilling
Mode: HITL
Assignee: Chris D
Blocked by: [Inventory project draft owners](001-inventory-project-draft-owners.md), [Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md), [Locate a deterministic packaged persistence hold](003-locate-packaged-persistence-hold.md), [Close the named census gaps](007-close-named-census-gaps.md)
Parent: [Draft-safe update adoption](../map.md)
Related: [Draft-safety research and change map](../../../research/2026-11-07-019-draft-safety-change-map.md)

## Question

Given the owner census and real draft/save observations, what is the smallest
interface that lets both Update Now safety checkpoints account for unresolved
project drafts without changing their existing Apply/Save behavior?

## Decisions to resolve with the user

- Compare extending real binding/owner seams with narrow adapters for unbound
  composite forms; consider an aggregate only after establishing the actual callers.
  Do not assume `PendingEditSignals` is already a global dirty-state inventory.
- Specify accepted-baseline comparison, valid versus invalid local values, submission
  before/after asynchronous work, older settlement after newer typing, and multiple
  owners. Separate Draft value, Pending edit and Edit preview.
- Specify identity, registration/retirement, hidden/portal retention, genuine unmount
  and rebinding, reset/cancel, scope/project changes and failed admission. Name what
  must be observed synchronously and what cannot be inferred from focus or DOM ARIA.
- Keep `update_safety_inputs` shared by initial adoption and post-takeover reload;
  retain engine/save/recovery policy. Choose truthful initial/installed deferral copy.
  Retain existing automatic resume unless the user explicitly changes that policy.

## Resolution and boundaries

Use `grilling`, `domain-modeling` and `codebase-design`; require the human's live
answers, not an agent-authored stand-in. Record the selected contract, alternatives
and tradeoffs in `## Answer`, linking evidence and the implementation change surface.
An optional disposable sketch may clarify the discussion, but this is not product
implementation. Do not add an ADR unless the repo's tradeoff criteria actually apply.
Claim first; unresolved facts remain explicit rather than silently assumed.

## Answer

Resolved 2026-11-07 by live grilling (three rounds, user accepted all
recommendations), then amended per the adversarial check
[004 contract check](../inventory/004-contract-check.md) (core not falsified;
5 blocks-recording + 6 should-amend findings, all one-sentence amendments,
user accepted). Inputs: [001 census](../inventory/001-draft-owner-census.md)
(14 Class A owners),
[002 probe](002-establish-real-editor-draft-loss-probe.md) (confirmed packaged
loss), [003 persistence hold](003-locate-packaged-persistence-hold.md).

### The contract

- **Seam.** A small scoped registry of live draft owners, owned by `Runtime`
  (`RefCell` of type-erased entries: key, epoch, `is_unresolved` closure,
  area label, remedy), registered/unregistered from components holding
  `Rc<Runtime>` and polled synchronously inside `update_safety_inputs` at
  both Update Now checkpoints (initial adoption and post-takeover
  `reload_when_safe`), feeding a new unresolved-draft blocker alongside the
  existing engine/save/recovery inputs. Guard-owned (RAII/`use_drop`)
  registration; poll prunes entries whose project scope is not the current
  accepted document, and demonstrably dead entries are dropped, not treated
  as blockers. Adapters registering per-owner closures are the primary
  vehicle for Class A forms; `PendingEditSignals` bound-field dirtiness
  reporting is the convenience for the four already-bound owners (Geometry
  scripts, legacy position, module profile, mounted-module placement).
  The `aria-invalid` DOM query stays as an additive-only fallback. No editor
  Apply/Save behavior changes.
- **State rule (per owner + admission key).** Unresolved = live draft differs
  from baseline AND no in-flight submission for that admission key carries
  the current draft value. Baseline is per-key: a live accepted projector
  returning `Option` (projected at each poll, never snapshotted), falling
  back where no accepted value can exist (creation forms, action parameters)
  to a mount-time initial-default snapshot. Comparison is owner-canonical
  (the value that would be submitted; each registration states its equality,
  parse-then-compare vs exact). Landing updates the baseline; failure or
  refusal never clears; newer typing re-raises; reverting to baseline
  clears. Submitted memory is per admission key (the existing per-key
  `Submission` shape, extended by adapters); the click → preparing-flag →
  `begin` → pending/failed transfer covers async admission (module attach,
  assembly Place/Matrix) with no unprotected window.
- **Lifetime.** Registration keyed (project scope, workspace, owner identity,
  admission key) + binding epoch; hidden/portal-retained owners still block;
  retires on genuine unmount/owner retirement or scope switch (mechanically:
  poll-pruning above). Only the owner's own Cancel/Reset clears a report
  outside submission; the gate only defers — never force-submits or discards.
- **Copy.** Area label + remedy are per admission key (e.g. Assembly tree →
  "Save assembly or close"; Assembly origin → "Place, Apply to a matrix, or
  close"); initial deferral names area + remedy, installed-phase wording is
  distinct, cancel-only, values/focus preserved. Automatic resume retained.
- **Coverage.** The 14 Class A owners (including creation forms and action
  parameters under their consuming admission keys) are required. Class B
  (blur-committing) registration is deferred as an **explicit, revocable
  risk acceptance**: activating Update Now moves focus, firing blur-commit
  before the gate reads, after which Session pending work blocks; named
  races are Enter→blur double paths, combobox-portal focusout, and
  programmatic unmount-without-blur. The registry entry shape accepts B
  owners later without redesign. Draft value, Pending edit, Edit preview
  and the new [Unresolved draft](../../../../GLOSSARY.md) term stay distinct.

### Later user amendment: sticky Pair action parameter (2026-11-07)

During [011 remediation](011-layout-explicit-submit-owners.md), the coordinator
asked: “Approve this narrow contract amendment? Keep Pair checked after Apply,
but advance its safety baseline only when that Apply succeeds. Failure/refusal
would still block, and later changes would block again.” The user answered:
“Approved. Retry a few times when gortex fails the continue”.

Preserve the original sticky checkbox and its mount-time initial baseline.
Only the Pair action parameter's safety baseline may subsequently advance, and
only from the submitted parameter of this binding's own successful Apply.
Failure/refusal, another admission key, an old owner's settlement, or a newer
unsent checkbox value never advances it. Later checkbox changes block again.
Mode, tweak and targets still use the live accepted projection at every poll,
not the last submitted composite. This exception grants no other owner's
action parameters a successful-submit baseline and changes no Apply policy.
The [historical adversarial check](../inventory/004-contract-check.md) remains
unchanged; this is a later explicit user decision, not its original conclusion.

### Alternatives considered

Extending `PendingEditSignals` alone (misses unbound composite forms, no
baseline at bind time), a presentation-owned global registry (no synchronous
Runtime poll path), inferring drafts from focus/DOM ARIA (cannot see valid
unsubmitted values; `PcbInput` cannot express `aria-invalid`), and forcing
blur-commit on explicit-submit editors (changes protected Apply/Save
semantics). A disposable sketch was waived by the user in favor of the
adversarial check.

### Change surface (for implementation planning, not done here)

`web/crates/runtime` (registry domicile, `update_safety_inputs`, blocker),
`web/crates/ui-shared/src/pending_edit_helpers.rs` (bound-field reporting),
per-owner adapters at the 14 Class A owners, deferral copy in
`web/src/presentation/notifications.rs`. Whether any refactoring of the
geometry-script typed draft earns its cost is
[Decide whether refactoring earns its cost](005-decide-whether-refactoring-earns-its-cost.md),
next. No ADR: the contract follows ADR-0022's existing adoption amendment
and reuses existing helper disciplines rather than reversing a decision.
No product code changed in this ticket.
