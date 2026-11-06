# 018: Notify the editor when a build is waiting

Status: claimed
Type: build
Blocked by: none
Parent: [013: Build update umbrella](013-surface-build-updates.md)
Related: [017: Specification and inspected sources](017-build-update-spec.md), [019: Protected refresh](019-adopt-build-safely.md)

## What to build

A designer keeping an Editor session open sees one persistent, dismissible
Update Available Toast when a real newer installation is waiting. This is the
first vertical slice through registration observation, application state and
mounted shell presentation. It can be demonstrated without adopting the update.
Refresh is delivered by 019; do not ship a misleading enabled no-op action here.
This slice alone does not satisfy the umbrella's chosen complete interaction.

## Acceptance criteria

- [ ] Handle a worker already waiting at observation startup and one that finishes
  installation during editing. Initial installation, failed installation and no
  update do not produce a false waiting-update notice.
- [ ] Drive the production shell notification without interrupting editing or
  stealing focus. It stays beyond normal Toast expiry; dismissal does not reload.
  Workspace changes and repeated observations do not duplicate the same notice.
- [ ] Reuse the shared Toast pattern with keyboard dismissal, atomic announcement,
  semantic Theme roles and reduced-motion handling. Preserve transient result
  Toasts and inline persistent reports; document the narrow update exception.
- [ ] Mounted production notification/shared-control tests prove those behaviors,
  including observer cleanup. Follow specification S5–S7 rather than inspecting
  source strings or substituting a standalone lookalike fixture.
- [ ] Record RED/GREEN using real packaged A then B releases at the same test
  origin while A remains open: actual waiting installation must produce the
  production notice. Keep this fixture usable by 019 without clearing caches.
  Record loaded A and waiting B identities independently.
- [ ] Running version/time display (user: drop it if it needs plumbing): include
  only if actual loaded-build metadata makes it nearly free; otherwise record its
  omission. Do not call the waiting/server identity the running identity.
- [ ] Dismissal (user, 2026-11-06): dismissing the Toast does not reload. Derive the
  notice from the real waiting worker and store nothing about dismissal; the Toast
  shows again on the next page load (confirmed by the user). The persistent **header**
  notice that follows dismissal needs a working "Update Now", so it is delivered with
  the action in [019](019-adopt-build-safely.md), not here. This slice ships no
  action and no header notice, so nothing misleading is enabled; say plainly that the
  user-chosen interaction is incomplete until 019.

## Verification and completion

Use the specification's mounted and real-browser RED/GREEN protocol, including
initial-install and failed-install controls, plus
python3 scripts/check.py quick for touched crates. Record exact commands,
test counts, source revisions, identities and log paths in completion comments.
Use a dedicated test origin/profile, never the live preview. No builds or
browser checks have been run as part of publishing this ticket.

## Comments

- 2026-11-06: Published through to-tickets at the user's explicit request.
  No prefactoring-only ticket is needed: observation and visible behavior travel
  together. 017 is the specification, not an unresolved implementation blocker.
