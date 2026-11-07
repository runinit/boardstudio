# 006: Agree the verification plan and implementation handoff

Status: resolved
Type: grilling
Label: wayfinder:grilling
Mode: HITL
Assignee: Chris D
Blocked by: [Decide whether refactoring earns its cost](005-decide-whether-refactoring-earns-its-cost.md)
Parent: [Draft-safe update adoption](../map.md)
Related: [Adopt the waiting build through protected Refresh](../../ui-integration-recovery/issues/019-adopt-build-safely.md)

## Question

Do the resolved owner/lifetime contract and refactor decision support an agreed,
implementation-ready specification and verification plan with no hidden decision
left before someone implements draft-safe Update Now?

## Decisions and handoff to assemble

Inputs are [Inventory project draft owners](001-inventory-project-draft-owners.md),
[Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md),
[Locate a deterministic packaged persistence hold](003-locate-packaged-persistence-hold.md),
[Choose the draft-safety interface and lifetime contract](004-choose-draft-safety-interface.md)
and [Decide whether refactoring earns its cost](005-decide-whether-refactoring-earns-its-cost.md).

- Link the authoritative owner census, real draft-loss observations, persistence-hold
  protocol, interface contract and refactor choice. Keep each decision's detail in its
  owning ticket; the implementation handoff indexes those answers rather than becoming
  a competing source of truth.
- Define RED/GREEN assertions for real valid explicit-submit and realistic invalid
  drafts, trusted pointer/Tab/Enter/Space entry points, Cancel value/focus preservation,
  explicit resolution, actual B loaded and exact project/value/revision reopened.
- Cover genuinely unanswered persistence and failure/abort, new drafts during takeover,
  hidden/multiple owners and old settlement after newer input. State precisely which
  native/mounted tests exercise shared behavior and which packaged checks remain
  indispensable. No injected invalid marker, save label or worker label as substitute.
- Name focused checks, mutation controls, later full gate/review requirements, evidence
  identity/log conventions, ownership, phase stopping points and remaining blockers.
  Match the parent acceptance requirements without claiming this map completes them.
- Resolve or explicitly scope any new concrete questions first; graduate genuine fog
  into child tickets rather than publishing a build-ready claim over an open decision.

## Resolution and stopping point

Use `grilling` + `domain-modeling`, with the human confirming the handoff is ready.
Record the final verification plan and linked specification/handoff asset in `## Answer`.
Do not implement product changes, run final acceptance as a substitute for planning,
close the adoption umbrella, or waive the separate worker/cache/Toast concerns.
The map ends when the route is clear; implementation needs a subsequent instruction.
Claim first. No child resolution is authorized merely by creating this ticket.

## Answer

Resolved 2026-11-07 by live grilling (three rounds; the user accepted all
recommendations Q1–Q11 and confirmed Q12: this handoff is implementation-ready
with no hidden decision left). This Answer **is** the implementation handoff: it
indexes the owning tickets below and adds only the cross-cutting verification
plan. It is not a competing source of truth, and it implements nothing.

### Authoritative inputs (detail lives in the owning ticket)

- [Inventory project draft owners](001-inventory-project-draft-owners.md) — owns
  the source-grounded draft-owner census.
- [Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md)
  — owns the real packaged A→B draft-loss observation, the failing regression
  assertion, and the header-entry-point and Layout-constraint follow-up cases.
- [Locate a deterministic packaged persistence hold](003-locate-packaged-persistence-hold.md)
  — owns the persistence-hold location, the named diagnostic-only
  gated-persistence adapter, and its packaged reuse protocol.
- [Choose the draft-safety interface and lifetime contract](004-choose-draft-safety-interface.md),
  as amended by [the 004 contract check](../inventory/004-contract-check.md) —
  owns the Runtime-owned registry contract, per-admission-key unresolved state,
  RAII lifetime, per-key label+remedy copy rule, all 14 Class A owners required,
  and the Class B deferral. Registry domicile is the Runtime: poll-prune drops
  dead entries (not blockers, one-time diagnostic); genuine unmount/scope change
  retires via RAII. Implementation names the diagnostic's log/surface as a
  checklist item, not a new decision.
- [Decide whether refactoring earns its cost](005-decide-whether-refactoring-earns-its-cost.md)
  — owns the no-refactor decision. Excluded from implementation scope; re-enters
  only via a fresh ticket with new both-benefits evidence.
- [Close the named census gaps](007-close-named-census-gaps.md) — owns closure of
  the census's named gaps and the final 14 Class A total.

### Mandatory RED/GREEN assertion inventory (no subsetting)

- Real valid explicit-submit drafts for all 14 Class A owners, and realistic
  invalid drafts with no submittable form, each deferring Update Now until the
  owner's own Cancel/Reset or submission resolves them.
- Trusted pointer, Tab, Enter and Space activation at both the toast and the
  header Update Now entry points; Cancel preserves the exact draft value and
  focus.
- Explicit resolution only: landing updates the baseline, failure/refusal never
  clears, revert-to-baseline clears, newer typing re-raises.
- After resolution and activation: actual build B loaded (loaded-document and
  served-binary identity) and the exact saved project reopened with the intended
  value at the intended revision.
- Genuinely unanswered persistence (held, no acknowledgement, no durable
  change), failure/abort and explicit retry; pending/failed saves never count
  as safe.
- New drafts during takeover, hidden/multiple owners, and old settlement after
  newer input never clearing the newer draft.
- No injected invalid marker, save label or worker label as a substitute for
  any of the above.

### Native/mounted versus packaged split

- Native/mounted tests exercise the shared behaviour: gate/registry logic,
  poll/prune, per-key unresolved and epoch rules, guard-drop lifetime, deferral
  copy, and hold/acknowledgement semantics. New-draft-during-takeover proof is
  mounted-primary.
- Packaged checks remain indispensable and cannot be replaced by mounted tests:
  real A→B loss/protection at both entry points with trusted interaction; the
  diagnostic-gated hold, release, fail and single-armed hold-then-fail legs
  (release-into-failure is one armed mode, not two re-armed gates) against real
  BrowserStore; one packaged new-draft-during-takeover leg reusing the Geometry
  scripts probe shape; sole-client/other-tab, offline and subpath legs;
  loaded-B identity; exact reopened project/value/revision.
- Exact diagnostic A/B build flags, build-identity stamping and cleanup
  attestation are implementation-phase checklist items owned by the adapter
  work; the adapter shape and window surface are already settled by
  [Locate a deterministic packaged persistence hold](003-locate-packaged-persistence-hold.md).

### Checks, mutation, gate, evidence, ownership, stopping points

- Focused loops during implementation: `python3 scripts/check.py quick`,
  `build` and `browser` as scoped by README; packaged A/B via the adoption
  driver named in the owning tickets.
- Mutation/rejection controls: `scripts/mutation-check.py` reruns over the
  registry poll/prune and per-key unresolved branches.
- Later closure obligations (not run by this ticket): full
  `python3 scripts/check.py` gate with receipt, plus one combined
  spec+standards review at implementation closure.
- Evidence: logs only under `.evidence/`; redirect long output there and report
  only results and relevant failures. No target directories, source copies or
  custom manifests in `.evidence/`.
- Ownership: the Runtime crate owns the registry and the diagnostic adapter;
  per-owner adapters land at the 14 Class A sites; deferral copy lives in the
  notifications presentation. The per-(owner, admission-key) label+remedy table
  is built and reviewed during implementation against the settled copy rule;
  literal strings are implementation content, not an open decision.
- Phase stopping points: report after RED, after mounted GREEN, and after
  packaged GREEN, and stop at each before proceeding.

### Risk acceptance, exclusions, remaining blockers

- Class B deferral remains an explicit, revocable risk acceptance. Revocation
  trigger: any observed Class B draft loss, any new Class B owner, or a
  blur-ordering counterexample (Enter→blur, portal focus-out,
  unmount-without-blur). Class B adapters are out of the initial implementation
  scope; the trigger re-opens them via a fresh ticket.
- No hidden decision remains: every candidate question surfaced while
  assembling this handoff was settled above as a handoff requirement or an
  implementation checklist item; no child ticket is graduated.
- Not waived and not blockers for implementation readiness: worker-restart/
  ADOPTER lifetime, navigating-client/cache races, and Toast title/display-text
  concerns remain separate adoption concerns. The adoption umbrella
  [Adopt the waiting build through protected Refresh](../../ui-integration-recovery/issues/019-adopt-build-safely.md)
  is not closed and its acceptance is not claimed by this map.
- Vocabulary: use *Draft value*, *Unresolved draft*, *Pending edit* and
  *Edit preview* as settled; never use unsaved/dirty state as a safety
  predicate, and never treat a `Saved locally` label or a completed preview as
  safe. No `GLOSSARY.md` change in this ticket.

No product code changed, no final acceptance was run in this ticket.
Implementation requires a subsequent instruction.
