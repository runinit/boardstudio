# 013: Make a stale or updated build visible to the user

Status: ready-for-agent
Type: umbrella
Blocked by: none for coordination; completion requires implementation children
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [002: Diagnose MCU rotation](002-diagnose-mcu-rotation.md), [ADR-0022](../../../adr/0022-static-local-first-rust-application-with-worker-execution.md)

## Delivery structure

This remains the decision-owning umbrella, not a second implementation claim.
[017: Build update specification](017-build-update-spec.md) expands the recorded
choices into source-grounded behavior and test requirements. Child completion
does not close this umbrella until every acceptance criterion below has evidence.
The Decisions and original Comments remain authoritative and unchanged.

- [018: Notify the editor when a build is waiting](018-notify-waiting-build.md):
  unblocked detection-to-notification slice; does not alone complete Refresh.

## What to build

Let a user tell which build they are running and learn that a newer one is
waiting, so a stale cached version cannot pass for the current one. In 002 the
user reported an MCU orientation problem that came from a cached, wrong version of
the app; nothing in the interface showed that. The user asked on 2026-11-06 for
this to be added as a feature.

Facts at the time of writing, to re-verify during implementation:

- The app ships an offline service worker with a versioned release manifest and a
  cache named from that version (`web/src/service_worker.rs`,
  `web/crates/host/src/offline.rs`).
- ADR-0022 deliberately prefers delayed worker activation over forcing an update,
  so open editors keep the assets they need. That trades immediate adoption for
  continuity, and as written leaves no signal that an update is waiting.
- It is not established that this was the cause of the user's stale version; 002
  never captured it.

## Decisions

Answered by the user on 2026-11-06: show an **"Update Available" notification with
a refresh button**. Not answered, so these are the working assumptions; the user
can overrule any of them:

- The notification appears when a newer worker is installed and waiting. It does
  not interrupt editing and uses the existing Toast pattern, persisting until
  dismissed or adopted.
- The refresh button reloads the app to adopt the update. If a save is pending or
  an edit is unsaved, the user is told before the reload and can cancel, so
  nothing is lost (this is what keeps ADR-0022's continuity decision intact).
- Showing the running build's identity (version and time) is a small addition,
  not requested; include it only where it is nearly free, for example in the
  notification text, and say so in the ticket.

## Acceptance criteria

- [ ] The notification and its refresh button behave as in Decisions; any
  working assumption the implementation changes is recorded here.
- [ ] When a newer worker is installed and waiting, the interface says so without
  interrupting editing, and offers a way to adopt it. Adopting never discards
  pending edits or breaks an open editor's assets (ADR-0022).
- [ ] RED/GREEN evidence at the real boundary: a mounted or packaged-browser
  scenario that serves an old build, then a new one, and asserts the waiting-update
  notification, the refresh action and that the new build is then running. Source presence or a changed label is not
  evidence.
- [ ] Reduced-motion and keyboard access for the new control, using the shared
  controls and the existing Toast or status patterns where they fit.
- [ ] If the ADR-0022 trade-off is changed or refined, add a dated amendment
  instead of rewriting it.

## Verification and completion

The ticket's own scenario plus `python3 scripts/check.py quick` for touched
crates; a full gate belongs to [010](010-final-technical-verification.md) only if
the user adds this to that scope. It is not a blocker of 010 unless the user says
so.

## Comments

- 2026-11-06: Added at the user's request ("we should add this feature") after
  002 closed on a stale cached build.
- 2026-11-06: The user chose an "Update Available" notification with a refresh
  button. Status moved from `ready-for-human` to `ready-for-agent`; the pending-edits
  behavior and build identity are working assumptions, listed under Decisions.

- 2026-11-06: Documentation planning made this a coordination umbrella with
  specification 017. No implementation or acceptance completion is claimed.
