# 013: Make a stale or updated build visible to the user

Status: ready-for-human
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [002: Diagnose MCU rotation](002-diagnose-mcu-rotation.md), [ADR-0022](../../../adr/0022-static-local-first-rust-application-with-worker-execution.md)

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

## Decisions needed from the user before building

- What should the user see: the running build's identity (version and time), an
  "update available" prompt, or both, and where (Project menu, Settings, the
  status area)?
- Should adopting an update be a reload the user chooses, with a warning when
  unsaved or pending edits exist? This keeps ADR-0022's continuity decision and
  needs no change to it, unless the user decides otherwise.

## Acceptance criteria

- [ ] The user's answers to the decisions above are recorded here.
- [ ] A user can read the running build's identity from the interface, and it
  matches the release manifest version served.
- [ ] When a newer worker is installed and waiting, the interface says so without
  interrupting editing, and offers a way to adopt it. Adopting never discards
  pending edits or breaks an open editor's assets (ADR-0022).
- [ ] RED/GREEN evidence at the real boundary: a mounted or packaged-browser
  scenario that serves an old build, then a new one, and asserts the waiting-update
  signal and the adopted identity. Source presence or a changed label is not
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
