# Map: Draft-safe update adoption

Label: planning:map
Spec: [spec.md](spec.md) (draft-safe update adoption, ready)
Related: [Adopt the waiting build through protected Refresh](../ui-integration-recovery/issues/019-adopt-build-safely.md)

## Destination

An agreed, implementation-ready draft-safety specification and verification plan for
protected Update Now: which project drafts must be protected, the owner-state
interface and lifetime rules, justified refactoring if any, and meaningful acceptance
proof. Reaching this destination does not implement the fix or close the adoption ticket.

## Notes

- **Planning only.** Child questions resolve facts or decisions needed for the
  specification, not product-build slices. Disposable diagnostic probes and narrowly
  scoped test-only fixtures may unblock a decision; production behavior changes,
  final acceptance runs, commits, pushes and merges require later authorization.
- **Approved charter (user, 2026-11-07):** cover all project-editing drafts across
  workspaces, setup and dialogs, not only the two confirmed source gaps. Disposable
  search/filter/navigation text is excluded. Require real-editor trusted interaction,
  packaged A-to-B adoption, genuinely held persistence acknowledgment and new drafts
  during takeover. Refactoring earns inclusion only if evidence shows both reduced
  testing effort and improved code quality; it must not remove the required safety
  proof. These are scope constraints, not resolutions of child questions.
- **Existing decisions stay with their owners.** Unresolved drafts defer, never
  discard; preserve explicit Apply/Save semantics. See [the user decisions](../ui-integration-recovery/issues/013-surface-build-updates.md#later-answers-user-2026-11-06)
  and [the adoption amendment](../../adr/0022-static-local-first-rust-application-with-worker-execution.md#amendment-opt-in-adoption-of-a-waiting-build-2026-11-07).
- **Evidence starts here:** [Draft-safety research and change map](../../research/2026-11-07-019-draft-safety-change-map.md).
  It confirms source-level detection gaps in the Layout constraint and Geometry
  scripts forms, not observed A-to-B loss or complete editor coverage. Reuse it;
  current dirty source wins over retained logs or a clean checkout.
- **Domain:** use [Draft value, Pending edit and Edit preview](../../../GLOSSARY.md#L411-L439)
  distinctly. A completed preview or a Saved locally label is not all-draft safety.
- **Guides:** `questioning` + `domain modelling` for human decisions; `source search`
  for source evidence; `module design` for the interface/refactor choices;
  `agent-browser` for browser probes; `test-first development` and `rust handbook` if a diagnostic
  fixture needs Rust; `the refactoring tool` only if a later approved refactor proceeds.
- **Source/ownership:** continue in the dirty `fix/ui-integration-recovery` worktree;
  preserve unrelated edits and the existing recovery map, tickets, ADR and research.
  Native Gortex is mandatory; no tracking, daemon changes or shell source fallback.
  Run probes only on disposable fixtures/scopes, never the user's live preview or
  project. Serialize builds per target and coordinate browser ownership before probes.
- **Tracker adaptation:** [committed Markdown tracker](../../agents/issue-tracker.md#planning-map-operations),
  with only `planning:*` labels. This planning effort uses `Status: open`, `claimed`
  or `resolved` as workflow states, not status roles. `Type:` and `Mode:` identify
  task/questioning and AFK/HITL; `Assignee: none` is unclaimed. Set both `Status: claimed`
  and the driving developer's `Assignee:` before work. On resolution append `## Answer`
  and supporting asset links to the child, set resolved, then add its gist here.
- **Frontier query:** use native Gortex file search for
  `docs/plans/update-draft-safety/issues/`, read headers, and take open, unclaimed
  children whose `Blocked by:` tickets are all resolved, lowest number first.
  Open children live in that query, not a duplicated list on this map. Dependencies
  use the tracker's `Blocked by:` links. One non-research question per work session.
- **Charting stop:** no child is claimed or resolved during map creation. All current
  knowledge collection is local, so no new external-research tickets or agents are
  needed; add one only when an actual outside-source question emerges.

## Decisions so far

- [Inventory project draft owners](issues/001-inventory-project-draft-owners.md): source census (asset: [001 census](inventory/001-draft-owner-census.md)) finds 12 explicit-submit owners invisible to the gate — the Layout constraint and Geometry scripts gaps plus legacy position, companion placement, and seven Parts/PCB forms, with the New matrix creation form's protected-set classification left to the interface question; 22 blur-committing owners carry ordering risk only, and neither `PendingEditSignals` nor `OwnedEdits` exposes an unsubmitted draft. Named unread regions/lifetimes graduated to [Close the named census gaps](issues/007-close-named-census-gaps.md), which now also blocks the interface question.
- [Establish a real-editor draft-loss probe](issues/002-establish-real-editor-draft-loss-probe.md): packaged A→B probe confirms real loss — a valid unapplied Geometry scripts draft (settled `Saved locally`, no `aria-invalid`) did not defer toast Update Now; the page reloaded onto verified build B and the reopened script held only the applied baseline. Names the failing regression assertion plus header-entry-point and Layout-constraint follow-up cases for the verification plan.
- [Close the named census gaps](issues/007-close-named-census-gaps.md): every census-named unread region/lifetime is now source-classified — two new explicit-submit owners promoted (existing-half setup, mirrored-pair creation form; 14 Class A total), accepted updates confirmed to clobber an open Layout constraint draft and Geometry script switching confirmed unguarded, and only browser-dependent questions remain open.
- [Choose the draft-safety interface and lifetime contract](issues/004-choose-draft-safety-interface.md): a Runtime-owned registry of live draft owners, polled synchronously by both Update Now checkpoints, with per-admission-key unresolved state (live-projected baseline or mount-default snapshot, owner-canonical equality), RAII registration retiring on genuine unmount/scope change, per-key label+remedy copy, all 14 Class A owners required, and Class B deferral recorded as an explicit revocable risk acceptance; contract survived a fresh-context adversarial check with five one-sentence amendments.
- [Decide whether refactoring earns its cost](issues/005-decide-whether-refactoring-earns-its-cost.md): no refactor — `GeometryScriptsEditor`'s JSON-encoded draft stays (typed conversion shows a modest code-quality gain but zero reduced testing effort, failing the both-benefits bar); the missing owner report is implementation-phase product behavior, safety proof unchanged, and the opportunity is parked in the master refactoring-opportunities document pending fresh both-benefits evidence.
- [Locate a deterministic packaged persistence hold](issues/003-locate-packaged-persistence-hold.md): the existing `gate_next_persist` hold is pre-transaction at the `DocumentPersistence` port (real BrowserStore target, demonstrated entered/`Saving`/release/`Saved` in fresh browser tests) but is compiled out of packaged builds and in-process-only, so packaged verification needs the named diagnostic-only gated-persistence adapter and reuse protocol the ticket specifies; no packaged held-save safety is claimed yet.
- [Agree the verification plan and implementation transfer](issues/006-agree-verification-and-implementation-transfer.md): user-confirmed implementation-ready transfer — the Answer indexes all owning decisions and fixes the mandatory RED/GREEN inventory, native/mounted vs packaged split, checks/mutation/full-gate/evidence obligations, ownership and RED → mounted GREEN → packaged GREEN stopping points, with the Class B revocation trigger recorded and no hidden decision left; implementation still needs a subsequent instruction.

- **2026-11-07 later Pair amendment:** [004 Answer](issues/004-choose-draft-safety-interface.md#later-user-amendment-sticky-pair-action-parameter-2026-11-07) records explicit user approval to preserve sticky Pair after Apply and advance only that binding's Pair safety baseline from its own successfully submitted parameter. Failure/refusal and later changes still block; all accepted fields remain live-projected per poll. No generalized action-parameter exception.

- **2026-11-07 authorized implementation checkpoint:** [008 registry/tracer](issues/008-registry-and-geometry-tracer.md#answer), [009 diagnostic adapter](issues/009-diagnostic-gated-persistence-adapter.md#answer), [011 Layout owners](issues/011-layout-explicit-submit-owners.md#answer) and [010 packaged A→B proof](issues/010-packaged-a-to-b-proof.md#answer) are reviewed and coordinator-accepted, including literal Retry, held-save/newer-input timing and final owned profile/database/output cleanup. The later implementation authorization and next 012→013→014→015 order live in [execution](execution.md); all-14/optimized/mutation/full-gate/final-review obligations and umbrella 019 remain open.

- **2026-11-08 012 closure:** [012 Parts owners](issues/012-parts-single-draft-owners.md#answer)
  reviewed (Spec/Standards PASS, full coverage, no Required findings) and
  coordinator-accepted — 25 mounted tests, full parts 97/97. Next is the
  [013 human origin-baseline decision](issues/013-pcb-placement-attach-assembly-net-owners.md#decision-required-before-implementation),
  then 014; see [execution](execution.md).

- **2026-11-08 015 closure:** [015 coverage/mutation/closure](issues/015-coverage-mutation-and-closure.md#answer)
  reviewed (combined Spec PASS / Standards resolved, 14/14 owners, 6+1 mutations killed,
  full gate passed) and coordinator-accepted — verification closure for draft-safe Update Now.
  At that checkpoint: coordinator commit, then parent 019; umbrella not closed by 015.
- **2026-11-08 parent 019 closure:** [019 protected adoption](../ui-integration-recovery/issues/019-adopt-build-safely.md#answer)
  reviewed and resolved at `118e60f1` (Spec PASS / Standards PASS, READY), including
  combined packaged draft + held-save proof. Its completion record retains failed-run
  receipts and ADR residual risks; 013/017 reconciled in the same documentation pass.

## Not yet specified

- (Graduated 2026-11-07: the census's named unread regions and untraced lifetimes
  became [Close the named census gaps](issues/007-close-named-census-gaps.md).)
- Further diagnostic questions if the real browser or persistence probes expose a
  materially different failure path or instrumentation limit. Do not preselect a
  workaround, global registry or shipping test hook before that evidence exists.

## Out of scope

- Product implementation, broad cleanup, final safety acceptance or closure of
  [Adopt the waiting build through protected Refresh](../ui-integration-recovery/issues/019-adopt-build-safely.md).
- Worker-restart/ADOPTER lifetime and navigating-client/cache races; these remain
  separate unresolved adoption concerns, not waived by this map.
- Toast title/display-text matching, build identity display, and reworking ordinary
  delayed worker activation or cache policy.
- Reopening the defer-versus-discard decision, changing explicit Apply into automatic
  commit, or reducing required acceptance proof to meet the refactor condition.
