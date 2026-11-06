# 019: Adopt the waiting build through protected Refresh

Status: ready-for-agent
Type: build
Blocked by: [018: Waiting-build notification](018-notify-waiting-build.md)
Parent: [013: Build update umbrella](013-surface-build-updates.md)
Related: [017: Specification and inspected sources](017-build-update-spec.md), [ADR-0022](../../../adr/0022-static-local-first-rust-application-with-worker-execution.md)

## What to build

Complete the chosen notification with a keyboard-accessible Refresh action that
actually adopts the waiting build while preserving work and other open editors'
assets. This is one safety-bounded vertical action: exposing reload separately
from its guards would violate 013. It depends on 018's real waiting state,
notification and A/B fixture; it does not depend on unrelated recovery tickets.

## Acceptance criteria

- [ ] The action is labelled **"Update Now"** and is clickable text in the Toast and
  in the persistent header notice that 018 shows after dismissal (user, 2026-11-06).
  Add it through the existing shared control/notification seam.
  Refresh uses real waiting state and rechecks safety at activation. Avoid
  duplicate adoption from repeated activation or stale notifications.
- [ ] Inventory the relevant Draft value, Pending edit and local-save owners.
  Warn before reload and offer cancellation when work is at risk. Cancellation
  preserves the editor and values; pending or failed save never counts as safe.
  A warning acknowledgment alone is not permission to discard work. The user
  decided on 2026-11-06 that unsaved drafts that cannot yet be saved **defer the
  update** until the user resolves them; explain that and record any gap in detecting
  them.
- [ ] Demonstrate safe refresh after work settles and saves, with the same saved
  project reopened. Do not equate the header save label with complete draft safety.
- [ ] Preserve ordinary delayed activation and other open editors' offline assets.
  Document the adoption/cache mechanism before applying it; the current activation
  cleanup removes old scoped caches, so forced activation with unchanged cleanup
  is insufficient. Add a dated ADR-0022 amendment for any policy refinement.
- [ ] When another editor prevents safe adoption, preserve it and tell the user to
  close other open tabs (user, 2026-11-06: wording like "Please close any other open
  tabs"). Never silently force, close or reload other tabs. Any further interaction
  that needs a new product decision is recorded as an exact blocker.
- [ ] Mounted production tests drive Refresh, warning/cancel and pending/failure
  states. Shared Toast tests cover action reachability, Tab/Enter/Space, focus
  after cancellation, normal/reduced motion and Theme/Appearance/narrow layouts.
  Preserve existing transient notifications and persistent inline reports.
- [ ] Extend 018's real A-then-B scenario: A remains controlled and open while B
  installs and waits; the notification appears; cancel with a draft and pending
  save; settle/save, activate Refresh and prove the loaded application is B and
  the saved project is intact. A worker label or server manifest alone is not proof.
- [ ] Repeat with a second A editor kept open and an offline request for a cached
  A asset it had not yet used. Prove continuity through adoption or explicit
  deferral, then complete B adoption once the obstacle is resolved. Verify
  root/subpath scope isolation and no reload on failed installation.
- [ ] Capture RED on the missing adoption behavior and GREEN on the same real
  scenario after implementation, plus mounted safety controls. Do not clear caches,
  unregister workers or replace real lifecycle with mocks to obtain GREEN.
- [ ] Record whether optional running version/time display was included or omitted
  (user: drop it if it needs plumbing). Any shown identity belongs to the loaded page.
  Reconcile 013's acceptance list with evidence without closing 010/011 or claiming
  historical stale-build causation.

## Verification and completion

Follow 017's full testing protocol and cited prior art (S3–S9). Run focused
mounted/shared-control tests, the dedicated packaged A/B browser scenarios and
python3 scripts/check.py quick for touched crates. Record commands, counts,
source revisions, both build identities and .evidence/ log paths. Keep builds
serialized per target and leave the live preview alone. A blocked real scenario
is an explicit incomplete criterion, not a mounted-test substitute.

## Comments

- 2026-11-06: Published through to-tickets at the user's explicit request.
  The two slices are intentionally notification first, then the complete guarded
  action; safety is not deferred to a later hardening ticket. No implementation,
  RED/GREEN execution or acceptance completion has occurred in this planning task.
