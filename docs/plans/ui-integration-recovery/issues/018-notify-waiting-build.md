# 018: Notify the editor when a build is waiting

Status: resolved
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

- [x] Handle a worker already waiting at observation startup and one that finishes
  installation during editing. Initial installation, failed installation and no
  update do not produce a false waiting-update notice.
- [x] Drive the production shell notification without interrupting editing or
  stealing focus. It stays beyond normal Toast expiry; dismissal does not reload.
  Workspace changes and repeated observations do not duplicate the same notice.
- [x] Reuse the shared Toast pattern with keyboard dismissal, atomic announcement,
  semantic Theme roles and reduced-motion handling. Preserve transient result
  Toasts and inline persistent reports; document the narrow update exception.
- [x] Mounted production notification/shared-control tests prove those behaviors,
  including observer cleanup. Follow specification S5–S7 rather than inspecting
  source strings or substituting a standalone lookalike fixture.
- [x] Record RED/GREEN using real packaged A then B releases at the same test
  origin while A remains open: actual waiting installation must produce the
  production notice. Keep this fixture usable by 019 without clearing caches.
  Record loaded A and waiting B identities independently.
- [x] Running version/time display (user: drop it if it needs plumbing): include
  only if actual loaded-build metadata makes it nearly free; otherwise record its
  omission. Do not call the waiting/server identity the running identity.
- [x] Dismissal (user, 2026-11-06): dismissing the Toast does not reload. Derive the
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

- 2026-11-06: Independent review found the earlier pre-mode driver expected
  absence and therefore returned a passing negative control rather than RED.
  The driver now applies the same positive waiting-notice assertion in both
  modes and compares the pre-run load marker with its original value. The
  pre-feature run exits 1 with exactly that assertion failing; the post-feature
  run exits 0 with it passing and no other failures. Logs:
  `.evidence/ui-integration-recovery/018-recheck-pre.log` and
  `.evidence/ui-integration-recovery/018-recheck-post.log`. The earlier 21-pass
  pre run below is historical evidence of absence, not the RED assertion.
- 2026-11-06: `python3 scripts/check.py test` passed on the isolated 018 source:
  all 2,384 baselined native tests executed, with 28 new tests. Log:
  `.evidence/ui-integration-recovery/018-full-native-tests.log`. Independent
  spec and standards reviews identified the corrected issues recorded above;
  the final driver correction was inspected and its two affected browser runs
  were rerun. Ticket 019 owns the update action and persistent header notice.
- 2026-11-06: Real packaged A-then-B evidence, first run (dedicated origin http://127.0.0.1:4723/, root scope `/`, isolated browser session `018-ab-270824a22972`; no live preview; no cache clears; no unregister).
  - Source: WIP commit 2b7d1927 plus working-tree clippy-alias fixes in `web/crates/host/src/host/offline.rs` and `web/src/presentation/notifications.rs`.
  - Packaged with `python3 scripts/build-web.py --fast --routes root --output web/target/site-018-a` (full) and `--skip-providers ... --output web/target/site-018-b`. Loaded A identity `boardstudio-c947e6770e10178a-1793994010714373437`; waiting B identity `boardstudio-c947e6770e10178a-1793994577136586039` (same source, distinct release versions).
  - Negative controls (not pre-change RED): fresh first load installs once, page controlled via claim, `waiting` null, editor opened with a saved demo copy, no `Update Available` notice. Failed-install control: same-version worker with an injected top-level throw makes `registration.update()` reject, leaves `installing`/`waiting` null and the editor/controller intact, and shows no notice.
  - GREEN: same origin swapped to B (`touch` on B's `service-worker.js` first: the poisoned broken-script HTTP-cache entry otherwise revalidates `304` and the update keeps evaluating stale bytes), `registration.update()` from the open A page installs B to `waiting` with the controller still A and the editor alive; the production Toast appears (`role=status`, `aria-atomic=true`, `data-type=info`, close `tabIndex=0`, region announces `Update Available`). Dismissal via close removes it with `waiting` still present, controller unchanged and editor alive: no reload. Screenshot `.evidence/018-green-notice.png`; logs `.evidence/018-build-{a,b}.log`, `.evidence/018-serve-{a,b}.log`. Build dirs retained for 019; live browser/server closed.
  - Limits: A/B are `--fast` unoptimized packaged builds from identical source (identities differ by release version only).
- 2026-11-06: Abridged second run (binaries since superseded; kept as history, not evidence): loaded A `boardstudio-c947e6770e10178a-1793995701582141371`; waiting B `boardstudio-c947e6770e10178a-1793995734423473772`, session `018-post-270824a22972`. Superseded by the driver runs below. Logs `.evidence/018-build-{a2,b2}.log`, `.evidence/018-serve-{a2,b2}.log`; screenshot `.evidence/018-green-notice-run2.png`.
- 2026-11-06: Pre-change RED, tracked driver (`python3 scripts/verify-waiting-update.py pre`, isolated session `018-verify-pre-270824a22972`; ad-hoc /tmp drivers superseded). Packaged pre-feature app from `7688074ccf5597fd617d35ce78cfa0d28ceeb619` (`git archive` to `/tmp/pre018-7688074`, page bundle rebuilt with `--fast --skip-providers`; 018-untouched provider artifacts symlinked from the worktree; built bundle verified to contain no `Update Available` string). Loaded pre-A identity `boardstudio-f8d90b4c254618b9-1793996277155741643` (distinct digest: pre-feature source). Pre-feature A page controlled with a saved demo copy open, then served waiting B (`boardstudio-c947e6770e10178a-1793999157862258339`, current-tree build) at the same scope and ran `registration.update()` from the open page: B installed to `waiting` with the pre-feature controller intact and the editor alive, and NO `Update Available` notice appeared (no `role=status` notice at all). The driver asserts every observation and fails otherwise: 21 PASS, 0 FAIL. Every driver command is echoed verbatim and every browser output retained raw in `.evidence/018-verify-pre.log` (payloads `scripts/verify-waiting-update-payloads/*.js`).
- 2026-11-06: Post-change GREEN, tracked driver from the exact current tree (`python3 scripts/verify-waiting-update.py post`, isolated session `018-verify-post-270824a22972`; A `boardstudio-c947e6770e10178a-1793999131633832380`, B `boardstudio-c947e6770e10178a-1793999157862258339`; ad-hoc runs superseded). RED, initial-install and failed-install controls re-pass; GREEN re-confirmed with the production Toast attributes recorded (`role=status`, `aria-atomic=true`, `data-type=info`, close `tabIndex=0`, region announces the update); production workspace switch (real Workspace navigation, Layout→PCB) with the notice up leaves exactly one notice with PCB active; real keyboard dismissal (focus + Enter) removes it with B still waiting; reload reappears the notice with B still waiting (no adoption, nothing stored). The driver asserts every observation and fails otherwise: 40 PASS, 0 FAIL. Every command echoed verbatim with raw outputs in `.evidence/018-verify-post.log` (payloads `scripts/verify-waiting-update-payloads/*.js`; the driver owns its server PIDs and verifies the dedicated port is free before/after; sessions closed, no cache clears, no unregisters).
- 2026-11-06: Reduced-motion run from the exact current tree (`python3 scripts/verify-waiting-update.py rm`, isolated session `018-verify-rm-270824a22972`, same A/B identities as the GREEN run above; ad-hoc runs superseded). `prefers-reduced-motion` emulated and verified in-page; with B waiting, the rendered notice and its close report computed `animation-name: none` and `0s` durations (no motion to depend on) and the control stays fully operable: focus plus real Enter dismisses with B still waiting, controller still A and editor alive. The driver asserts every observation: 32 PASS, 0 FAIL. Every command echoed verbatim with raw outputs in `.evidence/018-verify-rm.log`.
- 2026-11-06: How the loaded and waiting identities are bound (all three driver runs above): the served release version is recorded by curl before load and after each swap; an in-page random load marker is set at load and re-read on every observation (unchanged ⇒ no reload/navigation, so the loaded page runs the release served at load, and the waiting worker cannot be the loaded one); `caches.keys()` plus a read of each versioned cache's stored `service-worker.js` bootstrap text independently confirm both installed release versions from production bytes. Residual limitation, stated plainly: no web API exposes a service worker object's own version, and a page fetch of the worker URL returns the currently served file rather than the installed worker's bytes, so the active↔A / waiting↔B attribution chains install chronology (a fresh profile admits exactly one install per phase) with production bytes; it is not a single direct worker-version read. Nothing above is inferred beyond what the retained logs show.
- 2026-11-06: Running version/time display is omitted: showing the loaded build's identity needs plumbing that is not nearly free, so the notice carries no identity at all rather than risk calling the waiting/server identity the running one.

- 2026-11-06: Published through to-tickets at the user's explicit request.
  No prefactoring-only ticket is needed: observation and visible behavior travel
  together. 017 is the specification, not an unresolved implementation blocker.
