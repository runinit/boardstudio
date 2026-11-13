# UI review status scratchpad

Capture observations from interactive review here before creating or updating tickets.
This is not a numbered ticket, acceptance evidence, or a change to the plan's statuses.
No findings have been recorded yet. Check [existing tickets](map.md) before filing a
new one; link an existing ticket rather than duplicating its scope.

## Preview/source identity — 2026-11-06

- Worktree: `/home/chris/.t3/worktrees/ts-boardstudio2/fix-ui-integration-recovery`
- Branch: `fix/ui-integration-recovery`
- Inspected HEAD: `366d52100d9dbf6686a9cd9dfdf02f0262a13eed`
  (`docs: specify truthful quiet save feedback`).
- Dirty source at inspection: application session code and pending-work tests;
  browser-harness drag smoke; header CSS; host offline code; runtime code and
  new update-safety module; shared Toast code/tests; page presentation,
  header, notifications and service worker. Dirty tracking documents: rust-ui-build
  026, UI recovery 022 and map. These pre-existing changes are preserved, not owned
  by this review setup. This scratchpad is an additional untracked document.
- Build/readiness: **blocked; no new build or server started by this setup**.
  At 19:48:33 -05:00 another session was running `wasm-pack test --headless
  --chrome web --locked --no-default-features --features page --
  mounted_installation_finishing` against this worktree's target (wasm-pack PID
  1438820; cargo PID 1438925; owning `pi` PID 1378049). The test exited during
  inspection, but its owning implementation session remained active in this
  worktree. Coordinate a build window with that owner before compiling the
  changing 019 source. No competing build was launched.
- Existing artifact, **not certified current**: `web/target/site`, index timestamp
  18:27 on 2026-11-06; service-worker identity
  `boardstudio-61c0c2785a1f11b2-1794007642106962249`. It predates inspected
  source changes and was not served as the latest build.
- Pending preview fields: URL/port; build start/end; exact source revision and
  dirty paths at build; build command/result/log; packaged offline identity;
  server PID/session owner; HTTP/isolation-header and mounted-page readiness.
- Supported next commands after exclusive build access is confirmed:
  `python3 scripts/build-web.py --fast --output web/target/site-ui-status`, then
  `python3 scripts/serve-web.py web/target/site-ui-status 4174` (recheck port is free).
  Keep the server persistent, leave existing previews alone, and have the parent
  open the preview for human review. A successful build is not a full test gate.

## Findings

None recorded. Copy the blank template below for each observed issue.

### Finding: <short title>

- Existing ticket / related scope:
- Surface / workspace:
- Theme:
- Appearance (light/dark/system):
- Viewport (width × height; browser; zoom/device scale):
- Preview URL / loaded build identity / observed time:
- Repro steps and starting project/state:
  1.
- Expected:
- Actual:
- Severity and user impact:
- Screenshot / evidence path:
- Reproducibility / follow-up questions:
