# 017: Build update notification specification

Status: resolved
Type: planning
Owner: fix/ui-integration-recovery
Blocked by: none for planning
Parent: [013: Build update umbrella](013-surface-build-updates.md)
Related: [Plan map](../map.md)

## Problem Statement

A keyboard designer can keep editing a cached build without knowing a newer one
is waiting. [013](013-surface-build-updates.md) records the user's stale-build
report and 2026-11-06 choice of an **Update Available** notification with a refresh
button. Delayed service-worker activation is a possible explanation, not the
established cause of that incident; no reproduction was captured.

## Solution

Notify the user when an installed update is waiting without interrupting their
Editor session. Offer Refresh to adopt it safely while preserving local work
and other open editors' access to required assets.

At specification time, persistence until dismissal/adoption, protection of pending
or unsaved work and optional running version/time display were 013's working
assumptions. The user's 2026-11-06 answers below supersede them where they differ;
013's 2026-11-08 reconciliation records the delivered behavior.

## User Stories

1. As a keyboard designer, I want Update Available when a newer build is waiting, so that I know my current build is no longer the latest available one.
2. As a keyboard designer, I want editing to continue when the notice appears, so that discovery does not interrupt my work.
3. As a keyboard designer, I want Refresh in the notice, so that I can deliberately adopt the waiting build.
4. As a keyboard designer, I want the notice to remain until dismissed or adopted, so that a short-lived message does not hide the update.
5. As a keyboard designer, I want dismissal without refresh, so that I can continue the current Editor session.
6. As a keyboard designer with a Draft value, I want warning and cancellation before refresh, so that typed work is not silently lost.
7. As a keyboard designer with a Pending edit or incomplete save, I want refresh to preserve my work, so that adoption does not race persistence.
8. As a keyboard designer whose save failed, I want refresh to respect that failure, so that it does not imply my work is safe.
9. As a keyboard designer cancelling refresh, I want my editor and Draft values intact, so that cancellation is meaningful.
10. As a keyboard designer with another editor open, I want its offline assets preserved, so that updating one page does not break another.
11. As a keyboard user, I want Refresh and dismissal reachable without stolen focus, so that discovery fits my editing workflow.
12. As a user requesting reduced motion, I want usable controls without required animation, so that updates remain accessible.
13. As a keyboard designer reopening after refresh, I want my saved project intact and the new build actually running, so that Refresh means adoption.
14. As a keyboard designer, I want any displayed identity to distinguish the running build from the waiting one, so that diagnostics are accurate. Display remains optional under 013's nearly-free condition.

## Implementation Decisions

- **Authority:** 013 owns the notification/Refresh choice. Its assumptions remain
  overridable. No automatic reload, discard-work approval, new Settings page or
  periodic polling product policy is introduced.
- **Ownership:** host observes real registration; application owns update state
  and refresh safety; shared controls own presentation. Keep authored application
  policy in Rust and project edit policy outside visual primitives (S1–S4 below).
- **Discovery:** handle an already-waiting registration and installation finishing
  during an Editor session. Initial install alone is not an update. Installing,
  failed or unknown state must not claim readiness. Workspace changes and
  re-renders must not duplicate one waiting notification.
- **Presentation:** reuse shell Toast placement, shared Button styling, semantic
  Theme roles and non-modal announcement behavior. A permanent actionable update
  Toast narrowly extends the transient-result pattern; it does not move existing
  persistent errors into Toasts. Document this distinction with implementation.
- **Protection:** evaluate Draft values, Pending edits and local-save completion
  when Refresh is activated, not only when the notice appears. Warn and allow
  cancellation before reload. Acknowledging a warning does not authorize losing
  work. Identify actual owners and prove preservation or defer with an explanation;
  the header's Saved label alone cannot establish that every draft is safe.
- **Activation:** ordinary install stays delayed. Opt-in adoption must preserve
  assets needed by other open editors. Current activation deletes older scoped
  caches, so simply forcing that activation is insufficient. Select and document
  a safe mechanism during implementation; add a dated ADR-0022 amendment if the
  policy is refined. Ordinary reload has not been proven to adopt a waiting worker.
- **Identity:** generated release identity includes a digest prefix and generation
  time, not a semantic application version or Git revision. Add running version/time
  only if nearly free and bound to the loaded page rather than the newest server
  response. Otherwise record omission under 013's assumption.

## Testing Decisions

Use the highest existing production boundary: mounted shell/Toast tests for UI
and refresh gating, plus a real packaged browser for worker lifecycle. Mocked
waiting state cannot prove installation, cache continuity or adoption.

- **Mounted seam (S5–S7):** assert one named Update Available announcement,
  persistence past normal Toast expiry, dismissal without reload, no focus steal,
  no duplicate on Workspace changes, initial-install/failure negative controls
  and observer cleanup on unmount. Drive the production bridge and controls.
- **Refresh seam (S8):** use real application action/state owners with controlled
  pending/failing persistence and representative draft-owning fields. Cancel must
  retain values and editor; pending/failed work must not reload; safe adoption
  must preserve the saved project. Inventory draft owners during implementation;
  do not infer coverage from a save-status label.
- **Accessibility (S2, S6, S7):** extend atomic status and dismissal coverage with
  real Tab/Enter/Space operation of Refresh, cancellation focus and reduced motion.
  Verify readable/reachable actions across Themes, Appearances and narrow layouts.
  Use role/name helpers and rendered behavior, not selector/source-string evidence.
- **Real old-then-new scenario:** use a dedicated test origin/profile, package
  identifiable build A, open a populated saved project and establish A's real
  worker control/cache. Keep the editor open, replace the served release with B at
  the same scope, cause a real update check and observe completed installation
  and waiting state. Assert the production notification. Exercise cancellation
  with a Draft value and pending save; settle/save work and activate Refresh.
  Assert the loaded application is B, not merely the server manifest or worker
  label, and the saved project reopens unchanged without advertising B again.
- **Continuity:** repeat with a second A editor open. While offline, request an
  A asset that is cached but not yet used by that editor and verify availability
  through the chosen adoption/defer behavior. Cover root/subpath isolation,
  failed installation and no-new-release controls. Never unregister workers or
  clear caches between A and B; that removes the condition being tested.
- **RED/GREEN:** retain commands, source revisions, A/B identities and assertion
  logs showing failure before the relevant change and success after it, using the
  same scenario. Setup failures, missing tests, mocked worker events and changed
  labels are not RED evidence. Store logs in .evidence/; Git retains source.
  No copied source trees or custom manifests. An unavailable real deployment
  fixture is a blocker, not permission to substitute a mounted-only pass.
- Run focused mounted/shared-control checks and the packaged scenario plus
  python3 scripts/check.py quick for touched crates. Record exact invocations
  and counts in completion comments. Historical planning-task limit: no builds/browser
  tests ran in the 2026-11-06 planning task and it claimed no RED/GREEN. Delivered
  proof is now indexed in the Answer below. 010/011 gain no blockers.

## Out of Scope

Historical stale-build diagnosis; Core/geometry fixes; automatic reload;
version-history or Settings pages; remote update services; forced loss of Draft
values; general offline redesign; uncached remote assets; browser-support
certification; changes to tickets 001–012 or their acceptance scope.

## Further Notes

### Inspected sources and evidence limits

- **S1:** [013](013-surface-build-updates.md) supplies choices/assumptions;
  [ADR-0022](../../../adr/0022-static-local-first-rust-application-with-worker-execution.md)
  requires continuity for open editors and Rust-owned local-first operation.
  [Architecture](../../../architecture.md) describes host/runtime/presentation
  ownership and root/subpath assets with isolation headers.
- **S2:** [PRODUCT](../../../../PRODUCT.md) describes local persistence and offline
  use. [DESIGN](../../../../DESIGN.md) describes transient Toasts, inline persistent
  reports, shared controls, Themes and reduced motion.
  [Glossary](../../../../GLOSSARY.md) distinguishes Editor session, Draft value,
  Pending edit, Accepted document, Theme, Appearance and Workspace.
- **S3:** [Worker](../../../../web/src/service_worker.rs) caches manifest assets at
  install and waits; activation deletes other matching scoped caches and claims
  clients; fetch prefers its current cache including the shell. **2026-11-08
  superseded observation:** the original "storage-window queries, not update adoption"
  description predates 019. The handler now accepts `ADOPT_WAITING_MESSAGE` and
  authoritatively gates opt-in adoption on the requesting page being its sole
  window client, with guarded cleanup; ordinary delayed activation is unchanged.
  See 019 and the dated ADR-0022 amendments for proof and residual risks.
  [Manifest policy](../../../../web/crates/host/src/offline.rs) validates versions
  and normalized assets and isolates cache names by scope path.
- **S4:** [Registration](../../../../web/crates/host/src/host/offline.rs) returns a
  registration. [Runtime initialization](../../../../web/crates/runtime/src/runtime.rs)
  reports failure without retaining the successful registration there, and
  excludes registration in test/test-support configurations. Mounted tests alone
  therefore cannot establish the real release lifecycle.
- **S5:** [Notification bridge/tests](../../../../web/src/presentation/notifications.rs)
  drain transient results once, keep reports inline and test expiry/unmount.
- **S6:** [Toast wrapper](../../../../web/crates/ui-shared/src/ui/toast.rs) forwards
  permanence, defaults to five-second expiry and renders title/description/close.
  **2026-11-08 superseded observation:** "no Refresh/action slot" was the original
  inspected-source limit; 019 adds `ToastActionSlot` for the reachable Update Now
  action. [Mounted tests](../../../../web/crates/ui-shared/src/toolbar_toggle_slider_toast_tests.rs)
  now cover the action as well as atomic status and dismissal; packaged keyboard
  proof is recorded in 019.
- **S7:** [Keyboard scenario](../../../../web/crates/ui-shared/ticket-008-keyboard.py)
  covers retained focus, Tab, Enter/Space dismissal and Escape not being captured.
- **S8:** [Header](../../../../web/src/presentation/header.rs) distinguishes local
  saved/saving/failed/unavailable states. **2026-11-08 superseded observation:**
  the original unverified inventory/multi-tab/deployed-behavior limit is discharged
  by draft-safety 008–015's census and Runtime registry (14 Class A owners /
  17 registrations), and 019's packaged second-editor/offline/registration-scope
  and adoption proof. Class B is revocable risk acceptance; worker-restart and
  navigating-client/cache races remain explicit risks, not verified-safe claims.
- **S9:** [Release builder](../../../../scripts/build-web.py), embed_offline_worker,
  inventories route assets and embeds version/assets in the generated bootstrap.
  Version contains a content digest prefix and time.time_ns(). A separate readable
  timestamp and loaded-page identity binding were not verified.

The original entries are planning-time source observations, not current passing
test results. The dated superseding notes and Answer point to delivered proof;
unchanged historical observations are not a fresh source audit.

### Open questions, not new decisions

1. After dismissal, should the same waiting release reappear next Editor session
   or page load? 013 does not define dismissal scope.
2. When another editor prevents safe adoption, what explanation/next action should
   Refresh offer? Preserve ADR-0022 meanwhile; never silently force other tabs.
3. For Draft values that cannot yet be saved, is deferring refresh until the user
   resolves them the desired interaction? No discard permission is recorded.
4. If running version/time display requires material plumbing, omit it under the
   nearly-free assumption or request explicit scope expansion?

These do not block notification work. Record a precise blocker if adoption needs
a new product choice rather than inventing one.

**Answered by the user, 2026-11-06** (the questions above are kept as asked):

1. The Toast is dismissible and carries "Update Now" as clickable text. After
   dismissal the header bar shows the update notification, with a working "Update
   Now", and keeps it while the update waits. The Toast itself reappears on the next
   page load (confirmed by the user); it is derived from the real waiting worker on
   each load and nothing about dismissal is stored. Slice placement: the Toast is 018,
   the action and the header notice are 019.
2. Show something like "Please close any other open tabs"; never force other tabs.
3. Yes: defer adoption until unsaved drafts are resolved.
4. Drop the version/time display if it needs plumbing.

## Comments

- 2026-11-06: Documentation-only spec drafting synthesis. The user explicitly requested
  publication and breakdown; source citations are included as requested.
  Ready for agent work is not feature completion or product acceptance.

## Completion record (coordinator, 2026-11-08)

HEAD `118e60f1`; Status `resolved`. Executed in this pass: documentation reconciliation,
doc-links PASS (719; `.evidence/ui-integration-recovery/019-doc-reconciliation-links.log`),
`git diff --check` PASS and final diff read; no product tests or new full gate claimed.
Reused: reviewed 018/019 and
[draft-safety 008–015](../../update-draft-safety/map.md#decisions-so-far), especially
[010 packaged proof](../../update-draft-safety/issues/010-packaged-a-to-b-proof.md#answer)
and [015 closure](../../update-draft-safety/issues/015-coverage-mutation-and-closure.md#answer).
[019's completion record](019-adopt-build-safely.md#completion-record-coordinator-2026-11-08)
records the fixed combined-review endpoints, final delta PASS/READY, combined AC7
proof, failed/flake runs, reused full gate and owned cleanup. No unmet gate or
retained cleanup for 017; ADR residual risks and the exact Layout Space-to-pan
product question remain, outside this closure. Next: parent plan 020/021; no change
to parent 001 or UI recovery 010/011 acceptance.

## Answer

Stories and seams are discharged by [018 notification](018-notify-waiting-build.md),
[019 protected adoption](019-adopt-build-safely.md) and draft-safety 008–015:
notification/persistence/dismissal (stories 1–5), draft/pending-save/failure/Cancel
protection (6–9), packaged other-editor/offline continuity (10), keyboard and
reduced-motion controls (11–12), and loaded B plus exact saved-project reopening
(13). Draft-safety 010/015 packaged receipts and 019's combined AC7 proof are the
real-boundary evidence, not source-presence assertions. Story 14's optional running
identity/time was omitted under the user's answer. S3/S6/S8 above mark superseded
source limits explicitly. ADR-0022 amendments 2026-11-07/08 record the refinements
and retained risks. 017 is resolved; historical stale-build causation remains unproven.
