# Spec: Draft-safe update adoption

Status: ready-for-agent
Source: [Draft-safe update adoption map](map.md), resolved by
[Agree the verification plan and implementation handoff](issues/006-agree-verification-and-implementation-handoff.md)
(user-confirmed implementation-ready, 2026-11-07).
Map: [map.md](map.md). Parent: [Adopt the waiting build through protected Refresh](../ui-integration-recovery/issues/019-adopt-build-safely.md).
Tickets: split into implementation tickets 008–015, listed in Further Notes; the wayfinder tickets below own the decisions.

## Problem Statement

A keyboard designer can lose work by accepting a waiting build. When a newer
build is waiting, Update Now reloads the page — but the update gate cannot see
most unsubmitted project drafts. A designer who has typed a valid value into a
form and not yet pressed its Apply or Save button can click Update Now, watch
the page reload onto the new build, and find the draft gone, with no warning
and no way back. This is not hypothetical: a packaged A→B probe confirmed a
valid unapplied Geometry scripts draft was lost exactly this way, even though
the app showed "Saved locally" and nothing was marked invalid.

The gate is blind because draft state lives inside each editor form. Only some
forms report anything at all, and the fallback signal (fields marked invalid
in the DOM) misses valid drafts and invalid drafts that were never submitted.
With fourteen explicit-submit draft owners across workspaces, setup and
dialogs, checking them one by one at the update prompt is not a reliable
alternative: hidden forms, forms with several submit actions, and drafts typed
while an update is being adopted all escape ad-hoc checks.

## Solution

Update Now asks every live draft owner, at the moment it is clicked, whether
it holds an unresolved draft. A Runtime-owned registry holds one registration
per live draft owner; both Update Now entry points (the update toast and the
header notice) poll it synchronously before adopting the waiting build. If any
owner reports an unresolved draft, adoption is deferred — never forced — with
a cancel-only message naming the editing area and how to resolve the draft.
The designer resolves the draft through that form's own Apply/Save or
Cancel/Reset controls, then chooses Update Now again. If no owner reports an
unresolved draft and no save is pending or failed, adoption proceeds as it
does today. Explicit Apply/Save semantics are unchanged: nothing is
auto-committed and nothing is discarded on the designer's behalf.

## User Stories

1. As a keyboard designer with a valid unapplied Geometry scripts draft, I want Update Now to defer and tell me to apply or cancel the script edit, so that my script text is never silently lost.
2. As a keyboard designer with an unapplied Layout constraint draft, I want the same deferral, so that constraint work survives my decision about the update.
3. As a keyboard designer editing any of the fourteen protected explicit-submit forms (Layout, Geometry, legacy position, companion placement, existing-half and mirrored-pair setup, matrix creation, and the Parts/PCB forms), I want my unapplied draft to block adoption, so that protection does not depend on which workspace I am in.
4. As a keyboard designer with a realistic invalid draft that cannot be submitted, I want Update Now to defer, so that work I still need to fix is not discarded for being invalid.
5. As a keyboard designer whose draft is invalid but unmarked in the DOM, I want deferral anyway, so that protection never depends on an invalid-field marker.
6. As a keyboard designer, I want the deferral message to name the editing area and its remedy (which Apply/Save or Cancel/Reset resolves it), so that I know exactly what to do next.
7. As a keyboard designer who sees the deferral while using only the mouse, I want to cancel the update with the pointer, so that my draft and focus stay where they were.
8. As a keyboard user, I want to reach and operate Update Now and its cancellation with Tab, Enter and Space at both the toast and the header entry points, so that keyboard operation is as safe as pointer operation.
9. As a keyboard designer cancelling a deferred update, I want my exact draft text and focus preserved, so that cancelling costs me nothing.
10. As a keyboard designer, I want only my own Apply/Save or Cancel/Reset on the owning form to clear the deferral, so that the update flow never force-submits or discards my work.
11. As a keyboard designer whose submission lands, I want the deferral to clear, so that I can then adopt the update without re-editing.
12. As a keyboard designer whose submission fails or is refused, I want the deferral to stay, so that failed work is not mistaken for resolved work.
13. As a keyboard designer who edits a draft back to its original value, I want the deferral to clear, so that I am not blocked by a draft that no longer differs.
14. As a keyboard designer who types more after a draft was submitted, I want the deferral to reappear, so that newer input is protected even when an older submission has just landed.
15. As a keyboard designer, I want an older submission's completion never to clear a newer draft, so that fast typing across an in-flight save is safe.
16. As a keyboard designer with drafts in two forms at once, I want Update Now to defer until both are resolved, so that resolving one never exposes the other.
17. As a keyboard designer with one form that has several submit actions (for example Save versus Place versus Matrix), I want the deferral judged per action, so that submitting one action does not clear a draft belonging to another.
18. As a keyboard designer whose draft form is hidden or kept mounted behind another workspace, I want it to still block adoption, so that out-of-sight work is not out of protection.
19. As a keyboard designer who closes a form or switches scope, I want its registration to retire, so that a gone form never blocks an update forever.
20. As a keyboard designer filling in a creation form (new matrix, mirrored pair, existing half), I want my entered values compared against the form's starting values, so that a half-filled creation blocks adoption like any other draft.
21. As a keyboard designer whose project save is still in flight when I choose Update Now, I want adoption deferred as save-in-progress, so that an unacknowledged save is never treated as safe.
22. As a keyboard designer whose save failed, I want adoption deferred with the failure respected and a working explicit retry, so that I can recover before updating.
23. As a keyboard designer who starts a new draft while an update adoption is waiting on a held save, I want that new draft to defer adoption too, so that work created during the takeover window is protected.
24. As a keyboard designer who resolves everything and chooses Update Now again, I want the new build to actually load, so that "updated" means the running app is build B.
25. As a keyboard designer reopening my project after the update, I want the exact project, value and revision I saved, so that the update provably preserved my work.
26. As a keyboard designer with no drafts and no pending save, I want Update Now to proceed without added friction, so that safety never punishes a clean state.
27. As a keyboard designer using disposable search, filter or navigation text, I want that text excluded from protection, so that I am never blocked by text that is not project work.
28. As a keyboard designer using blur-committing fields, I want today's commit-on-blur behaviour unchanged, so that this change does not alter fields that already commit as I leave them (their deferred protection is a recorded, revocable risk acceptance, re-opened if a loss is ever observed).
29. As a designer requesting reduced motion or using any Theme, Appearance or narrow layout, I want the deferral and its controls fully usable, so that safety is accessible in every presentation.
30. As a developer adding a new explicit-submit form, I want one registry to register its draft owner with, so that new forms become update-safe by construction rather than by remembering to patch the update gate.
31. As a developer, I want the gate to poll live owners synchronously at both checkpoints, so that there is no cached or event-ordered view of draft state to go stale.
32. As a developer, I want registrations to retire automatically when an owner unmounts or changes scope, so that owner lifetime cannot leak into a permanent block.
33. As a developer, I want unresolved state decided per admission key by owner-canonical comparison against the live-projected baseline or the mount-time default snapshot, so that equality means what the owning form means by "unchanged".
34. As a developer, I want dead registry entries pruned with a one-time diagnostic rather than treated as blockers, so that a retired owner is observable but never a false deferral.
35. As a developer, I want to prove the shared behaviour in native/mounted tests and reserve packaged runs for what only a real build can prove, so that the test suite stays fast without weakening the safety claim.
36. As a developer verifying packaged builds, I want a diagnostic-only way to hold, release, fail and hold-then-fail a packaged save, so that genuinely unanswered persistence can be proven in the packaged app and never shipped as product behaviour.
37. As a maintainer, I want the confirmed packaged draft-loss scenario as a permanent failing-then-passing regression assertion, so that this loss cannot silently return.
38. As a maintainer, I want mutation controls over the registry polling, pruning and per-key unresolved branches, so that a weakened gate is caught by tests rather than by users.

## Implementation Decisions

- **Decision authority.** Every decision below is already settled by the
  wayfinder map; this spec synthesizes, it does not re-decide. Detail lives in
  the owning tickets, indexed here: the owner census in
  [Inventory project draft owners](issues/001-inventory-project-draft-owners.md)
  and [Close the named census gaps](issues/007-close-named-census-gaps.md)
  (fourteen Class A explicit-submit owners; blur-committing Class B excluded);
  the observed loss in
  [Establish a real-editor draft-loss probe](issues/002-establish-real-editor-draft-loss-probe.md);
  the persistence hold in
  [Locate a deterministic packaged persistence hold](issues/003-locate-packaged-persistence-hold.md);
  the interface contract in
  [Choose the draft-safety interface and lifetime contract](issues/004-choose-draft-safety-interface.md)
  as amended by the contract check in the plan inventory; the no-refactor
  decision in
  [Decide whether refactoring earns its cost](issues/005-decide-whether-refactoring-earns-its-cost.md);
  and the verification plan in
  [Agree the verification plan and implementation handoff](issues/006-agree-verification-and-implementation-handoff.md).
  Where this spec and an owning ticket disagree, the owning ticket wins.
- **Runtime-owned registry.** A registry of live draft owners lives in the
  Runtime and is polled synchronously by the Runtime's update-safety inputs at
  both Update Now checkpoints (toast and header). No event-fed cache, no
  DOM-derived signal, and no save-status label substitutes for the poll. The
  existing DOM invalid-field fallback stays additive-only.
- **Owner registration is RAII.** Each live draft owner registers on mount and
  its registration retires on genuine unmount or scope change. Hidden,
  keep-mounted and portal-retained owners remain registered and keep blocking.
  Poll-prune drops dead entries rather than treating them as blockers, and
  emits a one-time diagnostic; the diagnostic's log/surface is named during
  implementation.
- **Unresolved is per admission key.** Each owner's state is judged per
  admission key (each distinct submit action is its own key; one owner may
  carry several keys, and one action may settle several keys). An owner is
  unresolved for a key when its current Draft value differs, by the owner's
  own canonical equality, from the baseline for that key — the live-projected
  accepted value where one exists, otherwise the snapshot of defaults taken at
  mount (creation forms). A submitted value is remembered per key; an older
  settlement never clears newer input, and reverting to the baseline clears
  the key.
- **Deferral behaviour.** Any unresolved key defers adoption. The deferral is
  cancel-only, names the editing area and the per-key remedy, preserves draft
  values and focus, and never force-submits or discards. Resolution happens
  only through the owning form's own Apply/Save or Cancel/Reset. Successful
  landing updates the baseline; failure or refusal never clears; newer typing
  re-raises.
- **Copy.** The per-(owner, admission-key) label and remedy table is built
  during implementation against the contract's copy rule and reviewed at the
  review gate; the literal strings are implementation content, not an open
  decision.
- **All fourteen Class A owners are required.** Each gains a registry adapter
  reporting its unresolved keys. Adding the missing owner reports is product
  behaviour, delivered with its own RED/GREEN proof per owner cluster.
- **Class B deferral is a recorded risk acceptance.** Blur-committing owners
  are not adapted in this effort. The acceptance is revoked — and Class B
  re-opens via a fresh ticket — by any observed Class B draft loss, any new
  Class B owner, or a blur-ordering counterexample (Enter→blur, portal
  focus-out, unmount-without-blur).
- **No refactoring.** The Geometry scripts editor's JSON-encoded draft stays
  as it is; its registry adapter compares the live encoded draft against the
  owner-canonically encoded baseline per admission key. The parked typed
  conversion re-enters only via a fresh ticket carrying new evidence of both
  reduced testing effort and improved code quality — never opportunistically
  during this implementation.
- **Packaged persistence diagnostic.** Packaged verification uses the named
  diagnostic-only gated-persistence adapter at the existing persistence port,
  supporting hold, release, fail and hold-then-fail as a single armed mode
  (release-into-failure is atomic, not two re-armed gates), against the real
  packaged store. It is build-gated as diagnostic-only and never ships as
  product behaviour. Exact diagnostic A/B build flags, build-identity
  stamping and cleanup attestation are implementation checklist items for the
  adapter ticket. Genuinely unanswered persistence is judged by the real
  store's completion/abort acknowledgements, not request-success callbacks,
  save labels or worker labels.
- **Adoption identity.** "Updated" is proven by the loaded document and the
  served binary both identifying build B, with the exact saved project
  reopened at the intended value and revision — not by manifests, labels or
  server responses.
- **Domain language.** Use *Draft value*, *Unresolved draft*, *Pending edit*
  and *Edit preview* as settled in the glossary and the interface contract.
  Unsaved/dirty state is never a safety predicate; a "Saved locally" label or
  a completed Edit preview is never evidence that a Draft value is safe.
  Disposable search, filter and navigation text is not a Draft value.
- **Scope discipline.** Builds are serialized per target; browser probes run
  only on disposable fixtures, scopes and databases, never on the user's live
  preview or project, and are cleaned up afterwards. Unrelated dirty-worktree
  work is preserved.

## Testing Decisions

- **What a good test is here.** Tests assert externally visible behaviour —
  deferral or adoption, the named area and remedy, preserved draft text and
  focus, the actually loaded build, and the reopened project/value/revision —
  through the highest available seam. They never assert registry internals as
  a substitute for behaviour, never inject an invalid-field marker, and never
  accept a save label or worker label as proof of safety. Targeted unit and
  mutation coverage of the polling, pruning and per-key branches complements,
  and does not replace, the behavioural assertions.
- **Seam S1 — protected Update Now (native/mounted, primary).** Both entry
  points driven through the real Runtime safety checkpoint with live owners
  mounted. This seam carries: every Class A owner's valid-draft deferral;
  realistic invalid drafts, including ones never submitted and unmarked in the
  DOM; per-key judgement across multiple submit actions; hidden, keep-mounted
  and multiple simultaneous owners; registration retirement on unmount and
  scope change; explicit resolution (landing, failure/refusal, revert-to-
  baseline, newer typing re-raising, older settlement never clearing newer
  input); new drafts during takeover as the mounted-primary proof; Cancel
  preserving exact value and focus; trusted Tab/Enter/Space operation; and
  hold/failure semantics against controllable persistence. Prior art: the
  existing mounted update-notification safety tests and the mounted
  held-save tests named in the verification plan.
- **Seam S2 — packaged A→B (real packaged browser, indispensable).** Real
  packaged build A with a waiting build B, exercising: the confirmed Geometry
  scripts draft-loss scenario as the tracer regression (RED before the fix at
  the toast entry point, GREEN after), the header entry point after toast
  dismissal, and the Layout constraint owner; the diagnostic-gated hold,
  release, fail and single-armed hold-then-fail legs against the real packaged
  store, consumed again after takeover in build B; exactly one packaged
  new-draft-during-takeover leg reusing the tracer scenario's shape;
  sole-client/other-tab, offline-asset continuity and subpath-isolation legs
  inherited from the parent adoption acceptance; failed-install and
  no-new-build negative controls; and loaded-B identity with exact
  project/value/revision reopened. Prior art: the packaged probe and A/B
  driver established by the draft-loss probe ticket, and the parent adoption
  ticket's packaged legs. No mounted result substitutes for any of these.
- **RED/GREEN phasing with stopping points.** Work stops and reports after
  RED (the tracer regression and mounted assertions failing for the expected
  reason), after mounted GREEN at seam S1, and after packaged GREEN at seam
  S2, proceeding only on instruction at each point.
- **Checks and gate.** Focused loops during implementation use the project's
  quick/build/browser check scopes. Mutation and rejection controls rerun
  over the registry polling, pruning and per-key unresolved branches using
  the project's mutation-check tooling. Closure requires the full project
  check gate with its receipt, plus one combined spec-and-standards review.
  Long output is redirected to the project's evidence log directory and only
  results and relevant failures are reported; no source copies, target
  directories or custom manifests are placed there.
- **Evidence identity.** Every packaged leg records the A and B build
  identities and distinguishes loaded-app identity from served or labelled
  identity; the packaged diff between A and B is the update mechanism itself,
  not hand-edited content.

## Out of Scope

- Product behaviour beyond draft-safe Update Now: no change to explicit
  Apply/Save semantics, no auto-commit, no discard path, no automatic reload.
- Class B (blur-committing) owner adapters, until the recorded revocation
  trigger fires and a fresh ticket re-opens them.
- The typed conversion of the Geometry scripts draft, and any other
  refactoring that has not passed the both-benefits bar in a fresh ticket.
- Worker-restart/ADOPTER lifetime and navigating-client/cache races; Toast
  title/display-text matching; build identity display; ordinary delayed
  worker activation and cache policy. These remain separate concerns of the
  parent adoption effort and are not waived by this spec.
- Closing the parent adoption umbrella, final safety acceptance of the
  adoption feature as a whole, commits, pushes and merges — each requires its
  own later instruction.
- Disposable search, filter and navigation text protection.

## Further Notes

- This spec is a synthesis of a completed wayfinder map: the route is clear
  and no product decision is left open. The next step is splitting it into
  tracer-bullet implementation tickets with blocking edges (for example with
  the `to-tickets` skill) in a subsequent session; do not implement directly
  from this spec in the same breath.
- The natural tracer bullet is the confirmed Geometry scripts packaged loss:
  registry plus one owner adapter plus both checkpoints proves the whole
  contract end-to-end before the remaining owner clusters land.
- Implementation tickets (work the frontier; blockers first):
  [008 Registry and Geometry scripts tracer](issues/008-registry-and-geometry-tracer.md) and
  [009 Diagnostic gated-persistence adapter](issues/009-diagnostic-gated-persistence-adapter.md)
  can start immediately; [010 Packaged A→B proof](issues/010-packaged-a-to-b-proof.md) follows them;
  [011 Layout owners](issues/011-layout-explicit-submit-owners.md),
  [012 Parts owners](issues/012-parts-single-draft-owners.md),
  [013 PCB owners](issues/013-pcb-placement-attach-assembly-net-owners.md) and
  [014 Creation-form owners](issues/014-creation-form-owners.md) follow 008;
  [015 Coverage, mutation and closure](issues/015-coverage-mutation-and-closure.md) closes the spec.
- Ready-for-agent means the specification is buildable without further
  decisions; it is not feature completion, acceptance, or closure of the
  parent adoption ticket.
